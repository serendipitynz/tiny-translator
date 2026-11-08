# tiny-translator

PLaMo 3 Translate をローカルで動かす簡易翻訳デスクトップアプリ。設計は `backlog/docs/` の doc-1、
作業の単位は `backlog/tasks/` にある (Backlog.md で管理する)。

## 前提

- Node.js 24 (`.node-version`) と pnpm 10 (`package.json` の `packageManager`)
- Tauri 2 が求める Rust ツールチェーン (macOS では Xcode の Command Line Tools も)
- Windows (x86_64) では、MSVC 向けの Rust (`x86_64-pc-windows-msvc`)、Visual Studio Build Tools の
  「C++ によるデスクトップ開発」、WebView2 ランタイム (Windows 10 以降は通常入っている)

## 翻訳エンジンとモデル

翻訳エンジン (llama-server) は Tauri の sidecar で、リポジトリには含めない。初めてビルドする前に、リポジトリのルートで
次を実行して `src-tauri/binaries/` へ作る (llama.cpp を `~/.cache/tiny-translator/` へ取得して静的にビルドする)。これが無いと `cargo` の
ビルド・テスト・clippy も失敗する。

macOS (Apple Silicon) では Metal 版を作る。CMake が要る。

```sh
./scripts/build-llama-server.sh
```

Windows (x86_64) では Vulkan 版を作る。前提の Visual Studio Build Tools の「C++ によるデスクトップ開発」に加えて
Vulkan SDK (`winget install KhronosGroup.VulkanSDK`) が要る。CMake は PATH に無ければ Visual Studio に同梱のものを使う。
できる exe は OS の DLL と GPU ドライバーが入れる `vulkan-1.dll` だけを読み込む (スクリプトが `dumpbin` で確かめる)。

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-llama-server.ps1
```

モデルの GGUF (既定は `plamo-3-translate-2b-Q4_K_M.gguf`) は、アプリデータディレクトリの `models/` に置く
(macOS では `~/Library/Application Support/com.serendipitynz.tiny-translator/models/`、Windows では
`%APPDATA%\com.serendipitynz.tiny-translator\models\`。シンボリックリンクでもよい)。
翻訳エンジンのログは `llama-server.log` に出る (macOS では `~/Library/Logs/com.serendipitynz.tiny-translator/`、
Windows では `%LOCALAPPDATA%\com.serendipitynz.tiny-translator\logs\`)。

## 開発コマンド

リポジトリのルートで実行する。

```sh
pnpm install
pnpm tauri dev       # アプリを起動する (Vite の開発サーバーも立ち上がる)
pnpm check           # svelte-check (型と Svelte の検査)
pnpm lint            # biome lint
pnpm test            # vitest
pnpm build           # 画面を dist/ へビルドする
```

Rust 側は `src-tauri/` で実行する。

```sh
cargo clippy --all-targets -- -D warnings
cargo test
cargo fmt --check
```

`scripts/` には技術検証 (m-0) で使った計測用のスクリプトがあり、アプリのビルドとは独立している。
