---
id: TASK-15
title: JSON の構造保持翻訳を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-3
dependencies:
  - TASK-13
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R8 のうち JSON。i18n のリソースファイルなどを訳す用途を想定し、キーと値の型を保つ (doc-1 §7.1)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 文字列の値だけを翻訳し、キー・数値・真偽値・null と入れ子の構造を保つ
- [ ] #2 翻訳しないキーを設定で指定できる
- [ ] #3 訳文が JSON として読め、キーの順序が原文と同じであることをテストで確かめている
<!-- AC:END -->
