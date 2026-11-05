---
id: TASK-3
title: macOS でクリップボードを読むときの確認の有無を確かめる
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-0
dependencies: []
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
起動時のクリップボード翻訳 (doc-1 §6.3) は、起動直後にアプリがクリップボードを読む。新しい macOS ではアプリがプログラムからクリップボードを読むと確認が出ることがあり、出るなら起動のたびに利用者の操作が増える。Tauri の clipboard プラグインで最小の確認アプリを作って確かめる。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 現行の macOS (バージョンを記録) で、Tauri の clipboard-manager プラグインがテキストを読むときに確認が出るかどうかを記録している
- [ ] #2 確認が出る場合、許可を保存できるか、出さない読み方があるかを調べ、doc-1 §6.3 へ反映している
<!-- AC:END -->
