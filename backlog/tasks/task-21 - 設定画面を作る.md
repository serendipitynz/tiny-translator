---
id: TASK-21
title: 設定画面を作る
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-09 11:13'
labels: []
milestone: m-4
dependencies:
  - TASK-10
  - TASK-5
references:
  - ../snz-design/backlog/docs/doc-9 - 複合UI部品の共通仕様と擬似コード.md
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
ここまでのフェーズで既定値にした項目を利用者が変えられるようにする。設定は再起動後も残る必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 第一優先言語と第二優先言語、自動解放までの時間、使うモデル (所定のディレクトリの GGUF から選ぶ)、起動時のクリップボード翻訳の有効・無効、配色の 2 軸を設定できる
- [x] #2 設定画面を snz-design doc-9 §6.6 のモーダルと §7 の 2 軸の設定画面の実例に従って描く
- [x] #3 翻訳エンジンの状態とメモリ使用量の目安を表示する
- [x] #4 設定が再起動後も保たれ、読めない設定ファイルは上書きせず退避する
- [x] #5 JSON の翻訳しないキー (doc-1 §7.1、TASK-15 で translate が受け取る一覧) を設定できる
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 保存先: 配色の 2 軸は doc-1 §8.3 のとおり localStorage (index.html の head スクリプトが読む)。ほかはアプリデータディレクトリの settings.json を Rust が持つ (翻訳エンジン・起動時のクリップボード翻訳が画面より先に読むため)。version を持ち、版が新しすぎるファイルには書かない。
2. 読めない settings.json (JSON でない・オブジェクトでない・読み取りエラー) は起動時に settings.unreadable-<日時>.json へ改名して退避し、既定値で起動する (2026-11-09 オーナー判断)。改名できないときと版が新しすぎるときは書かずに、その起動の間だけ効かせる。項目ごとの不正な値はその項目だけ既定値で読み、書き換えない (snz-design doc-7 §6.1)。
3. 翻訳エンジン: 自動解放までの時間を set_idle_release で反映。モデルは models/ の GGUF から選び、保存したらすぐ読み直す。翻訳中なら終わってから読み直す (2026-11-09 オーナー判断)。選んだ GGUF が無いときは既定の選び方へ戻し、設定画面で伝える。状態とメモリ使用量の目安 (macOS は RSS、Windows はワーキングセット) を返すコマンドを足す。
4. 画面: ヘッダーの設定ボタンで開く作業モーダル (doc-9 §6.6)。下書きを編集して [キャンセル][保存] で確定 (2026-11-09 オーナー判断)。保存の失敗は入力を残して告知、未保存のまま閉じると破棄前確認。外観はパネルに配色系統と明暗の 2 つの選択欄を置き、doc-7 §5.1〜§5.3 の縮退を告知の段で伝える (doc-9 §7)。チェックボックスと告知の部品を足す。
5. App は起動時に設定を読み、優先言語と翻訳しないキーを翻訳に渡す。起動時のクリップボード翻訳の無効は Rust 側の launch_clipboard で判定する。
6. テスト (cargo test / vitest) と lint を通し、実アプリで保存・再起動・壊れたファイルの退避を確かめる。doc-1 の TASK-21 待ちの記述を更新する。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
保存の方式・退避の時点・モデル変更時の扱いは 3 つとも 2026-11-09 オーナー判断 (doc-1 §8.5 に記録)。

- 配色の 2 軸は localStorage、それ以外は settings.json (Rust が起動時に読む)。翻訳エンジンと起動時のクリップボード翻訳が画面より先に設定を要るため、1 つのファイルにまとめず分けた。
- 配色を保存した後も OS 追従が正しく効くよう、index.html の head スクリプトが window.tinyColorScheme.choose を出し、画面はそれで反映する (スクリプトの閉包の mode を更新しないと、OS の切り替えで古い選択が描かれるため)。
- 告知 (doc-9 §6.4 の失敗・進行中の縮退・案内) とチェックボックス (doc-8 §6.4) の部品を足した。2 系統とも明暗の両側を持つので、選択欄の組の「選択肢が減る」挙動と説明域は出ない。
- 共通の Modal.svelte で、面の max-block-size が効かずに操作域が画面の外へ出る不具合を直した (grid の行が内容に合わせて伸びていた)。履歴のモーダルも件数が多いと同じだった。
- 翻訳しないキーは前後の空白を落とす。キー名の前後に空白を持つ JSON より、入力の打ち間違いのほうが多いと判断した。
- 本番依存は増やしていない。Windows のメモリ読み取りは kernel32 の K32GetProcessMemoryInfo を手で宣言した (既存の image_path と同じ形)。

確認したこと:
- pnpm check / lint / test (85 件。settings.test.ts・appearance.test.ts と color-scheme.test.ts の追加分を含む)、cargo fmt --check / clippy -D warnings / cargo test (204 件。settings.rs の退避・新しい版・項目ごとの既定値、engine.rs の reload と memory を含む) が通る。
- Windows 向けのメモリ読み取り関数だけを切り出して cargo check --target x86_64-pc-windows-msvc が通る (クレート全体は SQLite の C コードのため macOS から検査できない)。
- 内蔵ブラウザで Vite の開発サーバーを開き IPC をモックして操作: 全項目の変更と保存 (settings_save の引数、Solarized Dark の即時反映と localStorage、通知)、開き直して値が残ること、同じ優先言語で保存が理由付きで無効になること、Escape で破棄前確認 (焦点は「編集に戻る」、Escape は確認だけを閉じる)、保存失敗で入力を残し失敗の告知、退避・新しい版・GGUF なし・エンジンの起動失敗の告知、保存後の翻訳が en 優先で target ja・翻訳しないキー [id, url] を送ること、幅 480px で横スクロールなし。
- 実アプリ (pnpm tauri dev): 壊れた settings.json で起動すると settings.unreadable-<ms>.json へ内容そのまま移り、settings.json は作られず既定の 2B で起動した。保存済みの設定 (0.8B・自動解放 1 分・起動時のクリップボード翻訳 無効) で起動すると、llama-server が 0.8B で起動し、読み込みの約 1 分後に終了し、翻訳履歴は増えなかった。

確かめていないこと (オーナーの確認待ち):
- 実窓 (WKWebView) での設定画面の操作 (チェックボックスの描画、保存してからモデルを読み直す流れ、メモリ使用量の目安の表示)。
- Windows での実行 (ワーキングセットの読み取りを含む)。m-4 の Windows 確認のタスクで行う。

レビュー (PR #21、Codex) で直したこと:
- [P1] モデルを変えて保存すると、設定のロックを持ったまま翻訳エンジンを読み直し、spawn が同じロックで設定を読むため保存が返らなかった。ロックを手放してから読み直すようにし、spawn が設定を読む形の回帰テストを足した (読み直しをロックの内側に戻すと失敗することを確認)。
- [P2] 配色を localStorage に書けなかった後に保存し直すと、描いた配色と同じなので書き込みを飛ばし、成功と報告していた。書けなかった配色は次の保存で書き直す。
<!-- SECTION:NOTES:END -->
