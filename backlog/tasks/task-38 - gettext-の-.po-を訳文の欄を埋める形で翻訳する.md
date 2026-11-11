---
id: TASK-38
title: gettext の .po を訳文の欄を埋める形で翻訳する
status: To Do
assignee: []
created_date: '2026-11-11 02:30'
updated_date: '2026-11-11 02:30'
labels: []
dependencies:
  - TASK-31
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-31 で ARB と Apple .strings を足したとき、gettext の .po は別タスクに分けた (2026-11-11 オーナー判断)。.po はほかの形式と違い、原文 (msgid) を書き換えずに訳文の欄 (msgstr) を埋めるのが翻訳であり、構造を変えずに訳す doc-1 §7.1 の方式とは形が違う。複数形 (msgid_plural と msgstr[n]) は翻訳先の言語で数が変わり (日本語は 1 つ)、ヘッダーの Plural-Forms・Language を書き換えるか、既に訳のある msgstr と fuzzy の印をどう扱うかも決める必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 msgstr を埋める方針 (空の msgstr だけか、既にある訳も訳し直すか、複数形とヘッダーの扱い) を決め、doc-1 §7.1 に書いている
- [ ] #2 msgid・msgctxt・コメント (#.・#:・#,) を変えずに msgstr を埋めたファイルを出力できる
- [ ] #3 TASK-28 で守ると決めたプレースホルダー (%s・%(name)s・{name} など) が msgid のとおりに msgstr に残る
- [ ] #4 形式の自動判定 (doc-1 §7.2) で判定され、形式の選択欄からも選べる
- [ ] #5 訳したファイルを .po のパーサーで読み戻せることをテストで確かめている
<!-- AC:END -->
