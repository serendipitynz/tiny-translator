---
id: TASK-19
title: 翻訳履歴の保存と全文検索を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-4
dependencies:
  - TASK-9
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R7。以前の翻訳を探して再利用したい。日本語も検索できる必要がある (doc-1 §9)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 翻訳が完了するたびに原文・訳文・翻訳元言語・翻訳先言語・形式・モデル名・日時を SQLite へ保存し、停止した翻訳は保存しない
- [ ] #2 FTS5 (trigram) で原文と訳文を日本語・英語とも検索できる
- [ ] #3 保存件数の上限を超えたら古いものから消える
- [ ] #4 保存・検索・削除に単体テストがある
<!-- AC:END -->
