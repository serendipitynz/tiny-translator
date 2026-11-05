---
id: TASK-17
title: 形式の自動判定と形式の選択を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-3
dependencies:
  - TASK-14
  - TASK-15
  - TASK-16
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
利用者が形式を毎回選ばなくて済むように、原文から形式を判定する (doc-1 §7.2)。誤判定のときは手で選び直せる必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 原文欄の上に形式の選択 (自動・テキスト・Markdown・JSON・HTML) があり、自動のとき判定した形式を出す
- [ ] #2 doc-1 §7.2 の規則で判定し、判定の単体テストがある
<!-- AC:END -->
