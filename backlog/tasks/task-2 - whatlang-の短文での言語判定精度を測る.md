---
id: TASK-2
title: whatlang の短文での言語判定精度を測る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-0
dependencies: []
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
優先順位による翻訳先の決定 (doc-1 §6.2) は、アプリ側で翻訳元言語を判定できることが前提になる。クリップボードから来る原文は単語 1 つや短い 1 文のことが多く、whatlang がそこで当たるかが分からない。日本語の仮名優先の判定と、確からしさが低いときにモデルの推定へ任せる閾値もここで決める。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 日本語・英語・中国語 (簡体・繁体)・韓国語を含む短文 (1 語・1 文・段落) の判定結果を表にしている
- [ ] #2 漢字だけの日本語の短文を、仮名の有無の判定と組み合わせてどれだけ正しく扱えるかを記録している
- [ ] #3 確からしさの閾値を決め、doc-1 §6.1 へ反映している
<!-- AC:END -->
