---
id: TASK-18
title: 構造保持翻訳を Windows で確かめる
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-3
dependencies:
  - TASK-17
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
解析と再構成は Rust で OS に依存しないはずだが、改行コード (CRLF) とクリップボードの形式で差が出うる。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Windows で Markdown・JSON・HTML の構造保持翻訳が macOS と同じ結果になる
- [ ] #2 CRLF の原文で構造が崩れないことを確かめている
<!-- AC:END -->
