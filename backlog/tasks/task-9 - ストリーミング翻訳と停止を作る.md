---
id: TASK-9
title: ストリーミング翻訳と停止を作る
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-07 05:25'
labels: []
milestone: m-2
dependencies:
  - TASK-8
  - TASK-6
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
原文を翻訳エンジンへ渡し、訳文を生成されるそばから訳文欄へ出す (doc-1 §4.3)。小さいモデルでも長い訳文は時間がかかるので、途中で止められる必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 翻訳ボタンと Ctrl/Cmd+Enter で、PFML3 のプロンプトを /completion に渡し、訳文がストリーミングで訳文欄に出る
- [ ] #2 翻訳中は翻訳ボタンが処理中の表示になり、停止できる
- [x] #3 翻訳エンジンが停止中なら起動を待ってから翻訳する
- [x] #4 プロンプトの組み立てに単体テストがある
- [x] #5 翻訳エンジンを起動のたびに作る乱数の API キー (--api-key) 付きで起動し、翻訳の要求にそのキーを付ける。キーの無い要求 (ブラウザのページからの要求など) は受け付けない
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 翻訳の要求は Rust 側が出す (src-tauri/src/translate.rs)。画面は Tauri のコマンド translate / stop_translation と Channel だけを使い、ポートと API キーは WebView へ出さない。CSP の connect-src も ipc のままにする。
2. PFML3 のプロンプトを組み立てる関数 (翻訳先言語は必須、翻訳元言語は任意) と、その単体テスト (AC#4)。
3. /completion へ stream: true・temperature 0 (TASK-1 と同じ)・stop [<|plamo:tag|>, <|plamo:bos|>] で POST する。HTTP は health() と同じく TcpStream で手書きし、chunked の本文を解いて SSE の data: 行を serde_json で読み、content を Channel で順に送る。
4. 翻訳中は Engine::acquire() の Lease を持つ。停止中なら起動を待つ (AC#3)、翻訳中は自動解放されない。
5. 停止: 実行中の翻訳の取り消し印と TcpStream の複製を持ち、stop_translation で接続を閉じる。llama-server は切断で生成を取り消す (実測: cancel task の直後に次の要求が 0.26 秒で返った)。新しい翻訳は前の翻訳を止めてから始める。
6. API キー: 起動 (spawn) のたびに getrandom で 32 バイトを作り、16 進にして LLAMA_API_KEY (--api-key と同じ設定の環境変数版) で渡す。Lease がキーを持ち、要求に Authorization: Bearer を付ける。キーの無い要求は 401 になる (実測)。/health はキー無しで 200 のまま。
7. 画面: 翻訳ボタンと Ctrl/Cmd+Enter (IME 変換中は除く) で翻訳、訳文欄を空にしてから埋める。翻訳中は翻訳ボタンを処理中にし、隣の停止ボタンで止める (翻訳していないときは理由付きで無効)。翻訳先は選択値、自動のときは仮に日本語 (TASK-10 で優先順位に置き換える)。
8. 本番依存: serde_json と getrandom 0.3 を Cargo.toml に直接足す (どちらも tauri 経由ですでにビルドに入っている同じ版で、新しいクレートは増えない)。オーナーの承認後に追加する。

9. オーナー判断 (2026-11-07): serde_json と getrandom 0.3 の追加を承認。停止ボタンは翻訳ボタンの左に常に置き、翻訳していないときは理由付きで無効にする。翻訳の失敗の理由は翻訳ボタンの行に一行で出し、読み上げにも届ける。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装の判断

- 翻訳の要求は Rust 側 (src-tauri/src/translate.rs) が出し、画面は Tauri のコマンド translate / stop_translation と Channel だけを使う。翻訳エンジンのポートと API キーを WebView に渡さず、CSP の connect-src を ipc のままにするため。
- HTTP は engine.rs の health() と同じく TcpStream で手書きした (chunked の本文と SSE の data: 行を読む)。127.0.0.1 への POST 1 本のために HTTP クライアントのクレートを足さないため。JSON は serde_json で組み立て・読み取りする。
- 追加した本番依存は serde_json 1 と getrandom 0.3 (オーナー承認、2026-11-07)。どちらも tauri 経由ですでにビルドに入っていた版で、Cargo.lock に新しいパッケージは増えていない。
- API キーは起動 (spawn) のたびに getrandom で 32 バイトを作って 16 進にし、--api-key と同じ設定を環境変数 LLAMA_API_KEY で渡す。引数は同じ機械の他の利用者も ps で読めるが、環境変数は同じ利用者にしか読めないため。Lease がキーを持ち、要求に Authorization: Bearer を付ける。
- 翻訳は temperature 0 (TASK-1 の計測と同じ)、n_predict は指定しない (長い訳文を切らず、止めるのは利用者)。stop に <|plamo:tag|> と <|plamo:bos|> を入れる。
- 停止は、実行中の翻訳の取り消し印と接続の複製を持ち、接続を閉じて行う。新しい翻訳は前の翻訳を止めてから始める。画面は停止を押した時点で処理中の表示を消し、その翻訳の残りの断片を捨てる。翻訳エンジンの読み込みを待っている間に止めた翻訳は、読み込みが終わっても要求を送らない。
- 翻訳先は選択値、自動のときは仮に日本語、翻訳元は自動なら入れない (src/lib/languages.ts の promptLanguages)。TASK-10 が優先順位による翻訳先の決定に置き換える。
- 停止ボタンは翻訳ボタンの左に常に置き、翻訳していないときは理由付きで無効にする。失敗の理由は翻訳ボタンの行に一行で出し、読み上げにも届ける (どちらもオーナー判断)。告知の部品 (snz-design doc-9 §6.4) への置き換えは後のタスクで行う。

## 確かめたこと (macOS 26 / M2、2B Q4_K_M)

- 実際の llama-server (b12110 の静的ビルド) に対する一時的なテスト (コミットしていない):
  - 停止中から acquire して起動を待ち、2.2 秒で待機中になった (AC#3)。
  - 英日の 1 文が 15 個の断片で届き、最初の断片は 0.74 秒、最後は 1.07 秒。訳文は「デザインチームの数名が今週出張するため、会議は来週の月曜日に変更となりました。」
  - 誤ったキーの要求は「the engine refused the translation (HTTP 401): Invalid API Key」になった。
  - 長い原文の生成を 1.5 秒で止めると Stopped で返り、次の翻訳は 0.26 秒で返った (llama-server が切断で生成を取り消し、枠が空く)。
- pnpm tauri dev で起動したアプリの翻訳エンジン: ps の引数に api-key が無く、環境変数に LLAMA_API_KEY があった。キー無しの /completion は 401、/health は 200 (AC#5)。アプリの終了で翻訳エンジンも終了した。
- 画面: Vite の開発サーバーを内蔵ブラウザで開き、Tauri の IPC を差し替えて確かめた。Cmd+Enter と翻訳ボタンで translate が呼ばれ (翻訳先 ja・翻訳元なし)、断片が順に訳文欄へ出て、翻訳中は翻訳ボタンが「翻訳中…」になり停止ボタンが押せる。停止で途中までの訳文が残り、処理中の表示が消えた。失敗時は「翻訳できませんでした: …」が翻訳ボタンの行に出た。
- 単体テスト: プロンプトの組み立て (翻訳元あり・なし、原文をそのまま入れる)、要求の本文、chunked と chunked でない応答、途中で切れた応答、401、ストリーム中のエラー、API キーの送信、停止、接続前の停止、新しい翻訳による前の翻訳の停止 (AC#4)。Cmd/Ctrl+Enter の判定と IME 変換中の除外、言語の仮決め。
- cargo test 29 件、cargo clippy -D warnings、cargo fmt --check、pnpm check、pnpm lint、pnpm test 24 件が通った。

## 確かめていないこと

- AC#1・#2 を実アプリの WebView から通しで動かすこと。コンピューターを操作する手段がこのセッションに無く、ネイティブのウィンドウを操作できなかった。画面 (IPC を差し替え) と Rust 側 (実エンジン) は別々に確かめたが、実 WebView の IPC (Channel と引数名 onPiece の受け渡し) を通した動作は未確認なので、AC#1・#2 は未チェックのまま残した。pnpm tauri dev で原文を入れて翻訳・停止を一度試せば確かめられる。
<!-- SECTION:NOTES:END -->
