---
id: TASK-24
title: モデルの入手方法を決めて作る
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-09 21:50'
labels: []
milestone: m-5
dependencies:
  - TASK-23
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
開発中はモデルを手で置いていたが、配布するならアプリが入手を助ける必要がある。オフラインで使う要求 (R1) と、ライセンスの条件の両方を満たす形を選ぶ (doc-1 §10)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 同梱・初回ダウンロード・手元のファイルの取り込みを比べ、選んだ方式と理由を doc-1 §10 に書いている
- [ ] #2 選んだ方式が macOS と Windows で動き、ダウンロードするならライセンスへの同意と SHA-256 の照合を行う
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
方式は初回のダウンロード (2026-11-10 オーナー判断)。HTTPS は OS の curl を子プロセスで使い、SHA-256 は sha2 クレート (Cargo.lock に既にある 0.10.9) で計算する。models/ に手で置く今の方法はオフラインの機械向けに残す。
1. doc-1 §10 に、同梱・初回のダウンロード・手元のファイルの取り込みの比較と、選んだ方式・理由・入手の手順を書く
2. src-tauri/src/download.rs: 0.8B・2B・8B の Q4_K_M を、固定したコミット・バイト数・SHA-256 の一覧として持つ。curl (macOS は /usr/bin/curl、Windows は System32\curl.exe) で models/<名前>.part へ取得し (-C - で再開、https 以外へのリダイレクトを拒む)、.part の大きさで進捗を出し、停止は子プロセスの終了で行う。取得後に SHA-256 を照合し、一致したら .gguf へ改名する
3. コマンド models_catalog・model_download (Channel で進捗)・model_download_cancel を足す。アプリ終了時に curl を終了する。取得後、モデルが無くて止まっていた翻訳エンジンを起動する
4. 固定したコミットの LICENSE/ja・LICENSE/en を書き写さずにリポジトリへ置き、同意の画面で示す (D3。TASK-29 の D2 でも使える)
5. 画面: 進捗の表示 (snz-design doc-8 §6.7.1、量が分かる変種) の部品、モデルの入手のモーダル (サイズの選択・条文・年齢の条件を含む同意・進捗・停止・失敗)、GGUF が無いときのメイン画面の案内と設定画面の入手ボタン
6. cargo test / clippy、vitest、svelte-check、biome。macOS で 0.8B を実際に取得し照合と読み込みを確かめる。Windows はオーナーの機械で確かめる
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 方式は初回のダウンロード、HTTPS は OS の curl を子プロセスで、SHA-256 は sha2 クレート (2026-11-10 オーナー判断。着手時に 3 つを選択肢で確認した)。比較と理由は doc-1 §10.4。手元のファイルの取り込みは models/ に手で置く今の方法で代わりになるので、取り込みの画面は作らなかった。
- AC#1: doc-1 §10.4 に 3 方式の比較表、選んだ方式と理由、固定したコミット・バイト数・SHA-256、取得・照合・画面・D3 の扱いを書いた。§10 冒頭の「m-5 で決める」と §10.3 の D3 の担当も書き直した。
- AC#2 の macOS 側で確かめたこと:
  - 実際の Hugging Face から 0.8B (573,126,848 バイト) を `download::fetch` で取得し、大きさと SHA-256 が一致して models/ に置かれた (`cargo test -- --ignored fetches_the_real`、581 秒。回線は約 1 MB/s)。
  - HF の CDN が `--continue-at -` に 206 を返し、続きから取得できることを curl で直接確かめた (302 で us.aws.cdn.hf.co へリダイレクトされる)。
  - curl を `file://` で動かす単体テスト 12 件: 取得と改名、`.part` からの再開、完全な `.part` は照合だけ、SHA-256 の不一致で削除、長すぎる `.part` の取り直し、大きさ違い、curl の失敗、https 以外の拒否、既にある GGUF を上書きしない、停止で `.part` を残す、開始前の停止。条文ファイルの SHA-256 が固定コミットの LICENSE と一致するテストも足した。
  - 画面は Tauri の IPC をモックしてブラウザで動かした (コミットしていない一時ファイル): GGUF が無いときのメイン画面の案内、入手のモーダル (2B が先に選ばれる、同意しないと入手できない、日本語と英語の切り替え)、進捗、成功時の通知と案内の消去、失敗の告知、停止でフォームへ戻りフォーカスが残る、設定画面から開く経路、前回の `.part` の続きの案内。
- AC#2 を未チェックにした理由: (1) Windows (curl.exe・Schannel・利用者名に日本語を含むパス) は確かめていない。(2) 実際のアプリ (`pnpm tauri dev`) で入手の画面を操作し、取得後に翻訳エンジンが起動するところは見ていない (モックと Rust の単体の確認だけ)。
- 画面の判断: 入手中はモデルの選択と条文を隠して進捗だけを出す (条文の下に進捗が押し出されて見えなかったため)。「閉じる」と「停止」は同じボタンの役割を切り替え、停止した後もフォーカスが残るようにした。同意は保存せず入手のたびに求める。
- `sha2` は `profile.dev.package.sha2` で開発時も最適化する。Cargo.lock の差分は依存の 1 行だけで、取得するクレートは増えていない。
- テスト: cargo test 218 件 (無視 1)、cargo clippy --all-targets 警告なし、vitest 96 件、svelte-check 0 件、biome lint 0 件。
<!-- SECTION:NOTES:END -->
