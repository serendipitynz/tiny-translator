---
id: TASK-31
title: i18n のリソースファイルの形式を足す
status: To Do
assignee: []
created_date: '2026-11-10 09:59'
labels: []
dependencies:
  - TASK-28
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
先行事例と差をつけるため、構造保持翻訳 (m-3) を開発者が自分のアプリを多言語化する作業へ広げる (2026-11-10 オーナー判断、doc-2 §2)。PLaMo 3 Translate を使う他のアプリはブラウザ拡張・画面への重ね表示・選択範囲の翻訳など「読む」用途に寄っていて、リソースファイルを壊さずに訳せるものは無い。今の構造保持は Markdown・JSON・HTML だけで、i18n では YAML、Apple の .strings / .xcstrings、gettext の .po、Flutter の ARB、Android の strings.xml もよく使われる。どの形式を対象にするかは着手時に決める。プレースホルダーの保持は TASK-28 で決める記法に従う。形式が多いので、着手時に形式ごとのタスクへ分けてもよい。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 対象にする形式を決め、doc-1 §7 に形式ごとの訳す部分と訳さない部分を書いている
- [ ] #2 決めた形式それぞれで、キー・コメント・構造を変えずに、訳すべき文字列だけを訳したファイルを出力できる
- [ ] #3 決めた形式それぞれで、TASK-28 で守ると決めたプレースホルダーが原文のとおりに残る
- [ ] #4 決めた形式が形式の自動判定 (doc-1 §7.2) で判定され、形式の選択欄からも選べる
- [ ] #5 形式ごとに、訳したファイルをその形式のパーサーで読み戻せることをテストで確かめている
<!-- AC:END -->
