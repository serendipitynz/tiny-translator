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

モデルの GGUF (既定は `plamo-3-translate-2b-Q4_K_M.gguf`) は、アプリの「モデルを入手」で Hugging Face の
`pfnet/plamo-3-translate-*-GGUF` から取得する (PLaMo Community License への同意と SHA-256 の照合を伴う)。
ネットワークの無い機械では、GGUF をアプリデータディレクトリの `models/` に手で置いてもよい
(macOS では `~/Library/Application Support/com.serendipitynz.tiny-translator/models/`、Windows では
`%APPDATA%\com.serendipitynz.tiny-translator\models\`。シンボリックリンクでもよい)。
翻訳エンジンのログは `llama-server.log` に出る (macOS では `~/Library/Logs/com.serendipitynz.tiny-translator/`、
Windows では `%LOCALAPPDATA%\com.serendipitynz.tiny-translator\logs\`)。

## ライセンスの表示

Built with PLaMo. 翻訳に使うモデル PLaMo 3 Translate は、Preferred Networks の
[PLaMo Community License](https://plamo.preferredai.jp/info/plamo-community-license-ja) で提供される。
アプリの「設定」の「ライセンス」から、条文の全文 (日本語版と英語版) と、使う人自身が守る条件
(業務での利用の登録、長文の公表物での明示、訳文を渡すときの表示、重要な決定での制限) の案内を開ける。

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

`scripts/` には技術検証 (m-0) で使った計測用のスクリプトもあり、アプリのビルドとは独立している。

## 配布物

配布物は macOS (Apple Silicon) の `.dmg` と Windows (x86_64) の NSIS インストーラー (`*-setup.exe`) で、
GitHub Actions の `.github/workflows/release.yml` が作る (設計は doc-1 §13)。

- macOS の `.app` と `.dmg` は Developer ID で署名し、Apple の公証を受けている。
- Windows のインストーラーはコード署名していない。ブラウザーでダウンロードすると、よく使われていないファイルとして警告が出るので、
  「保存」の横のメニューから「保持する」、続けて「保持する」(英語版では Keep → Keep anyway) を選ぶ。そうして保存したインストーラーは、
  2026-11-11 の確認ではダブルクリックで警告なく始まった。開いたときに SmartScreen が「Windows によって PC が保護されました」を出したら、
  「詳細情報」から「実行」を選ぶ。インストール先は利用者ごとの `%LOCALAPPDATA%\tiny-translator\` で、管理者権限は要らない。
- どちらにもモデルは入っていない。初回に「モデルを入手」で取得する。
- 同梱するソフトウェアのライセンスは `THIRD-PARTY-LICENSES.txt` にあり、配布物にも入る
  (macOS は `.app` の `Contents/Resources/`、Windows はインストール先のフォルダー)。

### リリースの手順

1. `package.json`・`src-tauri/tauri.conf.json`・`src-tauri/Cargo.toml` の `version` をそろえて上げ、
   `src-tauri/` で `cargo update -p tiny-translator` を実行して `Cargo.lock` も合わせる。
2. 依存を変えたときは `THIRD-PARTY-LICENSES.txt` を作り直す (`pnpm test` が古いと失敗する)。
   ```sh
   cargo fetch --manifest-path src-tauri/Cargo.toml --target aarch64-apple-darwin --target x86_64-pc-windows-msvc
   node scripts/generate-third-party-licenses.mjs
   ```
3. 既定のブランチにマージしたら、そのコミットに `vX.Y.Z` のタグを付けて push する。
   ワークフローが下書きのリリースを作り、両 OS の配布物を検査してから置く。
4. 下書きの配布物と自動生成のリリースノートを確かめて、手で公開する。

ワークフローは macOS の署名と公証に 6 つのリポジトリシークレットを使う。初めて使う前に、Developer ID Application の証明書を
キーチェーンアクセスから `.p12` で書き出し、`.env.signing` (`.env.signing.example` を写して埋める) を用意して、次で登録する。

```sh
./scripts/setup-ci-signing-secrets.sh path/to/DeveloperID.p12
```

手元の Mac で署名・公証した `.dmg` を作るときは、`.env.signing` を用意して `./scripts/macos-sign-build.sh` を実行する。
できた `.app` と `.dmg` は `scripts/macos-verify-gatekeeper.sh` が Gatekeeper の判定で確かめる。
