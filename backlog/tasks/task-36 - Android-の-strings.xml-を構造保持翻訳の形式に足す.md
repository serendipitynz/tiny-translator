---
id: TASK-36
title: Android の strings.xml を構造保持翻訳の形式に足す
status: To Do
assignee: []
created_date: '2026-11-11 02:30'
labels: []
dependencies:
  - TASK-31
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-31 で i18n のリソースファイルの形式として ARB と Apple .strings を足したとき、Android の strings.xml は別タスクに分けた (2026-11-11 オーナー判断、doc-1 §7.1 の ARB の節)。strings.xml は XML で、<string>・<plurals> の <item>・<string-array> の <item> の中身が訳す文字列だが、translatable="false" の要素、<xliff:g> で囲んだ訳さない部分、\' や \" のエスケープ、@string/ の参照、CDATA、中に書いた <b> などの HTML を扱う必要がある。位置付きで読む XML パーサーの依存 (quick-xml など) を足すか、HTML の翻訳で使う html5gum を流用するかを着手時に決める。依存を足すならオーナーの承認が要る。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 doc-1 §7.1 に strings.xml の訳す部分と訳さない部分を書いている
- [ ] #2 要素・属性・コメント・translatable="false" の値・<xliff:g> の中身を変えずに、訳すべき文字列だけを訳したファイルを出力できる
- [ ] #3 TASK-28 で守ると決めたプレースホルダー (%s・%1$d など) が原文のとおりに残る
- [ ] #4 形式の自動判定 (doc-1 §7.2) で判定され、形式の選択欄からも選べる
- [ ] #5 訳したファイルを XML パーサーで読み戻せることをテストで確かめている
<!-- AC:END -->
