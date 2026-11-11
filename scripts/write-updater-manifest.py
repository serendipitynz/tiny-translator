#!/usr/bin/env python3
"""Write the updater's latest.json for one release (doc-1 §14).

Usage: write-updater-manifest.py <packages dir> <out> <version> <base url>

<packages dir> holds the release's two updater packages, each beside its .sig: the macOS
`*.app.tar.gz` and the Windows `*-setup.exe`. Each package is named in latest.json as
<base url>/<file name>. Before anything is written, each signature is checked with minisign
against the public key in src-tauri/tauri.conf.json, the key the app verifies with, and must name
<version> in its trusted comment, so a release signed with the wrong key or for another version
fails here rather than in every installed app. Needs `minisign` on PATH.

The release workflow runs it; it runs as well against a local folder and a local URL, which is how
an update is tried out before a release (README).
"""

import base64
import datetime
import json
import subprocess
import sys
import tempfile
from pathlib import Path

PLATFORMS = (("darwin-aarch64", "*.app.tar.gz"), ("windows-x86_64", "*-setup.exe"))
CONFIG = Path(__file__).resolve().parent.parent / "src-tauri" / "tauri.conf.json"


def fail(message: str) -> None:
    sys.exit(f"::error::{message}")


def main() -> None:
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    folder, out, version, base_url = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3], sys.argv[4]
    pubkey = json.loads(CONFIG.read_text(encoding="utf-8"))["plugins"]["updater"]["pubkey"]
    scratch = Path(tempfile.mkdtemp())
    (scratch / "key.pub").write_bytes(base64.b64decode(pubkey))

    platforms = {}
    for key, pattern in PLATFORMS:
        found = sorted(folder.glob(pattern))
        if len(found) != 1:
            fail(f"expected one {pattern} in {folder}, found {[path.name for path in found]}")
        package = found[0]
        signature_file = package.with_name(package.name + ".sig")
        if not signature_file.is_file():
            fail(f"{package.name} has no signature beside it")
        signature = signature_file.read_text(encoding="utf-8").strip()
        # Tauri's .sig is a minisign signature file, base64-encoded once more.
        (scratch / "package.minisig").write_bytes(base64.b64decode(signature))
        result = subprocess.run(
            ["minisign", "-V", "-p", str(scratch / "key.pub"), "-m", str(package), "-x", str(scratch / "package.minisig")],
            capture_output=True,
            text=True,
        )
        print(result.stdout + result.stderr, end="")
        if result.returncode != 0:
            fail(f"{package.name}'s signature does not verify against the updater public key in tauri.conf.json")
        # The app is built with requireSignedVersion, so a signature naming another version, or
        # none, would be refused by every installed app after the release is published.
        trusted = next(
            (line for line in (scratch / "package.minisig").read_text(encoding="utf-8").splitlines()
             if line.startswith("trusted comment: ")),
            "",
        )
        if f"version:{version}" not in trusted.removeprefix("trusted comment: ").split("\t"):
            fail(f"{package.name} is signed for another version than {version}: {trusted!r}")
        platforms[key] = {"signature": signature, "url": f"{base_url.rstrip('/')}/{package.name}"}

    manifest = {
        "version": version,
        "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": platforms,
    }
    out.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(out.read_text(encoding="utf-8"), end="")


if __name__ == "__main__":
    main()
