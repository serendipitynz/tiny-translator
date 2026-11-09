---
id: TASK-22
title: 履歴と設定を Windows で確かめる
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-09T19:00:00Z'
labels: []
milestone: m-4
dependencies:
  - TASK-20
  - TASK-21
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
保存先のディレクトリと SQLite の扱いが Windows でも同じに動くかを確かめる。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Windows で翻訳履歴の保存・検索・一覧と設定の保存が動く
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Windows で pnpm check / lint / test と cargo fmt --check / clippy --all-targets / test を通す (rusqlite の bundled SQLite が MSVC でビルドでき、history.rs・settings.rs の単体テストが NTFS で通ること)
2. 実アプリ (pnpm tauri dev) を WebView2 のリモートデバッグで操作し、実機のエンジン (2B) で英日・日英を訳して %APPDATA%\com.serendipitynz.tiny-translator\history.sqlite3 に保存されること、停止した翻訳が保存されないことを確かめる
3. 履歴の一覧で、英語の語・3 文字以上の日本語・1〜2 文字の日本語の検索、1 件を選んで画面へ戻す操作、1 件削除、全件削除を確かめる。SQLite のファイルを別に開いて件数と FTS の索引を照合する
4. 設定画面で全項目を変えて保存し、アプリを終了・再起動して値が残ること (settings.json と WebView2 の localStorage の配色)、選んだモデル (0.8B) で翻訳エンジンが起動すること、メモリ使用量の目安 (ワーキングセット) が出ること、モデルを変えた保存でエンジンが読み直されることを確かめる
5. 読めない settings.json で起動して settings.unreadable-<ms>.json へ退避されることを Windows で確かめる
6. 結果を Implementation Notes に書き、doc-1 の §8.5・§9 に Windows での確認を TASK-18 (§7.1) と同じ形で足す
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 判断
- 確かめ方は TASK-18 と同じく、実アプリ (pnpm tauri dev) を WebView2 のリモートデバッグ (WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222) につなぎ、画面のボタン・選択欄・入力欄を操作して値を読んだ。保存先のファイルは別に開いて照合した (history.sqlite3 は Python の sqlite3 で読み取り専用に開いた)。
- コードは変えていない。Windows で食い違う動作は見つからなかった。
- 確認のあと、アプリデータディレクトリを確認前の状態へ戻した: 履歴は全件削除 (history.sqlite3 は残す)、settings.json と退避した settings.unreadable-*.json は削除、localStorage の配色は消した。

## 確認 (この PC: Windows 11 Home 10.0.26200、Ryzen 7 5800X、RTX 5060 Ti 16 GB、Vulkan 版 llama-server b12110)
- pnpm check (0 件)・pnpm lint・pnpm test (88 件)・cargo fmt --check・cargo clippy --all-targets -- -D warnings・cargo test (183 件。history 13 件・settings 12 件を含む) が Windows で通った。rusqlite の bundled SQLite は MSVC でビルドできた。macOS の 204 件との差は engine.rs の cfg(unix) のテスト。
- 履歴の保存: %APPDATA%\com.serendipitynz.tiny-translator\history.sqlite3 が作られ (user_version=1)、起動時のクリップボード翻訳 1 件と英日・日英の 4 件、長文 1 件がすべて保存された。判定なしの翻訳元は NULL、モデル名はファイル名だけ (plamo-3-translate-2b-Q4_K_M.gguf)。長文 (約 8,700 文字) を出力 3 文字の時点で停止すると件数は増えなかった。表と FTS の索引 (translations_fts_docsize) の件数は常に一致した。
- 検索: fox・FOX (大小無視)、新幹線 (3 文字、FTS)、猫 (1 文字)・名前 (2 文字) (LIKE)、restart configuration (AND)、キツネ 川岸 (訳文の語の AND) がそれぞれ該当の 1 件だけを返し、zzz は 0 件、検索語を消すと全件に戻った。
- 一覧から 1 件を選ぶと原文・訳文と翻訳元 ja・翻訳先 en が戻り、形式は自動のまま、モーダルは閉じた。1 件削除で 6→5 件、全件削除で 0 件になり空の語が出た。削除後の sqlite_sequence は 6 で、次の保存は id 7 (再利用しない)。履歴はアプリを起動し直しても残った。
- 設定の保存: 全項目 (第一 en・第二 ja、0.8B、自動解放 1 分、起動時のクリップボード翻訳 無効、翻訳しないキー "  id / url / 空行 / id" → [id, url]、Solarized Dark) を変えて保存すると、settings.json (version 1) に書かれ、配色は localStorage に入り即座に描かれ、llama-server が 0.8B で起動し直した (コマンドラインで確認)。優先順位は、日本語 → en、英語 → ja、判定なし → en になった。最後の翻訳から約 1 分でエンジンが終了し、状態は stopped になった。
- 再起動後: 終了 (taskkill で WM_CLOSE) して起動し直すと、設定画面の全項目と配色 (data-color-family=solarized・data-color-mode=dark) が残り、エンジンは 0.8B で起動し、クリップボードに置いた英文は訳されなかった (原文は空、件数は変わらず)。
- 保存の上書き: settings.json がある状態で 2 回続けて保存しても (自動解放 5 分 → 15 分) 成功し、一時ファイルは残らなかった (Windows の fs::rename は既存ファイルを置き換える)。
- 読めない設定ファイル: 壊れた JSON の settings.json で起動すると settings.unreadable-<ms>.json へ内容そのまま移り、settings.json は作られず、既定値 (2B) で起動し、設定画面に退避先のパスを含む案内が出た。
- メモリ使用量の目安: engine_memory の値 2,153,181,184 バイト (画面では「約 2.0 GB」) が、Get-Process の WorkingSet64 (2053.4 MiB) と一致した。

## 見ていないこと
- 削除後・一覧から選んだ後のフォーカスの移り先。自動操作中はウィンドウにフォーカスが無く、document.activeElement が取れなかった (動きは OS によらず、TASK-20 で確かめている)。
- 配色が最初の描画より前に決まること (ちらつきの有無) は目で見ていない。再起動直後に data-color-* が保存値になっていることだけを確かめた。
- ROG Ally (Z1) での確認、IME の変換中の検索欄の動き。
- 気づいたこと (このタスクの範囲外): "The quick brown fox jumps over the lazy dog near the riverbank." は判定なし (§6.1 の閾値 0.5 を下回る) になった。判定は OS によらない。
<!-- SECTION:NOTES:END -->
