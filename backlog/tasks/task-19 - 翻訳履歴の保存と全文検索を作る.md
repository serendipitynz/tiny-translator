---
id: TASK-19
title: 翻訳履歴の保存と全文検索を作る
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-09 07:38'
labels: []
milestone: m-4
dependencies:
  - TASK-9
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R7。以前の翻訳を探して再利用したい。日本語も検索できる必要がある (doc-1 §9)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 翻訳が完了するたびに原文・訳文・翻訳元言語・翻訳先言語・形式・モデル名・日時を SQLite へ保存し、停止した翻訳は保存しない
- [x] #2 FTS5 (trigram) で原文と訳文を日本語・英語とも検索できる
- [x] #3 保存件数の上限を超えたら古いものから消える
- [x] #4 保存・検索・削除に単体テストがある
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. src-tauri に rusqlite 0.41.1 (bundled) を入れる (2026-11-09 オーナー承認)。
2. history.rs: アプリデータディレクトリの history.sqlite3 に translations 表 (原文・訳文・翻訳元言語 (判定なしは NULL)・翻訳先言語・形式・モデル名・日時 (UNIX ミリ秒)) と、原文・訳文を索引する外部コンテンツの FTS5 (trigram) 表を作り、トリガーで同期する。スキーマは user_version で版を持つ。
3. 保存: 1 件追加と上限 (既定 1,000) を超えた古いもの (id の小さいもの) の削除を 1 トランザクションで行う。
4. 検索: 空白で区切った語の AND。3 文字以上の語は FTS5 の MATCH (句として引用)、trigram が索引できない 1〜2 文字の語は同じ表への LIKE で探す。新しい順に返す。空の検索語は全件。
5. 削除: 1 件削除と全件削除。
6. モデル名: engine の Spawn が起動したモデルのファイル名も返すようにし、Lease から読めるようにする。
7. translate コマンドで Append/Replace から最終の訳文を組み立て、Outcome::Done のときだけ保存する。停止 (Stopped) と失敗は保存しない。保存の失敗は翻訳の失敗にしない。
8. 一覧画面 (TASK-20) が呼ぶ history_search / history_delete / history_clear コマンドを用意する。
9. 保存・検索 (日本語・英語、短い語)・上限・削除の単体テストを書き、cargo test / clippy / fmt と pnpm の check・lint・test を通す。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 本番依存に rusqlite 0.41.1 (bundled) を追加した (2026-11-09 オーナー承認)。推移的に入るのは libsqlite3-sys・hashlink・fallible-iterator・fallible-streaming-iterator・smallvec・bitflags。bundled の SQLite で FTS5 の trigram が使えることは、テストと実機のアプリでスキーマを作れたことで確かめた。
- 保存は画面ではなく Rust の translate コマンドの中で行う (translate_and_record)。訳文は document::translate の Append/Replace から組み立てるので、画面に出た最終の訳文と一致する。Outcome::Done のときだけ保存し、停止・失敗は保存しない。保存の失敗は翻訳の失敗にしない。
- モデル名は engine の Spawn が起動したモデルのファイル名も返すようにし (engine::Spawned)、Lease::model で読む。翻訳の時点で実際に読み込まれていたモデルを記録するためで、保存時に choose_model をやり直すと設定画面 (TASK-21) でモデルを変えた直後に食い違う。
- 検索: 3 文字以上の語は FTS5 の MATCH (句として引用するので演算子は効かない)、trigram の索引で探せない 1〜2 文字の語は LIKE (% と _ はエスケープ)。語は AND。
- 上限超過の削除は id 順。AUTOINCREMENT で id を再利用しない。スキーマは user_version=1 で、より新しい版のファイルは開かずに履歴なしで動く。
- 一覧画面 (TASK-20) が呼ぶ history_search / history_delete / history_clear コマンドも用意した。検索結果は全文を返すので、長い原文が多いと IPC が重くなる可能性がある。一覧で要約だけ返すかは TASK-20 で判断する。
- doc-1 §1 の「翻訳履歴」の定義にモデル名を加え、§9 に実装の判断を追記した。

検証:
- cargo test: 187 件通過。history の 13 件 (全項目の保存、再オープン後の保持、新しいスキーマの拒否、空の検索語、英語の大小無視、日本語、3 文字未満の語、AND、検索構文のエスケープ、上限 3 件での古いものの削除と索引の件数、1 件削除、id の非再利用、全件削除) と lib の 4 件 (完了は全項目つきで保存、停止・失敗は保存しない、履歴を開けなくても翻訳は完了する)、engine の Lease::model の 1 件。
- cargo clippy --all-targets -- -D warnings、cargo fmt --check、pnpm check / lint / test (47 件) が通過。
- pnpm tauri dev で起動し、アプリデータディレクトリに history.sqlite3 (user_version=1) が作られることを確かめた。

確かめていないこと: 実機のエンジンで翻訳 → 保存、翻訳中の停止 → 未保存、の手動確認はしていない (画面操作を伴うため)。経路は translate_and_record の単体テストで、偽のモデルを使って確かめている。Windows でのビルドは TASK-22。
<!-- SECTION:NOTES:END -->
