---
id: TASK-34
title: コマンドラインから翻訳する CLI を作る
status: To Do
assignee: []
created_date: '2026-11-10 09:59'
labels: []
dependencies: []
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
構造保持翻訳をファイルにまとめて使ったり (README.md から README.ja.md を作るなど)、スクリプトやエディタから呼んだりするには、画面を通さない入口が要る (2026-11-10 オーナー判断、doc-2 §2 の差別化)。MCP サーバーではなく CLI にする (同日オーナー判断)。PLaMo 公式の plamo-translate-cli は PLaMo 2 向けで Python 環境が要り、構造保持も無い。翻訳エンジン・モデル・設定はアプリと同じものを使い、アプリが起動していなくても動くことを想定する。配布物への入れ方 (アプリ同梱か別か、PATH への置き方) は着手時に決める。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ファイルか標準入力の原文を、指定した翻訳先言語へ訳し、ファイルか標準出力へ書ける
- [ ] #2 形式 (プレーンテキスト・Markdown・JSON・HTML) を自動判定し、引数でも指定できる
- [ ] #3 アプリがダウンロードしたモデルと翻訳エンジンを使い、アプリが起動していなくても動く。モデルが無いときは入手方法を示して失敗する
- [ ] #4 失敗したときは標準エラーに理由を出し、0 以外の終了コードで終わる
- [ ] #5 使い方と配布物からの入手方法を README に書いている
<!-- AC:END -->
