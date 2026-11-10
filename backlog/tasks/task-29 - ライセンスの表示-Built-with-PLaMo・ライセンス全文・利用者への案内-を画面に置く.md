---
id: TASK-29
title: ライセンスの表示 (Built with PLaMo・ライセンス全文・利用者への案内) を画面に置く
status: Done
assignee: []
created_date: '2026-11-09 20:20'
updated_date: '2026-11-10 22:30'
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
- [x] #1 画面の常に辿れる場所に「Built with PLaMo」を表示し、README にも書いている (doc-1 §10.3 の D1)
- [x] #2 アプリの中から PLaMo Community License の全文 (日本語版と英語版) と改定日を開ける。全文は GGUF リポジトリの LICENSE/ja・LICENSE/en を書き写さずにそのまま使っている (D2)
- [x] #3 全文の前に、利用者が負う条件の短い案内 (D5 の 4 項目) を置いている
- [x] #4 macOS と Windows で、表示と全文を開く操作を確かめている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. ModelFetchModal にある条文の表示 (言語の選択・全文の領域・改定日) を LicenceText.svelte へ切り出し、入手のモーダルと新しいライセンスのモーダルで共有する。全文は既存の src/licenses/ (GGUF リポジトリの LICENSE/ja・en をそのまま写したもの) を使う。
2. 設定画面の末尾に「ライセンス」の区画を置き、「Built with PLaMo」と「PLaMo Community License を表示」のボタンを出す (D1)。設定を読めないときも出るよう、外観と同じく設定の読み込みに依らない位置に置く。
3. ボタンで開く LicenceModal に、D5 の 4 項目 (業務での利用の登録、長文の公表物での明示、第 5 条 (4) の表示、重要な決定での制限) の短い案内を先に置き、その後に全文と改定日を出す (D2・D5)。案内の文言は models.ts に置いてテストする。
4. README に Built with PLaMo とライセンスの案内を書き、doc-1 §8.5・§10.3 に作った形を記録する。
5. pnpm check / lint / test と cargo のテストを通し、macOS の実機 (pnpm tauri dev) で表示と全文を開く操作を確かめる。Windows の確認 (AC#4 の半分) はオーナーに依頼する。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 置き場所は doc-1 §10.3 の例どおり設定画面の末尾 (区画「ライセンス」)。ヘッダーの設定ボタンから常に辿れる。区画は外観と同じく設定の読み込みに依らない位置に置き、settings.json を読めないときも出るようにした (Vite の開発サーバーで、設定の読み込みに失敗した状態でも区画とボタンが出ることを確かめた)。
- ヘッダーに常設のリンクや「About」画面を足すことはしなかった。D1 の条文 (第 4 条 (1)(ii)) は UI のどこかへの明示を求めるだけで、設定画面の末尾で足りると doc-1 が既に置き場所を示していたため。
- 全文の表示 (言語の切り替え・全文の領域・改定日) は入手のモーダル (D3) と同じものを使うので LicenceText.svelte へ切り出し、両方のモーダルから使う。全文は TASK-24 で置いた src/licenses/ (GGUF リポジトリの LICENSE/ja・en をそのまま写し、テストが SHA-256 を照らしているもの) のまま。改定日の注記の文言は「入手するモデルと同じコミット」から「入手できるモデルと同じコミット」に変え、両方の画面で通じるようにした。
- D5 の 4 項目は条文の該当箇所 (第 3 条 (3)・第 1 条 (10)、第 5 条 (2)、第 5 条 (4)、第 7 条 (2)) を読み直して書き、要約であって正確な内容は全文によると添えた。
- 確かめたこと: pnpm check (0 errors)、pnpm lint、pnpm test (103 件) が通る。Vite の開発サーバー (Chromium) で、設定 → ライセンスを表示 → 案内が全文より前に出る、日本語・英語の切り替えで全文が変わり末尾が改定日 (2026 年 11 月 9 日改定 / Revised on November 9, 2026) になる、全文の領域の名前が見出し「PLaMo Community License」になる、Escape で閉じるとフォーカスが「ライセンスを表示」に戻る、を確かめた。Rust 側は変えていないので cargo は走らせていない。
- 確かめていないこと (AC#4): Tauri の実機 (macOS の WKWebView と Windows の WebView2) での表示と操作。オーナーの確認を待つ。

- AC#4: オーナーが macOS と Windows の実機で、ライセンスの区画の表示と全文を開く操作を確かめた (2026-11-11)。
<!-- SECTION:NOTES:END -->
