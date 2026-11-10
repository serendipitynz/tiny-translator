#!/usr/bin/env sh
# Build a macOS bundle that is signed with a Developer ID Application
# certificate and notarized by Apple.
#
# Copied from backlog-atlas. Credentials come from `.env.signing` (git-ignored; see
# `.env.signing.example`). The sidecar (src-tauri/binaries/) must be built first.
# With them exported, `pnpm tauri build` signs the .app with hardened runtime,
# submits it to Apple's notary service, and staples the ticket. Extra arguments
# are forwarded to the build, e.g.
#     ./scripts/macos-sign-build.sh --verbose
# The build runs from the repository root (see below), so a relative path inside
# those arguments resolves against the root, not against the caller's directory.
#
# Tauri does NOT notarize the .dmg that wraps the .app: the CLI drives
# bundle_dmg.sh, that script takes a --notarize option, and the CLI's argument
# list does not pass it (read out of the shipped cli.darwin-arm64.node in
# @tauri-apps/cli 2.11.4). An un-notarized disk image is what a user downloads,
# so this script notarizes and staples each produced .dmg afterwards.
#
# Without `.env.signing` this script refuses to run — signing is its whole
# purpose. The plain `pnpm tauri build` remains the unsigned path and needs
# none of these values.
set -eu

root=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
env_file="$root/.env.signing"

if [ ! -f "$env_file" ]; then
  echo "error: $env_file not found." >&2
  echo "       Copy .env.signing.example to .env.signing and fill it in." >&2
  exit 1
fi

# Export every variable assigned in the env file so the build inherits them.
set -a
# shellcheck source=/dev/null
. "$env_file"
set +a

for var in APPLE_SIGNING_IDENTITY APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID; do
  eval "value=\${$var:-}"
  [ -n "$value" ] || { echo "error: $var is empty in $env_file." >&2; exit 1; }
done

# Resolve the identity before building. A release build takes minutes, and Tauri
# reports an unusable identity only once it reaches the signing step.
if ! security find-identity -v -p codesigning | grep -qF "$APPLE_SIGNING_IDENTITY"; then
  echo "error: APPLE_SIGNING_IDENTITY is not in the login keychain." >&2
  echo "       Listed identities:" >&2
  security find-identity -v -p codesigning >&2
  exit 1
fi

# Build from the repository root: `pnpm tauri build` picks its project up from
# the working directory while the bundle paths below are resolved against $root,
# so a run started elsewhere would build one tree and staple another.
cd "$root"

# Everything after the build acts on the artifacts THIS run wrote, selected by
# being newer than this stamp. A plain `pnpm tauri build` leaves an unsigned
# bundle under target/release/, and a `--target …` run writes to a different
# path, so globbing both would reach the stale unsigned image first: notarization
# rejects it, `stapler staple` fails, and `set -e` ends the run before the bundle
# that was just built is ever notarized.
stamp=$(mktemp)
artifacts=$(mktemp)
# INT/TERM exit rather than returning to where the signal landed. A handler that
# only cleans up would let a Ctrl-C during `stapler validate` read as "not
# stapled" and start a multi-minute notary submission, and would delete the
# artifacts file the verification below still has to read.
trap 'rm -f "$stamp" "$artifacts"' EXIT
trap 'rm -f "$stamp" "$artifacts"; trap - EXIT; exit 130' INT
trap 'rm -f "$stamp" "$artifacts"; trap - EXIT; exit 143' TERM

pnpm tauri build "$@"

# A .app's own directory mtime does not have to move when only its signature is
# replaced, but Contents/ does — codesign writes _CodeSignature/ into it. Match
# on Contents and strip it back to the bundle.
find "$root/src-tauri/target" -maxdepth 6 -newer "$stamp" \
  \( -path '*/release/bundle/macos/*.app/Contents' \
     -o -path '*/release/bundle/dmg/*.dmg' \) -print \
  | sed 's@/Contents$@@' > "$artifacts"

grep -q '\.app$' "$artifacts" || {
  echo "error: the build wrote no .app newer than its own start." >&2
  echo "       Refusing to notarize or verify artifacts this run did not produce." >&2
  exit 1
}

found_dmg=0
while IFS= read -r path; do
  case "$path" in *.dmg) ;; *) continue ;; esac
  found_dmg=1
  if xcrun stapler validate "$path" >/dev/null 2>&1; then
    echo "Already stapled, skipping: $path"
    continue
  fi
  echo "Notarizing DMG: $path"
  # The password rides on argv here, unlike the .p12 password in
  # setup-ci-signing-secrets.sh, which is fed on stdin. notarytool offers no
  # stdin form — the alternative is a stored keychain profile — and switching
  # only this call would not reduce the exposure: `pnpm tauri build` above runs
  # notarytool the same way for the .app, minutes earlier in the same run.
  xcrun notarytool submit "$path" \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait
  xcrun stapler staple "$path"
done < "$artifacts"
[ "$found_dmg" -eq 1 ] || echo "note: this build produced no .dmg (its targets may not include one)."

echo
echo "Verifying the bundles this build produced."
# Hand the verifier the exact paths. Its own discovery sweeps every target path,
# which would pull in leftovers from an earlier unsigned build and fail a run
# that in fact succeeded. The forwarded build arguments are spent by now, so
# reusing the positional parameters here costs nothing.
set --
while IFS= read -r path; do
  set -- "$@" "$path"
done < "$artifacts"
"$root/scripts/macos-verify-gatekeeper.sh" "$@"
