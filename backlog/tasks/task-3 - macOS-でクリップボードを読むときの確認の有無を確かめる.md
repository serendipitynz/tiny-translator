---
id: TASK-3
title: macOS でクリップボードを読むときの確認の有無を確かめる
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-06 04:23'
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
- [x] #1 現行の macOS (バージョンを記録) で、Tauri の clipboard-manager プラグインがテキストを読むときに確認が出るかどうかを記録している
- [x] #2 確認が出る場合、許可を保存できるか、出さない読み方があるかを調べ、doc-1 §6.3 へ反映している
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. scripts/clipboard_probe に Tauri 2 + tauri-plugin-clipboard-manager の最小アプリを作る。起動直後 (setup) に Rust 側の clipboard().read_text() でテキストを読み、結果と NSPasteboard.accessBehavior をログファイルと窓に出す。フロントエンドはビルド不要の静的 HTML にする。
2. .app にバンドルし (bundle identifier 付き)、pbcopy で別プロセスからテキストを置いてから起動し、確認が出るかを画面の撮影とログで確かめる。既定の状態と、EnablePasteboardPrivacyDeveloperPreview を有効にした状態の両方で試す。
3. 確認が出る状態で、システム設定の「ほかのアプリからのペースト」で許可を保存できるか、確認を出さない読み方 (利用者のペースト操作に続く読み取り、detect 系 API) があるかを調べる。
4. 結果を doc-1 §6.3 と §11 へ反映する。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 確認 (AC#1)
- macOS 26.7.1 (25G241、MacBook Air M2) で確かめた。scripts/clipboard_probe (Tauri 2 + tauri-plugin-clipboard-manager 2.5.0 の最小アプリ) を .app にまとめ、pbcopy で別プロセスから置いたテキストを setup の read_text で読んだ。結果と NSPasteboard.accessBehavior を ~/Library/Logs/<identifier>/probe.log に残し、確認の有無は画面の撮影で見た。
- 既定の状態では確認は出ず、accessBehavior は AlwaysAllow。
- 開発者プレビュー (defaults write <identifier> EnablePasteboardPrivacyDeveloperPreview -bool yes) を有効にすると確認「"clipboard_probe" is trying to access the pasteboard.」(Allow Paste / Don't Allow Paste) が出た。応答するまで read_text は返らず、setup が約 24 秒止まった。accessBehavior は Default。

## 許可の保存と確認を出さない読み方 (AC#2)
- 「Allow Paste」は 1 回だけの許可で、accessBehavior は Ask になり、再起動するとまた確認が出た。
- システム設定の「ほかのアプリからのペースト」に clipboard_probe.app が Ask で並び、「許可」にすると再起動後は確認なしで読めた (AlwaysAllow、36 ms)。確認の操作と設定の変更はオーナーが行った。
- JS コマンド (plugin:clipboard-manager|read_text) をボタンから呼んでも確認が出た。Cmd+V の貼り付けは AlwaysAllow では確認なしで読めた。Ask では確認は出なかったが、Edit メニューの Paste の処理の中でアプリが応答しなくなった (sample で NSMenu performKeyEquivalent のメニュー追跡のループにいた)。1 回だけの観測。
- 方針は「accessBehavior が AlwaysAllow のときだけ起動時に読み、それ以外は読まずに許可の方法と Cmd+V を案内する」にした (2026-11-06 オーナー判断)。確認が出ても読む案は、Allow Paste が保存されず起動のたびに操作が増えるので採らなかった。doc-1 §6.3 に方針と実測を、§11 に確認済みを書いた。
- 原文欄の [貼り付け] ボタンも確認が出る読み方になる。扱いは TASK-11 へ送った (doc-1 §6.3)。

## 測っていないもの
- detect 系の API (macOS 15.4) は Apple の説明を引いただけで試していない。
- Ask の状態での Cmd+V が毎回止まるか、Don't Allow Paste・AlwaysDeny のときに read_text が何を返すかは確かめていない。
- 確認アプリの「ほかのアプリからのペースト」の許可はこの Mac に残っている。開発者プレビューの設定は消した。
<!-- SECTION:NOTES:END -->
