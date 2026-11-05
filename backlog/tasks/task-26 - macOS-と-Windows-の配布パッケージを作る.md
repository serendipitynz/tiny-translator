---
id: TASK-26
title: macOS と Windows の配布パッケージを作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-5
dependencies:
  - TASK-24
  - TASK-25
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
配布すると決めた場合だけ行う。macOS は公証が無いと Gatekeeper で開けない。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 macOS の .dmg を署名・公証して作れる
- [ ] #2 Windows のインストーラーを作れる
- [ ] #3 サードパーティのライセンス表示 (llama.cpp・lucide・依存クレートと npm パッケージ) を同梱している
<!-- AC:END -->
