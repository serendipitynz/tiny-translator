---
id: TASK-7
title: アプリの骨格を Windows で確かめる
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-1
dependencies:
  - TASK-6
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Windows は副対象で、各マイルストーンの最後に確かめる (doc-1 §12)。WebView2 での描画の差をここで拾う。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Windows (x86_64) で pnpm tauri dev が動き、メイン画面が 4 配色で描ける
- [ ] #2 macOS との描画の差を記録し、直したか、直さない理由を残している
<!-- AC:END -->
