---
id: TASK-24
title: モデルの入手方法を決めて作る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-5
dependencies:
  - TASK-23
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
開発中はモデルを手で置いていたが、配布するならアプリが入手を助ける必要がある。オフラインで使う要求 (R1) と、ライセンスの条件の両方を満たす形を選ぶ (doc-1 §10)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 同梱・初回ダウンロード・手元のファイルの取り込みを比べ、選んだ方式と理由を doc-1 §10 に書いている
- [ ] #2 選んだ方式が macOS と Windows で動き、ダウンロードするならライセンスへの同意と SHA-256 の照合を行う
<!-- AC:END -->
