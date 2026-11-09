---
id: TASK-29
title: ライセンスの表示 (Built with PLaMo・ライセンス全文・利用者への案内) を画面に置く
status: To Do
assignee: []
created_date: '2026-11-09 20:20'
labels: []
milestone: m-5
dependencies:
  - TASK-23
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-23 で PLaMo Community License (2026-11-09 改定版) を読み、アプリが出すべき表示を doc-1 §10.3 に D1〜D5 として一覧にした。D1「Built with PLaMo」と D2 ライセンス全文は、モデルを同梱するときは第 4 条の義務になる。初回のダウンロードでは配布者に義務が生じないと読んだが PFN には確かめていないので、入手方法 (TASK-24) に関わらず出すことにした。D5 は利用者自身が負う条件 (業務での利用の登録、長文の公表物での明示、第 5 条 (4) の表示、重要な決定での利用の制限) の案内で、義務ではないが全文だけでは気付きにくいので添える。D3 (ダウンロード前の同意) は TASK-24 で、D4 (モデル名の PLaMo) は今の形で満たしているので、このタスクの範囲外。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 画面の常に辿れる場所に「Built with PLaMo」を表示し、README にも書いている (doc-1 §10.3 の D1)
- [ ] #2 アプリの中から PLaMo Community License の全文 (日本語版と英語版) と改定日を開ける。全文は GGUF リポジトリの LICENSE/ja・LICENSE/en を書き写さずにそのまま使っている (D2)
- [ ] #3 全文の前に、利用者が負う条件の短い案内 (D5 の 4 項目) を置いている
- [ ] #4 macOS と Windows で、表示と全文を開く操作を確かめている
<!-- AC:END -->
