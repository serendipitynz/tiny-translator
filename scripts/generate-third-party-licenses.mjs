#!/usr/bin/env node
// Generate THIRD-PARTY-LICENSES.txt, the notice every bundle carries (tauri.conf.json's
// bundle.resources). Adapted from backlog-atlas's generator of the same name.
//
// Run it after either lockfile, THIRD-PARTY-NOTICES.md or scripts/spdx/ changes:
//
//   pnpm install
//   cargo fetch --manifest-path src-tauri/Cargo.toml \
//     --target aarch64-apple-darwin --target x86_64-pc-windows-msvc
//   node scripts/generate-third-party-licenses.mjs
//
// and commit the result. src/lib/third-party-licenses.test.ts fails when the digests the file
// records no longer match its inputs.
//
// The output is committed rather than produced in CI because bundle.resources names it: a resource
// that exists only on a runner would break `pnpm tauri build` for everyone else. Committing it is
// safe because the output is byte-reproducible — nothing below reads a clock, a path or a
// machine-specific value, and everything is emitted in sorted order.
//
// Crate texts come from the `.crate` tarballs in the cargo registry cache, not from the extracted
// sources: `cargo fetch --target` fills the cache for a platform this machine never builds, and
// the notice has to cover both bundles at once.

import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { gunzipSync } from 'node:zlib'

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const SPDX_DIR = join(REPO, 'scripts', 'spdx')
const VENDORED_NOTICES = join(REPO, 'THIRD-PARTY-NOTICES.md')
const OUTPUT = join(REPO, 'THIRD-PARTY-LICENSES.txt')

// The triples the release workflow builds. Linux and Intel macOS are not targets (doc-1 §4.2).
const TARGETS = ['aarch64-apple-darwin', 'x86_64-pc-windows-msvc']

const LICENSE_FILE = /^(licen[cs]e|copying|copyright|notice|unlicen[cs]e)/i
// A source file whose name starts with "license" is code, not a notice.
const NOT_A_NOTICE = /\.(rs|js|mjs|cjs|ts|json|toml|py|sh)$/i

function fail(message) {
  console.error(`generate-third-party-licenses: ${message}`)
  process.exit(1)
}

// Line endings are the one thing normalized, so the output does not depend on which packages
// happen to publish CRLF.
function normalize(text) {
  return `${text.replace(/^﻿/, '').replace(/\r\n/g, '\n').replace(/\s+$/, '')}\n`
}

function sha256(buffer) {
  return createHash('sha256').update(buffer).digest('hex')
}

function run(command, args, options = {}) {
  try {
    return execFileSync(command, args, {
      cwd: REPO,
      encoding: 'utf8',
      maxBuffer: 256 * 1024 * 1024,
      ...options,
    })
  } catch (error) {
    fail(`\`${command} ${args.join(' ')}\` failed: ${error.message}`)
  }
}

// --- npm side -------------------------------------------------------------

// The packages whose code is in the built frontend, read from the sourcemaps of a throwaway build.
// Not `pnpm licenses list --prod`: svelte is a devDependency, yet its runtime (and the packages
// that runtime imports) is compiled into the bundle, and --prod would leave all of it out.
function collectNpm() {
  const outDir = mkdtempSync(join(tmpdir(), 'tiny-translator-licenses-'))
  try {
    run('pnpm', ['exec', 'vite', 'build', '--sourcemap', '--outDir', outDir, '--emptyOutDir', '--logLevel', 'error'])
    const assets = join(outDir, 'assets')
    const packageDirs = new Set()
    for (const name of readdirSync(assets).filter((file) => file.endsWith('.js.map'))) {
      const map = JSON.parse(readFileSync(join(assets, name), 'utf8'))
      for (const source of map.sources) {
        // Windows resolves with backslashes; fs accepts the forward-slash form there as well.
        const match = resolve(assets, source)
          .replaceAll('\\', '/')
          .match(/^(.*\/node_modules\/(?:@[^/]+\/)?[^/]+)\//)
        if (match) {
          packageDirs.add(match[1])
        }
      }
    }
    if (packageDirs.size === 0) {
      fail('the built frontend maps to no npm package. Run `pnpm install` first.')
    }
    return [...packageDirs].map((dir) => {
      const manifest = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'))
      return {
        ecosystem: 'npm',
        name: manifest.name,
        version: manifest.version,
        declared: typeof manifest.license === 'string' ? manifest.license : null,
        texts: readNpmTexts(dir),
      }
    })
  } finally {
    rmSync(outDir, { recursive: true, force: true })
  }
}

function readNpmTexts(path) {
  return readdirSync(path)
    .filter((name) => LICENSE_FILE.test(name) && !NOT_A_NOTICE.test(name))
    .filter((name) => statSync(join(path, name)).isFile())
    .sort()
    .map((name) => ({ name, text: normalize(readFileSync(join(path, name), 'utf8')) }))
}

// --- cargo side -----------------------------------------------------------

// One --filter-platform pass per shipped triple, unioned. The unfiltered graph would claim that
// crates for Linux, Android or wasm ship in these bundles.
function collectCargo() {
  const caches = registryCacheDirs()
  const shipped = new Map()
  for (const target of TARGETS) {
    const metadata = JSON.parse(
      run('cargo', [
        'metadata',
        '--format-version',
        '1',
        '--locked',
        '--filter-platform',
        target,
        '--manifest-path',
        join(REPO, 'src-tauri', 'Cargo.toml'),
      ]),
    )
    const byId = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]))
    for (const id of reachableFromRoot(metadata)) {
      const pkg = byId.get(id)
      // A package without a source is a path dependency: the app itself.
      if (!pkg.source || shipped.has(id)) {
        continue
      }
      shipped.set(id, {
        ecosystem: 'cargo',
        name: pkg.name,
        version: pkg.version,
        declared: pkg.license ?? null,
        texts: readCrateTexts(pkg, caches),
      })
    }
  }
  if (shipped.size === 0) {
    fail('cargo metadata resolved no dependencies.')
  }
  return [...shipped.values()]
}

// Every edge that is not a dev-dependency. Build-dependencies are kept: listing a build script's
// licence costs a paragraph, and which of its code the linker kept is not in the lockfile.
function reachableFromRoot(metadata) {
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]))
  const root = metadata.resolve.root
  if (!root) {
    fail('cargo metadata reported no resolve root.')
  }
  const seen = new Set([root])
  const queue = [root]
  while (queue.length > 0) {
    const node = nodes.get(queue.shift())
    for (const dep of node?.deps ?? []) {
      if (!dep.dep_kinds.some((kind) => kind.kind !== 'dev')) {
        continue
      }
      if (!seen.has(dep.pkg)) {
        seen.add(dep.pkg)
        queue.push(dep.pkg)
      }
    }
  }
  return [...seen]
}

function registryCacheDirs() {
  const home = process.env.CARGO_HOME || join(homedir(), '.cargo')
  const cache = join(home, 'registry', 'cache')
  let names
  try {
    names = readdirSync(cache)
  } catch {
    fail(`no cargo registry cache at ${cache}. Run \`cargo fetch\` for the release targets first.`)
  }
  return names.map((name) => join(cache, name))
}

function readCrateTexts(pkg, caches) {
  const file = `${pkg.name}-${pkg.version}.crate`
  for (const cache of caches) {
    let archive
    try {
      archive = readFileSync(join(cache, file))
    } catch {
      continue
    }
    return listTar(gunzipSync(archive), join(cache, file))
      .filter(({ name }) => {
        // Depth 1 only: deeper hits are a vendored tree or a test fixture.
        const parts = name.split('/')
        return parts.length === 2 && LICENSE_FILE.test(parts[1]) && !NOT_A_NOTICE.test(parts[1])
      })
      .map(({ name, body }) => ({ name: name.split('/')[1], text: normalize(body.toString('utf8')) }))
      .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))
  }
  fail(
    `${file} is not in the cargo registry cache. Run:\n` +
      `  cargo fetch --manifest-path src-tauri/Cargo.toml ${TARGETS.map((t) => `--target ${t}`).join(' ')}`,
  )
}

// A tar reader for crates.io tarballs. Long-name extensions do occur, and an unread extension
// header would put the next entry's name on the wrong bytes and lose a notice silently, so an
// extension this does not implement is an error rather than a skip.
function listTar(buffer, path) {
  const entries = []
  let pendingName = null
  for (let offset = 0; offset + 512 <= buffer.length; ) {
    const header = buffer.subarray(offset, offset + 512)
    if (header.every((byte) => byte === 0)) {
      break
    }
    const field = (start, length) => {
      const raw = header.subarray(start, start + length)
      const end = raw.indexOf(0)
      return raw.subarray(0, end === -1 ? raw.length : end).toString('utf8')
    }
    const size = Number.parseInt(field(124, 12).trim() || '0', 8)
    const typeflag = field(156, 1) || '0'
    const body = buffer.subarray(offset + 512, offset + 512 + size)
    offset += 512 + Math.ceil(size / 512) * 512

    if (typeflag === 'L') {
      pendingName = body.toString('utf8').replace(/\0+$/, '')
      continue
    }
    if (typeflag === 'K') {
      continue
    }
    if (typeflag === 'x' || typeflag === 'g') {
      if (typeflag === 'x') {
        const record = body.toString('utf8').match(/^\d+ path=(.*)$/m)
        pendingName = record ? record[1] : pendingName
      }
      continue
    }
    if (typeflag !== '0' && typeflag !== '\0') {
      if (typeflag !== '5' && typeflag !== '1' && typeflag !== '2') {
        fail(`${path} holds a tar entry of type ${typeflag} this reader does not implement.`)
      }
      pendingName = null
      continue
    }

    const prefix = field(345, 155)
    const name = pendingName ?? (prefix ? `${prefix}/${field(0, 100)}` : field(0, 100))
    pendingName = null
    entries.push({ name, body })
  }
  return entries
}

// --- SPDX fallback --------------------------------------------------------

// Some crates publish no notice and declare only an SPDX expression. The standard text of each
// identifier stands in, kept in scripts/spdx/ so a new identifier is a visible addition.
function spdxIdentifiers(expression) {
  return [
    ...new Set(
      expression
        .replace(/[()]/g, ' ')
        .split(/\s+(?:OR|AND|WITH)\s+|\//i)
        .map((part) => part.trim())
        .filter((part) => part.length > 0 && !/^(OR|AND|WITH)$/i.test(part)),
    ),
  ]
}

function spdxText(identifier) {
  try {
    return normalize(readFileSync(join(SPDX_DIR, `${identifier}.txt`), 'utf8'))
  } catch {
    return null
  }
}

// Every missing identifier is reported at once, so one run describes a whole dependency bump.
function applyFallback(packages) {
  const missing = new Map()
  const undeclared = []
  for (const pkg of packages) {
    if (pkg.texts.length > 0) {
      continue
    }
    if (!pkg.declared) {
      undeclared.push(`${pkg.ecosystem} ${pkg.name} ${pkg.version}`)
      continue
    }
    pkg.texts = spdxIdentifiers(pkg.declared).map((identifier) => {
      const text = spdxText(identifier)
      if (text === null) {
        missing.set(identifier, [...(missing.get(identifier) ?? []), `${pkg.ecosystem} ${pkg.name} ${pkg.version}`])
      }
      return { name: `${identifier} (standard text; this package publishes none)`, text: text ?? '' }
    })
  }
  if (undeclared.length > 0) {
    fail(
      'these packages publish no licence file and declare no licence; establish their terms by hand ' +
        `before shipping them:\n${undeclared.map((entry) => `  ${entry}`).join('\n')}`,
    )
  }
  if (missing.size > 0) {
    fail(
      `no standard text for ${missing.size} SPDX identifier(s). Add each one's official text under ` +
        `scripts/spdx/ and re-run:\n${[...missing]
          .map(([identifier, carriers]) => `  scripts/spdx/${identifier}.txt  (for ${carriers.join(', ')})`)
          .join('\n')}`,
    )
  }
}

// --- output ---------------------------------------------------------------

const order = (a, b) => {
  if (a.ecosystem !== b.ecosystem) {
    return a.ecosystem < b.ecosystem ? -1 : 1
  }
  if (a.name !== b.name) {
    return a.name < b.name ? -1 : 1
  }
  return a.version < b.version ? -1 : a.version > b.version ? 1 : 0
}

function rule(char = '=') {
  return char.repeat(78)
}

// The generator is an input too: a change to how the output is shaped stales it as surely as a
// change to what goes into it. The frontend's sources are not listed even though the npm side is
// read from a build of them: which packages reach the bundle moves with the lockfile, and listing
// every source file would make each UI change regenerate the notice.
function inputs() {
  const paths = [
    'scripts/generate-third-party-licenses.mjs',
    'pnpm-lock.yaml',
    'src-tauri/Cargo.lock',
    'THIRD-PARTY-NOTICES.md',
    ...readdirSync(SPDX_DIR)
      .filter((name) => name.endsWith('.txt'))
      .sort()
      .map((name) => `scripts/spdx/${name}`),
  ]
  return paths.map((label) => ({ label, digest: sha256(readFileSync(join(REPO, label))) }))
}

function inventory(packages, ecosystem) {
  const rows = packages.filter((pkg) => pkg.ecosystem === ecosystem)
  const width = Math.max(...rows.map((pkg) => `${pkg.name} ${pkg.version}`.length))
  return rows.map((pkg) => `  ${`${pkg.name} ${pkg.version}`.padEnd(width)}  ${pkg.declared ?? '(not declared)'}`)
}

// Each distinct text once, followed by the packages that carry it; the Apache-2.0 text alone is
// shipped by over a hundred crates.
function groupTexts(packages) {
  const groups = new Map()
  for (const pkg of packages) {
    for (const { name, text } of pkg.texts) {
      const key = sha256(Buffer.from(text, 'utf8'))
      if (!groups.has(key)) {
        groups.set(key, { text, carriers: [] })
      }
      groups.get(key).carriers.push(`${pkg.ecosystem} ${pkg.name} ${pkg.version} (${name})`)
    }
  }
  return [...groups.values()]
}

function render(packages) {
  const npm = packages.filter((pkg) => pkg.ecosystem === 'npm').length
  const cargo = packages.filter((pkg) => pkg.ecosystem === 'cargo').length
  const groups = groupTexts(packages)

  const lines = [
    rule(),
    'THIRD-PARTY LICENSES — tiny-translator',
    rule(),
    '',
    'This file carries the notices owed to the authors of the software that ships',
    'inside a tiny-translator bundle.',
    '',
    'It is generated by scripts/generate-third-party-licenses.mjs and must not be',
    'edited by hand.',
    '',
    'Section 1 is THIRD-PARTY-NOTICES.md reproduced in full. It covers what no',
    'lockfile mentions: the translation engine (llama.cpp and the libraries built',
    'into it) and the Lucide figures.',
    '',
    `Sections 2 and 3 list the ${npm} npm packages whose code is in the built frontend and`,
    `the ${cargo} cargo crates resolved for the macOS (aarch64) and Windows (x86_64)`,
    'builds. Dev-dependencies are excluded; build-dependencies are kept.',
    '',
    'Section 4 holds the licence texts, each printed once and followed by the',
    'packages that carry it. Where a package publishes no text of its own, the',
    'standard text of the licence it declares stands in, and the entry says so.',
    '',
    'Line endings are the only thing normalized. No text below is otherwise altered.',
    '',
    'Generated from these inputs (a digest that no longer matches means this file',
    'is stale; regenerate it with the commands at the top of the generator):',
    ...inputs().map(({ label, digest }) => `  ${label.padEnd(46)}  sha256 ${digest}`),
    '',
    rule(),
    'SECTION 1 — MATERIAL NOT RESOLVED FROM A LOCKFILE (THIRD-PARTY-NOTICES.md)',
    rule(),
    '',
    normalize(readFileSync(VENDORED_NOTICES, 'utf8')).trimEnd(),
    '',
    rule(),
    `SECTION 2 — NPM PACKAGES (${npm})`,
    rule(),
    '',
    ...inventory(packages, 'npm'),
    '',
    rule(),
    `SECTION 3 — CARGO CRATES (${cargo})`,
    rule(),
    '',
    ...inventory(packages, 'cargo'),
    '',
    rule(),
    `SECTION 4 — LICENCE TEXTS (${groups.length})`,
    rule(),
  ]

  groups.forEach((group, index) => {
    lines.push(
      '',
      rule('-'),
      `TEXT ${index + 1} of ${groups.length} — carried by ${group.carriers.length} package(s):`,
      ...group.carriers.map((carrier) => `  ${carrier}`),
      rule('-'),
      '',
      group.text.trimEnd(),
    )
  })

  lines.push('')
  return lines.join('\n')
}

const packages = [...collectNpm(), ...collectCargo()].sort(order)
applyFallback(packages)
const output = render(packages)
writeFileSync(OUTPUT, output)
console.log(
  `Wrote ${OUTPUT}: ${packages.filter((p) => p.ecosystem === 'npm').length} npm packages, ` +
    `${packages.filter((p) => p.ecosystem === 'cargo').length} cargo crates, ` +
    `${(Buffer.byteLength(output) / 1024).toFixed(0)} KB.`,
)
