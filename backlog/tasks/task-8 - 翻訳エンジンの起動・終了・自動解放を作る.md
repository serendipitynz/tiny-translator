---
id: TASK-8
title: 翻訳エンジンの起動・終了・自動解放を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-2
dependencies:
  - TASK-1
  - TASK-4
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
メモリを抑えるために翻訳エンジンを子プロセスにし、使わないときは終了する (doc-1 §4.1, §4.5, §5)。子プロセスが残るとメモリを圧迫し続けるので、異常終了の後始末まで含める。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 llama-server を Tauri の sidecar として同梱し、127.0.0.1 の空きポートで起動する
- [ ] #2 アプリ起動直後に翻訳エンジンを起動し、読み込み完了を待てる
- [ ] #3 自動解放までの時間 (既定 10 分、0 は解放しない) が過ぎると翻訳エンジンを終了し、次の翻訳で再起動する
- [ ] #4 アプリ終了時に翻訳エンジンが終了し、前回の異常終了で残ったプロセスを起動時に終了する
- [ ] #5 翻訳エンジンの状態 (停止中・読み込み中・待機中) を画面へ通知できる
<!-- AC:END -->
