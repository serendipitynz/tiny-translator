---
id: TASK-6
title: 2 ペインのメイン画面を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-1
dependencies:
  - TASK-5
references:
  - ../snz-design/backlog/docs/doc-8 - 基本UI部品の共通仕様と擬似コード.md
  - ../snz-design/backlog/docs/doc-9 - 複合UI部品の共通仕様と擬似コード.md
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-1 §8.1 のレイアウトと §8.2 の部品の対応を画面にする。翻訳の処理はまだ繋がないが、以降のフェーズはこの画面へ処理を繋ぐだけにしたい。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 doc-1 §8.1 の要素 (ヘッダーの履歴と設定、翻訳元と翻訳先の選択欄、原文と訳文のパネル、各アイコンのみボタン、入れ替え、翻訳ボタン) がそろっている
- [ ] #2 各部品が snz-design doc-8・doc-9 の状態 (hover・押下・フォーカス・無効・処理中) とトークン用途どおりに描かれ、4 配色で確かめている
- [ ] #3 WKWebView で select を自分で描いた枠の内側に置いている
- [ ] #4 ウィンドウ幅が狭いとき原文と訳文が上下に並ぶ
<!-- AC:END -->
