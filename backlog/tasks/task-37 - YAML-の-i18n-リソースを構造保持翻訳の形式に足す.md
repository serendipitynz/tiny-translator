---
id: TASK-37
title: YAML の i18n リソースを構造保持翻訳の形式に足す
status: To Do
assignee: []
created_date: '2026-11-11 02:30'
labels: []
dependencies:
  - TASK-31
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-31 で ARB と Apple .strings を足したとき、YAML (Rails の config/locales、i18next や vue-i18n の YAML など) は別タスクに分けた (2026-11-11 オーナー判断)。キー・コメント・字下げ・引用符の種類を変えずに値だけを差し替えるには、原文の位置を持つ YAML パーサーが要り、依存を足すことになる見込みが高い。ブロックスカラー (| と >)・フロースタイル・アンカーとエイリアス・複数文書の扱いも決める必要がある。依存を足すならオーナーの承認が要る。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 doc-1 §7.1 に YAML の訳す部分と訳さない部分を書いている
- [ ] #2 キー・コメント・字下げ・引用符の種類を変えずに、訳すべき文字列の値だけを訳したファイルを出力できる
- [ ] #3 TASK-28 で守ると決めたプレースホルダーが原文のとおりに残る
- [ ] #4 形式の自動判定 (doc-1 §7.2) で判定され、形式の選択欄からも選べる
- [ ] #5 訳したファイルを YAML パーサーで読み戻せることをテストで確かめている
<!-- AC:END -->
