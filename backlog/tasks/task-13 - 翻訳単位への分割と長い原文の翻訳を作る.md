---
id: TASK-13
title: 翻訳単位への分割と長い原文の翻訳を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-3
dependencies:
  - TASK-9
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
  - backlog/docs/doc-2 - 先行事例と差別化.md
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
構造保持翻訳の土台。原文を翻訳単位に分け、直前の対訳を文脈として渡しながら順に訳す (doc-1 §7.3)。小さいモデルは長い出力で繰り返しを起こすので、その検知と訳し直しも持つ (doc-2 §1.1 の先行事例の知見)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 プレーンテキストを空行で翻訳単位に分け、順に翻訳して訳文を組み立てる
- [ ] #2 直前の翻訳単位の原文と訳文を文脈としてプロンプトに入れる
- [ ] #3 コンテキスト長に収まらない翻訳単位を文の境界で分ける
- [ ] #4 出力の繰り返しを検知して打ち切り、文の単位に分けて訳し直す
- [ ] #5 分割と繰り返し検知に単体テストがある
<!-- AC:END -->
