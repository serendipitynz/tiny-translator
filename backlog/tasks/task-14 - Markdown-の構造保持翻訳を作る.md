---
id: TASK-14
title: Markdown の構造保持翻訳を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-3
dependencies:
  - TASK-13
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R8 のうち Markdown。コードやリンク先を訳すと壊れるので、翻訳しないものを解析で確実に外す (doc-1 §7.1)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 段落・見出し・リスト項目・表のセル・引用を翻訳し、コードブロック・インラインコード・URL・HTML ブロック・front matter を原文のまま残す
- [ ] #2 強調とリンクを番号付きのタグにして渡し、訳文で元の記法へ戻す。タグが合わないときはタグを外して訳し直す
- [ ] #3 代表的な Markdown の入出力を固定したテストがある
<!-- AC:END -->
