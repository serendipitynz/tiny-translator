---
id: TASK-1
title: macOS で llama-server により PLaMo 3 Translate を動かし速度とメモリを測る
status: To Do
assignee: []
created_date: '2026-11-05 19:41'
labels: []
milestone: m-0
dependencies: []
references:
  - 'https://huggingface.co/pfnet/plamo-3-translate-2b-GGUF'
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-1 は翻訳エンジンを llama-server の子プロセスにし、既定モデルを 2B Q4_K_M、コンテキスト長を 8,192 トークンとしたが、どれも M1/M2 での実測が無い。主対象の macOS で、PFML3 のプロンプトを /completion に渡して翻訳できるか、16 GB の機械で他のアプリを圧迫しないかを、アプリを作る前に確かめる。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Metal 版 llama-server (取得元とバージョンを記録) が PLaMo 3 Translate の GGUF を読み込み、doc-1 §4.3 の PFML3 プロンプトで英日・日英の翻訳を返す
- [ ] #2 0.8B・2B・8B の Q4_K_M について、読み込み時間、常駐メモリ (RSS とアクティビティモニタのメモリ)、生成速度 (tok/s) をコンテキスト長 8,192 で測り、表にして doc-1 §4.5 と §11 へ反映している
- [ ] #3 llama-server の終了後にメモリが OS へ戻ることを確かめている
- [ ] #4 停止トークン <|plamo:tag|> と <|plamo:bos|> で生成が止まることを確かめている
<!-- AC:END -->
