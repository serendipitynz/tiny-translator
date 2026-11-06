---
id: TASK-4
title: Tauri 2 と Svelte 5 のアプリ雛形を作る
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-06 22:10'
labels: []
milestone: m-1
dependencies: []
references:
  - ../backlog-atlas
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-1 §3 は backlog-atlas と同じ構成 (Tauri 2、Svelte 5、scoped SCSS、pnpm、biome、vitest、svelte-check、clippy) を採るとした。以降のタスクがこの雛形の上で進むので、開発コマンドと検査を先にそろえる。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 pnpm tauri dev で空のウィンドウが macOS で開く
- [x] #2 pnpm check・pnpm lint・pnpm test・cargo clippy・cargo test が通る
- [x] #3 追加した本番依存を一覧にしてオーナーの承認を得ている
- [x] #4 README に開発コマンドを書いている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. ルートに雛形を置く (backlog-atlas と同じ配置: src/ に Svelte、src-tauri/ に Rust)。index.html・main.ts・App.svelte は空のウィンドウを出すだけにする
2. 依存は backlog-atlas と同じメジャー版にそろえる (vite 6・@sveltejs/vite-plugin-svelte 5・vitest 4・typescript 5.6・svelte-check 4・biome 2.5.8 固定)
3. 本番依存は Rust の tauri (と build 時の tauri-build) と、画面に同梱される svelte に限る。@tauri-apps/api・serde などは最初に使うタスクで入れる
4. biome は atlas と同じ設定 (preset none + useBlockStatements、formatter 無効)。clippy は -D warnings で通す
5. vitest は空のテストで失敗しないよう passWithNoTests を設定する
6. CSP は atlas に倣って厳しめにする。index.html の head の配色スクリプト (doc-1 §8.3) は TASK-5 で CSP と合わせて扱う
7. README に前提ツールと開発コマンドを書く
8. pnpm tauri dev で macOS の空のウィンドウを確認し、check・lint・test・clippy・cargo test を通す
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 依存はオーナー承認済み (2026-11-07)。本番は Rust の tauri 2 (build 時の tauri-build 2) と、画面のバンドルに入る svelte 5 だけ。@tauri-apps/api と serde は空のウィンドウには要らないので、最初に invoke やコマンドを使うタスクで改めて承認を取る。
- 開発用の依存は backlog-atlas と同じメジャー版にした (vite 6・@sveltejs/vite-plugin-svelte 5・vitest 4・typescript ~5.6・svelte-check 4・sass 1・tslib 2・@biomejs/biome 2.5.8 固定)。最新は vite 8・vitest 5・TS 7 まで出ているが、atlas で snz-design を適用した手順をそのまま流用するため。
- biome は atlas と同じ設定 (formatter 無効、preset none + useBlockStatements)。規則が効くことを、if の本体を波括弧なしで書いた一時ファイルで確かめた (エラー 1 件)。
- vitest は passWithNoTests を設定した。テストが 1 つも無いと vitest run が終了コード 1 になるため。最初のテストファイルを足すときに外す (外さないと include の誤りが黙って通る)。
- CSP は atlas に倣って厳しめにした (script-src 'self')。doc-1 §8.3 の index.html の head 内の配色スクリプトは、TASK-5 で CSP と合わせて扱う必要がある。
- アプリのアイコンは scripts/clipboard_probe の 32x32 の仮のものを写した。正式なアイコンと macOS の icns は配布の準備 (m-5) で作る。
- ルートの .gitignore の dist は /dist/ に固定した。scripts/clipboard_probe/dist/index.html は追跡しているファイルなので、除外の対象にしないため。

確認 (macOS 26.7.1, MacBook Air M2):
- AC #1: pnpm tauri dev で 960x640 の空のウィンドウが開いた (CGWindowList で tiny-translator のウィンドウを確認し、画面を撮って空であることを見た)。
- AC #2: pnpm check (0 errors / 0 warnings)、pnpm lint (7 files, 0 件)、pnpm test (テストなしで終了コード 0)、cargo clippy --all-targets -- -D warnings、cargo test (0 件で成功)、cargo fmt --check がすべて通った。pnpm build も通った。
- AC #4: README.md に前提ツールと開発コマンドを書いた。
<!-- SECTION:NOTES:END -->
