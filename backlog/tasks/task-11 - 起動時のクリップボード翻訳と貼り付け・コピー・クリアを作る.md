---
id: TASK-11
title: 起動時のクリップボード翻訳と貼り付け・コピー・クリアを作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-2
dependencies:
  - TASK-9
  - TASK-3
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R3 と R4。起動したらクリップボードの文字列をすぐ訳したい。2 回目の起動でも既存のウィンドウで同じことをする (doc-1 §5, §6.3)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 起動直後にクリップボードがテキストを持てば原文欄へ入れて翻訳を始める。前回の起動時と同じ文字列なら翻訳しない
- [ ] #2 単一インスタンスで、2 回目の起動は既存のウィンドウを前面に出して起動時のクリップボード翻訳を行う
- [ ] #3 原文の貼り付け・コピー・クリア、訳文のコピーができ、コピーしたことを一時的な通知で伝える
- [ ] #4 ウィンドウの位置と大きさが再起動後も保たれる
<!-- AC:END -->
