---
id: TASK-21
title: 設定画面を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-08 21:37'
labels: []
milestone: m-4
dependencies:
  - TASK-10
  - TASK-5
references:
  - ../snz-design/backlog/docs/doc-9 - 複合UI部品の共通仕様と擬似コード.md
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
ここまでのフェーズで既定値にした項目を利用者が変えられるようにする。設定は再起動後も残る必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 第一優先言語と第二優先言語、自動解放までの時間、使うモデル (所定のディレクトリの GGUF から選ぶ)、起動時のクリップボード翻訳の有効・無効、配色の 2 軸を設定できる
- [ ] #2 設定画面を snz-design doc-9 §6.6 のモーダルと §7 の 2 軸の設定画面の実例に従って描く
- [ ] #3 翻訳エンジンの状態とメモリ使用量の目安を表示する
- [ ] #4 設定が再起動後も保たれ、読めない設定ファイルは上書きせず退避する
- [ ] #5 JSON の翻訳しないキー (doc-1 §7.1、TASK-15 で translate が受け取る一覧) を設定できる
<!-- AC:END -->
