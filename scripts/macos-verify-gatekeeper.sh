#!/usr/bin/env sh
# Check that a built macOS bundle passes Gatekeeper the way a downloaded copy
# has to, and exit non-zero when it does not.
#
#     ./scripts/macos-verify-gatekeeper.sh [path ...]
#
# With no arguments it checks every .app and .dmg under src-tauri/target's
# bundle directories. It also runs against a release asset downloaded from
# GitHub, which is the copy a user actually opens.
#
# On the .app: the signature verifies, its authority is a Developer ID
# Application certificate, hardened runtime is on, a notarization ticket is
# stapled, and `spctl` accepts it as notarized.
#
# On the .dmg: a ticket is stapled, `spctl` accepts it under the
# primary-signature context — the assessment Gatekeeper runs when a quarantined
# disk image is opened — and, on a copy carrying the quarantine attribute, the
# .app inside the mounted image is accepted as well. The quarantine value has
# the shape `flags;time;agent;uuid`; `0081` is the flags word a browser download
# on this machine was observed to write. Its presence is what makes
# LaunchServices assess the image at all; `spctl` runs that same assessment on
# demand, and that assessment is what the checks below read.
#
# The stapled ticket is what lets the assessment succeed with no network, so a
# failure here is also the signal that the .dmg went out un-notarized.
set -eu

root=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
fail=0
mount_dir=''
work_dir=''

problem() {
  echo "  FAIL: $1" >&2
  fail=1
}

release_mount() {
  if [ -n "$mount_dir" ]; then
    hdiutil detach "$mount_dir" -quiet >/dev/null 2>&1 || true
    rmdir "$mount_dir" >/dev/null 2>&1 || true
    mount_dir=''
  fi
  if [ -n "$work_dir" ]; then
    rm -rf "$work_dir"
    work_dir=''
  fi
}

# INT/TERM get their own handler that exits. A handler that only cleans up
# returns to where the signal landed, so a Ctrl-C during a mount would detach the
# image and then carry on reporting "the quarantined image would not mount" —
# describing the interrupt as if it were a finding about the bundle.
trap release_mount EXIT
trap 'release_mount; trap - EXIT; exit 130' INT
trap 'release_mount; trap - EXIT; exit 143' TERM

check_app() {
  app=$1
  echo "app: $app"

  if ! verify=$(codesign --verify --strict --verbose=2 "$app" 2>&1); then
    problem "the signature does not verify — $verify"
    return
  fi

  info=$(codesign -dv --verbose=4 "$app" 2>&1)
  case "$info" in
    *"Authority=Developer ID Application"*) : ;;
    *) problem "not signed by a Developer ID Application certificate" ;;
  esac
  case "$info" in
    *"(runtime)"*) : ;;
    *) problem "hardened runtime is not enabled" ;;
  esac

  if ! xcrun stapler validate "$app" >/dev/null 2>&1; then
    problem "no notarization ticket is stapled"
  fi

  assessment=$(spctl -a -vvv -t execute "$app" 2>&1) || true
  case "$assessment" in
    *"source=Notarized Developer ID"*) echo "  accepted: notarized Developer ID" ;;
    *) problem "Gatekeeper does not accept it — $assessment" ;;
  esac
}

# Assess the .app inside a quarantined copy of the image, which is the path a
# downloaded .dmg takes. Working on a copy leaves the build output untouched.
check_dmg_download_path() {
  dmg=$1
  work_dir=$(mktemp -d)
  copy="$work_dir/$(basename "$dmg")"
  cp "$dmg" "$copy"
  xattr -w com.apple.quarantine "0081;00000000;macos-verify-gatekeeper;$(uuidgen)" "$copy"
  if ! xattr -p com.apple.quarantine "$copy" >/dev/null 2>&1; then
    problem "the quarantine attribute did not stick to the copy"
    release_mount
    return
  fi

  mount_dir=$(mktemp -d)
  if ! hdiutil attach "$copy" -nobrowse -readonly -mountpoint "$mount_dir" >/dev/null 2>&1; then
    problem "the quarantined image would not mount"
    release_mount
    return
  fi

  mounted=0
  for app in "$mount_dir"/*.app; do
    [ -e "$app" ] || continue
    mounted=1
    assessment=$(spctl -a -vvv -t execute "$app" 2>&1) || true
    case "$assessment" in
      *"source=Notarized Developer ID"*)
        echo "  accepted from the quarantined image: $(basename "$app")" ;;
      *) problem "the app inside the quarantined image is refused — $assessment" ;;
    esac
  done
  [ "$mounted" -eq 1 ] || problem "the image holds no .app"

  release_mount
}

check_dmg() {
  dmg=$1
  echo "dmg: $dmg"

  if ! xcrun stapler validate "$dmg" >/dev/null 2>&1; then
    problem "no notarization ticket is stapled to the image"
  fi

  assessment=$(spctl -a -t open --context context:primary-signature -vvv "$dmg" 2>&1) || true
  case "$assessment" in
    *"source=Notarized Developer ID"*) echo "  accepted: notarized Developer ID" ;;
    *) problem "Gatekeeper does not accept the image — $assessment" ;;
  esac

  check_dmg_download_path "$dmg"
}

if [ "$#" -eq 0 ]; then
  for path in \
    "$root"/src-tauri/target/release/bundle/macos/*.app \
    "$root"/src-tauri/target/*/release/bundle/macos/*.app \
    "$root"/src-tauri/target/release/bundle/dmg/*.dmg \
    "$root"/src-tauri/target/*/release/bundle/dmg/*.dmg; do
    [ -e "$path" ] || continue
    set -- "$@" "$path"
  done
  if [ "$#" -eq 0 ]; then
    echo "error: no .app or .dmg under src-tauri/target — build one first." >&2
    exit 1
  fi
fi

for path in "$@"; do
  if [ ! -e "$path" ]; then
    problem "$path does not exist"
    continue
  fi
  case "$path" in
    *.app) check_app "$path" ;;
    *.dmg) check_dmg "$path" ;;
    *) problem "$path is neither a .app nor a .dmg" ;;
  esac
done

exit "$fail"
