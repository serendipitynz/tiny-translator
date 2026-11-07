---
id: TASK-9
title: ストリーミング翻訳と停止を作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-07 04:19'
labels: []
milestone: m-2
dependencies:
  - TASK-8
  - TASK-6
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
原文を翻訳エンジンへ渡し、訳文を生成されるそばから訳文欄へ出す (doc-1 §4.3)。小さいモデルでも長い訳文は時間がかかるので、途中で止められる必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 翻訳ボタンと Ctrl/Cmd+Enter で、PFML3 のプロンプトを /completion に渡し、訳文がストリーミングで訳文欄に出る
- [ ] #2 翻訳中は翻訳ボタンが処理中の表示になり、停止できる
- [ ] #3 翻訳エンジンが停止中なら起動を待ってから翻訳する
- [ ] #4 プロンプトの組み立てに単体テストがある
- [ ] #5 翻訳エンジンを起動のたびに作る乱数の API キー (--api-key) 付きで起動し、翻訳の要求にそのキーを付ける。キーの無い要求 (ブラウザのページからの要求など) は受け付けない
<!-- AC:END -->
