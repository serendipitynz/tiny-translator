---
id: TASK-8
title: 翻訳エンジンの起動・終了・自動解放を作る
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-07 04:19'
labels: []
milestone: m-2
dependencies:
  - TASK-1
  - TASK-4
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
メモリを抑えるために翻訳エンジンを子プロセスにし、使わないときは終了する (doc-1 §4.1, §4.5, §5)。子プロセスが残るとメモリを圧迫し続けるので、異常終了の後始末まで含める。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 llama-server を Tauri の sidecar として同梱し、127.0.0.1 の空きポートで起動する
- [x] #2 アプリ起動直後に翻訳エンジンを起動し、読み込み完了を待てる
- [x] #3 自動解放までの時間 (既定 10 分、0 は解放しない) が過ぎると翻訳エンジンを終了し、次の翻訳で再起動する
- [x] #4 アプリ終了時に翻訳エンジンが終了し、前回の異常終了で残ったプロセスを起動時に終了する
- [x] #5 翻訳エンジンの状態 (停止中・読み込み中・待機中) を画面へ通知できる
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. llama.cpp b12110 をソースから静的にビルドする scripts/build-llama-server.sh を作り (BUILD_SHARED_LIBS=OFF, GGML_METAL_EMBED_LIBRARY=ON, LLAMA_CURL=OFF)、src-tauri/binaries/llama-server-<target triple> へ置く (gitignore)。tauri.conf.json の bundle.externalBin に登録する。プリビルド版は dylib を 10 個以上 @loader_path で読むため、externalBin の 1 ファイル同梱に合わない (オーナー承認 2026-11-07)
2. 本番依存は画面側の @tauri-apps/api ^2 だけ (オーナー承認 2026-11-07)。Rust 側はプロセス・/health・空きポートを std で書き、通知は文字列にして serde を入れない
3. src-tauri/src/engine.rs: 翻訳エンジンの状態 (stopped / loading / ready)、起動 (空きポート、-m <gguf> -c 8192 --parallel 1 --host 127.0.0.1 --port N --cache-ram 0 --no-webui、ログはアプリのログディレクトリ)、/health 待ち、acquire / lease による使用中の管理と自動解放 (既定 10 分、0 は解放しない)、終了
4. モデルは <アプリデータディレクトリ>/models/ の GGUF。2B Q4_K_M があればそれ、無ければ名前順で最初の .gguf
5. PID ファイルで前回の異常終了で残った llama-server を起動時に終了する (プロセス名を確かめてから)。アプリ終了 (RunEvent::Exit) で終了する
6. 状態をイベント engine-state とコマンド engine_state で画面へ通知し、src/lib/engine.ts で受ける
7. Rust のテスト (偽の子プロセスと /health に答えるスレッド) と、実機 (pnpm tauri dev + ps / lsof) で確かめる。doc-1 §4.5 と §10 に引数とモデルの置き場所を追記し、README にビルド手順を書く

8. 読み込み中にすぐ終了した llama-server は、新しい空きポートで起動し直す (1 回の起動につき 3 回まで)。手放した直後のポートを macOS が短い間使用中のままにし、bind が失敗することをテストで観測したため
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 判断
- sidecar は llama.cpp b12110 (commit cd959bae9、TASK-1 と同じ) をソースから静的にビルドした (オーナー承認 2026-11-07)。リリース版の llama-server は 50 KB の本体が @loader_path で dylib を 10 個以上読み込み、externalBin の 1 ファイルに収まらない。最初のビルドは Homebrew の OpenSSL (/opt/homebrew/opt/openssl@3) をリンクしていたので LLAMA_OPENSSL=OFF を足し、OS の外のライブラリをリンクしたらスクリプトが失敗するようにした。バイナリは 23 MB。
- 本番依存は @tauri-apps/api ^2 だけ (オーナー承認 2026-11-07)。Rust 側は std だけで書き、画面への通知は [状態, エラー] の組にして serde を入れなかった。
- 起動引数は -c 8192 --parallel 1 --host 127.0.0.1 --port <空きポート> --cache-ram 0 --no-webui (オーナー承認 2026-11-07)。doc-1 §4.5 に「アプリが使う引数」として記録した。
- モデルは <アプリデータディレクトリ>/models/ に置く。2B Q4_K_M があればそれ、無ければ名前順で最初の GGUF (doc-1 §10)。
- 読み込み中にすぐ終了した llama-server は、新しい空きポートで 3 回まで起動し直す。テストを繰り返すと 10% 前後の回で、空きポートを手放した直後の bind が EADDRINUSE になった。10 ms 待てば通るので、macOS が手放した直後のポートを短い間使用中のままにしていると見ている。lsof や netstat では保持者が見えなかった。アプリの起動時にも同じ失敗が起きうるため、リトライは本体に入れた。リトライ中は読み込み中のままにする。
- acquire / Lease (TASK-9 が使う) と set_idle_release (TASK-21 が使う) は呼び出し元がまだ無い。cfg_attr(not(test), expect(dead_code)) を付けたので、使い始めると expect が外れていないことを clippy が知らせる。

## 確認 (macOS 26.7.1, MacBook Air M2)
- AC#1: pnpm tauri dev で target/debug/llama-server (externalBin のコピー) が起動し、lsof で 127.0.0.1:49328 だけを LISTEN していた。pnpm tauri build --bundles app の .app (30.5 MiB) にも Contents/MacOS/llama-server が入り、バンドルから起動したアプリでも 127.0.0.1 で LISTEN した。静的ビルドで PFML3 の英日翻訳を /completion に送り、訳文が返った (eos で停止、42.6 tok/s)。
- AC#2: 起動直後に画面が loading → ready を受け取った (一時的に入れた、画面から Rust へ状態を送り返す仕組みで確認。コミットには含めていない)。読み込み時間は 7.6 秒と 5.2 秒で、TASK-1 の 1.5 秒より長かった。計測時はスワップを 13 GB 使っており、リリース版のバイナリと静的ビルドを交互に起動すると、どちらも 3〜30 秒の幅でばらついた (同じ回で 2.88 秒と 3.34 秒、30.17 秒と 30.47 秒)。静的ビルドが遅いわけではないと判断した。
- AC#3: 既定の 10 分で、ready (15:19:57) から 15:30:02 までに stopped になり、プロセスと engine.pid が消えた。解放後の acquire による再起動は Rust のテスト (偽の子プロセス) で確かめた。翻訳からの再起動は TASK-9 で確かめる。0 (解放しない) もテストで確かめた。
- AC#4: アプリのメニューの Quit で RunEvent::Exit が来て、翻訳エンジンと engine.pid が消えた (dev と .app の両方)。kill -9 でアプリを止めると翻訳エンジンが残り、再起動したアプリがそれを終了した (2 回。2 回目は tauri dev の再ビルドがアプリを強制終了した場合)。PID が別のプログラムのものなら終了しないこともテストで確かめた。
- AC#5: イベント engine-state とコマンド engine_state で [状態, エラー] を送る。画面側の watchEngine は vitest で確かめた (取り逃がしが無いよう、listen が済んでから invoke する)。表示は TASK-21 で作る。
- cargo fmt --check / clippy -D warnings / cargo test (13 件。60 回続けて通した)、pnpm check / lint / test (17 件) が通った。

## 見ていないこと・気づいたこと
- ウィンドウを閉じたときにアプリが終了するか (RunEvent::Exit が来るか) は確かめられなかった。dev 版のウィンドウは System Events から見えず、閉じる操作を送れなかった。オーナーに手で確かめてほしい。
- System Events からアプリを前面にできていなかったため、確認の途中で送った Cmd+W 2 回と Cmd+Q 1 回が、そのとき前面にあった別のアプリへ届いた可能性がある。
- llama-server のログに「API キーが無く、CORS がすべてのオリジンを許可している」という警告が出る。127.0.0.1 でだけ待ち受けているが、ブラウザで開いたページがポートを探り当てれば、翻訳エンジンを使えてしまう。--api-key に起動ごとの乱数を渡す対策は、引数の追加になるので、入れるかどうかはオーナーの判断を待つ。

レビュー (PR #8、Codex 2 回) で直したこと:
- [P2] 監視スレッドが読み込み中の終了を先に見つけると、起動し直しを使わずに停止していた。監視スレッドが終了を扱うのは待機中 (Ready) のときだけにし、読み込み中の終了は読み込みスレッドの起動し直しに任せた (7887201)。この順序を強制するテストを足し、修正前の条件で失敗することを確かめた (Rust のテストは 15 件)。
- llama.cpp の取得先を src-tauri/target/ から ~/.cache/tiny-translator/ へ移した。リポジトリ内に置くと svelte-check が llama.cpp 同梱の UI を読み込むため (b788b55)。
- [P3] 2 つ目に起動したアプリが 1 つ目のアプリの翻訳エンジンを終了させてしまう。単一インスタンスは TASK-11 の AC#2 で扱う。その際は start_engine / kill_leftover より前で二重起動を判定する必要がある。
- 起動ごとの API キー (--api-key) を付ける対策は、オーナーの判断で TASK-9 で入れる。
<!-- SECTION:NOTES:END -->
