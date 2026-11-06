# tiny-translator

PLaMo 3 Translate をローカルで動かす簡易翻訳デスクトップアプリ。設計は `backlog/docs/` の doc-1、
作業の単位は `backlog/tasks/` にある (Backlog.md で管理する)。

## 前提

- Node.js 24 (`.node-version`) と pnpm 10 (`package.json` の `packageManager`)
- Tauri 2 が求める Rust ツールチェーン (macOS では Xcode の Command Line Tools も)

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
