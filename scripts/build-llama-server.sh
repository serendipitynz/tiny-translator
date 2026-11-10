#!/bin/sh
# Builds llama-server as a single static binary and puts it where Tauri's `bundle.externalBin`
# expects the translation engine's sidecar (doc-1 §4.1, §4.2).
#
# Built from source rather than taken from the GitHub release: the release's llama-server is a
# 50 KB stub that loads a dozen dylibs through @loader_path, and externalBin ships exactly one file
# — next to the app's executable in development and inside the bundle alike.
#
# The tag is the one TASK-1 measured (doc-1 §4.5); the commit pins it against a moved tag.
set -eu

tag=b12110
commit=cd959bae9

root=$(cd "$(dirname "$0")/.." && pwd)
triple=$(rustc -vV | sed -n 's/^host: //p')
out="$root/src-tauri/binaries/llama-server-$triple"
# Outside the repository: llama.cpp carries its own Svelte UI, which svelte-check and other
# workspace-wide tools would otherwise find and try to load.
work="${XDG_CACHE_HOME:-$HOME/.cache}/tiny-translator/llama.cpp-$tag"

case "$triple" in
  aarch64-apple-darwin) ;;
  *)
    echo "build-llama-server.sh: no build defined for $triple" >&2
    case "$triple" in
      *-windows-*) echo "on Windows, run scripts/build-llama-server.ps1 (the Vulkan build, doc-1 §4.2)" >&2 ;;
    esac
    exit 1
    ;;
esac

if [ ! -d "$work/.git" ]; then
  git clone --quiet --depth 1 --branch "$tag" https://github.com/ggml-org/llama.cpp.git "$work"
fi
actual=$(git -C "$work" rev-parse --short=9 HEAD)
if [ "$actual" != "$commit" ]; then
  echo "build-llama-server.sh: $tag is $actual, expected $commit" >&2
  exit 1
fi

# The app starts the engine with --no-webui, so llama.cpp's Web UI would be dead weight whose npm
# dependencies the shipped licence notice would have to list. With LLAMA_USE_PREBUILT_UI on (the
# default) the build downloads it from Hugging Face and embeds it; with it off and no assets present
# the server is built without one. llama.cpp embeds whatever an earlier build left in the build
# tree's dist/, so that is removed first.
rm -rf "$work/build/tools/ui/dist" "$work/build/tools/ui/.ui-stamp"

# GGML_NATIVE=OFF keeps the CPU code free of instructions an M2 has and an M1 lacks; the model runs
# on Metal, so the CPU path's speed hardly matters. LLAMA_CURL=OFF and LLAMA_OPENSSL=OFF: the engine
# reads a local GGUF and must not reach the network (R1), and OpenSSL would otherwise be linked from
# wherever Homebrew put it on the building machine.
cmake -S "$work" -B "$work/build" \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_SHARED_LIBS=OFF \
  -DGGML_METAL=ON \
  -DGGML_METAL_EMBED_LIBRARY=ON \
  -DGGML_NATIVE=OFF \
  -DGGML_RPC=OFF \
  -DLLAMA_CURL=OFF \
  -DLLAMA_OPENSSL=OFF \
  -DLLAMA_BUILD_TESTS=OFF \
  -DLLAMA_BUILD_EXAMPLES=OFF \
  -DLLAMA_BUILD_UI=OFF \
  -DLLAMA_USE_PREBUILT_UI=OFF
cmake --build "$work/build" --config Release --target llama-server -j "$(sysctl -n hw.ncpu)"

if grep -q 'asset_0' "$work/build/tools/ui/ui.cpp"; then
  echo "build-llama-server.sh: llama-server embeds a Web UI; it must be built without one" >&2
  exit 1
fi

# The sidecar ships alone, so it may link only what every Mac has.
foreign=$(otool -L "$work/build/bin/llama-server" | tail -n +2 | awk '{print $1}' | grep -v -e '^/System/' -e '^/usr/lib/' || true)
if [ -n "$foreign" ]; then
  echo "build-llama-server.sh: llama-server links libraries outside the OS:" >&2
  echo "$foreign" >&2
  exit 1
fi

mkdir -p "$(dirname "$out")"
cp "$work/build/bin/llama-server" "$out"
"$out" --version
echo "built $out"
