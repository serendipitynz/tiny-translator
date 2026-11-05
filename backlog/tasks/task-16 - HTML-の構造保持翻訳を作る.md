---
id: TASK-16
title: HTML の構造保持翻訳を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-3
dependencies:
  - TASK-13
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
  - backlog/docs/doc-2 - 先行事例と差別化.md
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R8 のうち HTML。インライン要素を含む文はタグごと訳す必要がある。PLaMo 3 Translate はタグを保つ学習をしているが、2B で保たれる割合は未確認 (doc-1 §11)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 HTML パーサーを選定し、本番依存としてオーナーの承認を得ている
- [ ] #2 ブロック要素の中身と alt・title 属性を翻訳し、script・style・code・pre とその他の属性を原文のまま残す
- [ ] #3 インライン要素を番号付きのタグで渡し、訳文で元の要素と属性へ戻す
- [ ] #4 2B でタグの数と順序が保たれた割合を測り、doc-1 §11 へ反映している
<!-- AC:END -->
