---
id: TASK-27
title: WebView2 が持つブラウザのキー操作を止める
status: To Do
assignee: []
created_date: '2026-11-07 00:44'
labels: []
milestone: m-2
dependencies:
  - TASK-6
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-7 で Windows (WebView2) の pnpm tauri dev を確かめたところ、F5 と Ctrl+R で画面が再読み込みされ、Ctrl+P で印刷プレビュー、Ctrl+F で検索バーが開いた。macOS の WKWebView には無い挙動である。今は入力しか状態が無いので実害は小さいが、翻訳ができるようになると F5 一つで原文・訳文・進行中の翻訳が消える。Tauri 2.12 は WebView2 の AreBrowserAcceleratorKeysEnabled を設定から変えられないので、止め方も含めて決める。mallow (TASK-27) は Ctrl+P を、印刷できない画面でもアプリ全体の keydown で常に握りつぶすことで止めている。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Windows で F5・Ctrl+R・Ctrl+Shift+R を押しても画面が再読み込みされず、入力中の原文が残る
- [ ] #2 Windows で Ctrl+P を押しても印刷プレビューが開かない
- [ ] #3 原文の入力欄で編集のキー (Ctrl+C・Ctrl+V・Ctrl+X・Ctrl+Z・Ctrl+A) と右クリックの切り取り・コピー・貼り付けがそのまま使える
- [ ] #4 Ctrl+F の検索バーと右クリックメニューのそれ以外の項目を残すか消すかを決め、理由を Implementation Notes に残している
- [ ] #5 macOS でも同じ操作 (⌘R・⌘P) で再読み込みや印刷が起きないことを確かめている
<!-- AC:END -->
