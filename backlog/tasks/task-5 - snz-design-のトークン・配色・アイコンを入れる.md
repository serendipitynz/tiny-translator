---
id: TASK-5
title: snz-design のトークン・配色・アイコンを入れる
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-1
dependencies:
  - TASK-4
references:
  - ../snz-design/tokens/README.md
  - ../snz-design/backlog/docs/doc-7 - テーマ切替と既存設定移行の共通仕様.md
  - ../snz-design/backlog/docs/doc-16 - Web系4アプリへの共通デザイン適用ガイド.md
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
見た目は snz-design に従う (R9)。トークンの写し方、配色の 2 軸、最初の描画より前の適用は snz-design の仕様が決めている。backlog-atlas は配色の適用が最初の描画より後でちらつくので、その形を避ける (doc-1 §8.3)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tokens-v0.1.1 の snz-tokens.css を vendor.mjs copy で写し、vendor.mjs verify が通る
- [ ] #2 配色系統 (標準・Solarized) と明暗の選択 (Light・Dark・OS 追従) を保存値から読み、最初の描画より前に data-color-family と data-color-mode を設定している
- [ ] #3 OS 追従のとき、OS の明暗の変更に再起動なしで追う
- [ ] #4 lucide の座標をコピーした Icon コンポーネントがあり、lucide のライセンス表示を同梱している
<!-- AC:END -->
