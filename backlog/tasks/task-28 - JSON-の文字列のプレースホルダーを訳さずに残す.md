---
id: TASK-28
title: JSON の文字列のプレースホルダーを訳さずに残す
status: To Do
assignee: []
created_date: '2026-11-08 21:36'
labels: []
milestone: m-3
dependencies:
  - TASK-15
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
i18n のリソースファイルの文字列は `{name}`・`{{count}}`・`%s`・`%1$s` などのプレースホルダーを含む。TASK-15 の JSON の構造保持翻訳はこれらを文字列の一部としてそのままモデルへ渡すので、プレースホルダーの中の語 (`{name}` の name) が訳されたり、記号が崩れたりすると、訳したリソースファイルがアプリで使えなくなる。TASK-15 の着手時に、守る範囲を決める作業を別タスクに分けた (2026-11-09 オーナー判断)。Markdown のインラインコードと同じ「そのまま」のタグ (doc-1 §7.1、TASK-14) で守る形が候補。ICU の plural・select (`{count, plural, one {# item} other {# items}}`) のように中に訳すべき文を持つ記法をどう扱うかも決める必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 守るプレースホルダーの記法を決め、doc-1 §7.1 の JSON の節に書いている
- [ ] #2 決めた記法のプレースホルダーは、訳文に何が書かれても原文のとおりに戻る
- [ ] #3 プレースホルダーを戻せない訳文の扱い (訳し直し・原文のまま残す) が決まっていて、テストで確かめている
<!-- AC:END -->
