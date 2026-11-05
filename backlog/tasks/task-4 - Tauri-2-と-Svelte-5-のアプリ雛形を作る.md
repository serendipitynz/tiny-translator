---
id: TASK-4
title: Tauri 2 と Svelte 5 のアプリ雛形を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
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
- [ ] #1 pnpm tauri dev で空のウィンドウが macOS で開く
- [ ] #2 pnpm check・pnpm lint・pnpm test・cargo clippy・cargo test が通る
- [ ] #3 追加した本番依存を一覧にしてオーナーの承認を得ている
- [ ] #4 README に開発コマンドを書いている
<!-- AC:END -->
