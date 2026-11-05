---
id: TASK-10
title: 翻訳の向き (翻訳元言語の判定と優先順位による翻訳先の決定) を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-2
dependencies:
  - TASK-9
  - TASK-2
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R5 の中心。翻訳元は自動判定を基本にし、翻訳先は第一優先言語と第二優先言語で決める (doc-1 §6.1, §6.2)。選択欄で言語を固定することもできる。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 翻訳元の選択欄が自動判定のとき whatlang で判定し、判定した言語を画面に出す
- [ ] #2 翻訳先の選択欄が「自動 (優先順位)」のとき、doc-1 §6.2 の規則で翻訳先を決める
- [ ] #3 選択欄で言語を指定するとその言語に固定される
- [ ] #4 入れ替えボタンで翻訳元と翻訳先の選択と、訳文と原文が入れ替わる
- [ ] #5 翻訳先の決定に単体テストがある
<!-- AC:END -->
