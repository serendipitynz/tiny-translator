<img src="src-tauri/icons/128x128@2x.png" alt="" width="128">

# tiny-translator

> 日本語: [README.ja.md](README.ja.md)

A small desktop translator that runs PLaMo 3 Translate locally. The design is doc-1 in
`backlog/docs/`, and the units of work are in `backlog/tasks/` (managed with Backlog.md). Both are
written in Japanese, as is the app's UI; this README quotes UI labels in Japanese with an
English gloss.

## Prerequisites

- Node.js 24 (`.node-version`) and pnpm 10 (`packageManager` in `package.json`)
- The Rust toolchain Tauri 2 requires (on macOS, also the Xcode Command Line Tools)
- On Windows (x86_64): Rust for MSVC (`x86_64-pc-windows-msvc`), the "Desktop development with C++"
  workload of Visual Studio Build Tools, and the WebView2 runtime (usually present on Windows 10 and later)

## Translation engine and model

The translation engine (llama-server) is a Tauri sidecar and is not in the repository. Before the
first build, run the following at the repository root to build it into `src-tauri/binaries/` (the
script fetches llama.cpp into `~/.cache/tiny-translator/` and builds it statically). Without it,
`cargo` builds, tests and clippy fail too.

On macOS (Apple Silicon) this builds the Metal variant. CMake is required.

```sh
./scripts/build-llama-server.sh
```

On Windows (x86_64) this builds the Vulkan variant. In addition to the "Desktop development with C++"
workload of Visual Studio Build Tools listed above, it needs the Vulkan SDK
(`winget install KhronosGroup.VulkanSDK`). If CMake is not on PATH, the one bundled with Visual Studio
is used. The resulting exe loads only OS DLLs and the `vulkan-1.dll` that the GPU driver installs
(the script checks this with `dumpbin`).

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-llama-server.ps1
```

The model GGUF (by default `plamo-3-translate-2b-Q4_K_M.gguf`) is fetched by the app's 「モデルを入手」
(Get the model) from `pfnet/plamo-3-translate-*-GGUF` on Hugging Face, after agreeing to the PLaMo Community
License and with a SHA-256 check. On a machine without a network, you can place the GGUF in `models/`
under the app data directory by hand (`~/Library/Application Support/com.serendipitynz.tiny-translator/models/`
on macOS, `%APPDATA%\com.serendipitynz.tiny-translator\models\` on Windows; a symbolic link also works).
The translation engine logs to `llama-server.log` (in `~/Library/Logs/com.serendipitynz.tiny-translator/`
on macOS, `%LOCALAPPDATA%\com.serendipitynz.tiny-translator\logs\` on Windows).

## License notices

Built with PLaMo. The translation model, PLaMo 3 Translate, is provided by Preferred Networks under
the [PLaMo Community License](https://plamo.preferredai.jp/info/plamo-community-license-ja). From
「ライセンス」 (License) in the app's 「設定」 (Settings) you can open the full license text (Japanese and English) and a
guide to the conditions users must meet themselves (registering business use, attribution in long
published works, the notice when handing over translations, and the restriction on important
decisions).

## Development commands

Run these at the repository root.

```sh
pnpm install
pnpm tauri dev       # launch the app (also starts the Vite dev server)
pnpm check           # svelte-check (types and Svelte)
pnpm lint            # biome lint
pnpm test            # vitest
pnpm build           # build the UI into dist/
```

Run the Rust side in `src-tauri/`.

```sh
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --check
```

`scripts/` also holds measurement scripts used in the technical validation (m-0); they are
independent of the app build.

## Distribution

The app is distributed as a `.dmg` for macOS (Apple Silicon) and an NSIS installer (`*-setup.exe`)
for Windows (x86_64), built by the GitHub Actions workflow `.github/workflows/release.yml` (design in
doc-1 §13).

- The macOS `.app` and `.dmg` are signed with a Developer ID and notarized by Apple.
- The Windows installer is not code-signed. When downloaded in a browser, it is flagged as a file
  that is not commonly downloaded. Open the menu next to "Save" and choose "Keep", then
  "Keep anyway". An installer saved this way started without a warning on double-click when checked
  on 2026-11-11. If SmartScreen shows "Windows protected your PC" on launch, choose "More info" and
  then "Run anyway". It installs per user into `%LOCALAPPDATA%\tiny-translator\` and needs no
  administrator rights.
- Neither includes the model. Fetch it with 「モデルを入手」 (Get the model) on first use.
- At launch the app checks for a new version at most once a day and shows a notice if there is one.
  「更新して再起動」 (Update and restart) replaces the app. To stop the check, turn off
  「新しいバージョンを自動で確認する」 (check for new versions automatically) under 「更新」 (Updates) in
  「設定」 (Settings) (doc-1 §14).
- The licenses of the bundled software are in `THIRD-PARTY-LICENSES.txt`, which is also shipped
  (in the `.app`'s `Contents/Resources/` on macOS, in the install folder on Windows).

### Release steps

1. Bump `version` in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` to the
   same value, and run `cargo update -p tiny-translator` in `src-tauri/` to update `Cargo.lock` to match.
2. If dependencies changed, regenerate `THIRD-PARTY-LICENSES.txt` (`pnpm test` fails when it is stale).
   ```sh
   cargo fetch --manifest-path src-tauri/Cargo.toml --target aarch64-apple-darwin --target x86_64-pc-windows-msvc
   node scripts/generate-third-party-licenses.mjs
   ```
3. After merging into the default branch, tag that commit `vX.Y.Z` and push the tag. The workflow
   creates a draft release and uploads the bundles for both OSes after checking them. It then verifies
   the update artifacts' signatures against the public key in `tauri.conf.json` and uploads
   `latest.json`.
4. Check the draft's bundles and the generated release notes, then publish it by hand. Besides the
   `.dmg` and `*-setup.exe`, the draft holds the update artifacts (`*.app.tar.gz` and the `.sig` files
   for both OSes) and `latest.json`. Once published, installed apps find the new version at their next
   check (the app reads `releases/latest/download/latest.json`, so a draft is not delivered).

The workflow uses six repository secrets for macOS signing and notarization. Before the first use,
export the Developer ID Application certificate from Keychain Access as a `.p12`, prepare
`.env.signing` (copy `.env.signing.example` and fill it in), and register them with:

```sh
./scripts/setup-ci-signing-secrets.sh path/to/DeveloperID.p12
```

The update artifacts are signed with the updater signing key (minisign). The owner keeps the private
key locally and must not lose it: without it, later versions cannot be delivered to installed apps.
Before the first use, generate the key, write the public key (the contents of the `.pub` file) to
`plugins.updater.pubkey` in `src-tauri/tauri.conf.json`, and register the private key and its
password as two repository secrets.

```sh
pnpm tauri signer generate -w ~/.tauri/tiny-translator-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/tiny-translator-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

Only a build given `src-tauri/tauri.release.conf.json` (`createUpdaterArtifacts`) produces update
artifacts, so a local `pnpm tauri build` does not need the private key. To try an update locally,
pass a different identifier, the URL of a local `latest.json` and
`"dangerousInsecureTransportProtocol": true` with `--config` to build an old and a new version, turn
the new version's update artifacts into `latest.json` with `scripts/write-updater-manifest.py`
(requires `minisign`), and serve them over HTTP.

To build a signed and notarized `.dmg` on a local Mac, prepare `.env.signing` and run
`./scripts/macos-sign-build.sh`. `scripts/macos-verify-gatekeeper.sh` checks the resulting `.app` and
`.dmg` against Gatekeeper's assessment.

## License

tiny-translator itself is licensed under the [MIT License](LICENSE). The licenses of the software
bundled with it are in [THIRD-PARTY-LICENSES.txt](THIRD-PARTY-LICENSES.txt). The translation model,
PLaMo 3 Translate, is part of neither the app nor its bundles; the model fetched with 「モデルを入手」
(Get the model) is under the PLaMo Community License (see "License notices" above).
