---
id: TASK-7
title: アプリの骨格を Windows で確かめる
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-07 00:50'
labels: []
milestone: m-1
dependencies:
  - TASK-6
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Windows は副対象で、各マイルストーンの最後に確かめる (doc-1 §12)。WebView2 での描画の差をここで拾う。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Windows (x86_64) で pnpm tauri dev が動き、メイン画面が 4 配色で描ける
- [x] #2 macOS との描画の差を記録し、直したか、直さない理由を残している
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. この Mac から使える Windows (x86_64) が無いので、実機での起動と 4 配色の確認はオーナーが手元の Windows で行う (2026-11-07 オーナー判断)。AI は手順と確認表を用意し、結果を受けて直す・記録する。
2. Chromium エンジンとしての差 (`:has()`・コンテナクエリ・格子・フォーカス枠) は TASK-6 が Chromium で確かめ済みなので、確認表は Windows と WebView2 に固有の差に絞る: 文字の書体 (system-ui が Yu Gothic UI / Segoe UI に解決される)、常に幅を取るスクロールバーとその配色、選択欄の開いた一覧の配色、タイトルバーの配色、拡大率 (125%・150%)、WebView2 自身のキー操作と右クリックメニュー。
3. README の前提に Windows で要るもの (MSVC の Rust・Visual Studio Build Tools の C++・WebView2) を足す。
4. オーナーの報告を受け、直すものは直して再確認を頼み、直さないものは理由を Implementation Notes に残す。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 確かめた環境と方法

- この Mac (arm64) から使える Windows (x86_64) が無いので、オーナーが手元の Windows 11 (x86_64) で `pnpm tauri dev` を動かし、スクリーンショットと所見を返した。AI はスクリーンショットを読み、差を整理した (2026-11-07)。
- 配色は DevTools のコンソールで `colorFamily`・`colorMode` を書き換えて切り替えた。設定画面 (TASK-21) はまだ無い。
- Chromium エンジンとしての差 (`:has()`・コンテナクエリ・格子・フォーカス枠) は TASK-6 が Chromium で 4 配色を確かめ済みなので、Windows と WebView2 に固有の差に絞って見た。

## AC#1

- 最初は Rust のビルドで止まった: tauri-build は Windows で `src-tauri/icons/icon.ico` から Windows のリソースを作るが、仮のアイコンは 32×32 の `icon.png` しか無かった (macOS のビルドは `.ico` を求めないので表に出ていなかった)。同じ PNG をそのまま包んだ `icon.ico` を足し、`bundle.icon` にも並べた。本物のアイコンは m-5 で作る。
- 直した後、Windows で `pnpm tauri dev` が起動し、標準 Light・標準 Dark・Solarized Light・Solarized Dark の 4 配色でメイン画面が描けた。幅 480 付近で原文と訳文が上下に並ぶことも見た。

## AC#2 (macOS との差と扱い)

| 差 | 扱い |
|---|---|
| Windows でだけ `icon.ico` が無くビルドが止まる | 直した (上記) |
| 書体が `system-ui` から Yu Gothic UI / Segoe UI に解決される | 4 配色で読みにくさは無く、直さない |
| 選択欄の開いた一覧 (WebView2 が描く) | 4 配色とも読める。`option` の色は指定しないまま |
| スクロールバーが常に幅を取り、矢印付きで描かれる。Solarized では配色の外の灰色 | 崩れもはみ出しも無く、読める。snz-design にスクロールバーのトークンが無いので、色を足すならそちらで決めることとし直さない |
| タイトルバーがアプリの配色でなく OS の明暗に従う (アプリを Dark、OS を Light にすると明るいタイトルバー) | macOS でも同じで Windows 固有ではない。配色を明示的に選べるのは設定画面 (TASK-21) からなので、そこでウィンドウのテーマを合わせるかを決める |
| F5・Ctrl+R で再読み込み、Ctrl+P で印刷プレビュー、Ctrl+F で検索バー、右クリックメニューが出る | 描画の差ではなく、今は入力しか状態が無いので直さない。翻訳ができると F5 で原文と訳文が消えるので、m-2 の TASK-27 として起票した (Tauri 2.12 は WebView2 の AreBrowserAcceleratorKeysEnabled を設定から変えられない) |
| Ctrl++ でページが拡大されない | Tauri の既定 (`zoomHotkeysEnabled` が false) で macOS と同じ |
| 画面の拡大率 150% | 標準 Light で、フォーカス枠・罫線・アイコン・無効の破線にぼやけや欠けは無かった |

## 確かめていないこと

- Windows の高コントラスト (forced-colors) と、125% など 150% 以外の拡大率。
- Windows での `pnpm check`・`pnpm lint`・`pnpm test`・`cargo clippy` (macOS では通る)。
<!-- SECTION:NOTES:END -->
