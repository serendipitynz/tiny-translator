---
id: TASK-39
title: Apple の .xcstrings に翻訳先の言語の訳を足す
status: To Do
assignee: []
created_date: '2026-11-11 02:30'
labels: []
dependencies:
  - TASK-31
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-31 で ARB と Apple .strings を足したとき、Xcode の String Catalog (.xcstrings) は別タスクに分けた (2026-11-11 オーナー判断)。.xcstrings は JSON で、1 つのファイルが全言語の訳を持つ。翻訳は sourceLanguage の値を書き換えることではなく、各キーの localizations に翻訳先の言語のエントリ (stringUnit の state と value) を足すことなので、構造を変えずに訳す doc-1 §7.1 の方式とは形が違う。複数形 (variations.plural) とデバイス別の分岐、既にある訳と state (translated・needs_review) の扱い、翻訳先の言語のコード (zh-hans など) と Xcode の言語コード (zh-Hans) の対応を決める必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 訳を足す方針 (既にある訳の扱い、state、複数形とデバイス別の分岐、言語コードの対応) を決め、doc-1 §7.1 に書いている
- [ ] #2 原文の言語の値・コメント・キーの順序を変えずに、翻訳先の言語の訳を足したファイルを出力できる
- [ ] #3 TASK-28 で守ると決めたプレースホルダー (%@・%lld など) が原文のとおりに残る
- [ ] #4 形式の自動判定 (doc-1 §7.2) で判定され、形式の選択欄からも選べる
- [ ] #5 訳したファイルを JSON として読み戻し、String Catalog の構造を保っていることをテストで確かめている
<!-- AC:END -->
