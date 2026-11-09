---
id: TASK-17
title: 形式の自動判定と形式の選択を作る
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-09 02:02'
labels: []
milestone: m-3
dependencies:
  - TASK-14
  - TASK-15
  - TASK-16
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
利用者が形式を毎回選ばなくて済むように、原文から形式を判定する (doc-1 §7.2)。誤判定のときは手で選び直せる必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 原文欄の下 (言語の判定と同じ行) に形式の選択 (自動・テキスト・Markdown・JSON・HTML) があり、自動のとき判定した形式を出す
- [x] #2 doc-1 §7.2 の規則で判定し、判定の単体テストがある
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. src-tauri/src/format.rs に形式の判定を置き、翻訳と同じパーサーで判定する: JSON は serde_json で読めて最上位がオブジェクトか配列、HTML は html5gum のトークンで `<html>`・doctype があるか、閉じタグを持つブロック要素が 2 つ以上で Markdown の記法より多い (Markdown のコードフェンスとインラインコードの中は数えない)、Markdown は pulldown-cmark の ATX 見出し・リスト・コードフェンス、どれでもなければテキスト
2. detect_format コマンドを lib.rs に足し、判定の単体テストを書く
3. 画面: 原文パネルの下端に形式の選択 (自動 (判定した形式) / テキスト / Markdown / JSON / HTML) を言語の判定と並べる。自動のときは言語と同じく入力を少し待って判定し、翻訳の開始時にも判定し直す
4. doc-1 の §7.2 (判定規則の詳細) と §8.1 を直す。AC #1 の位置は 2026-11-09 オーナー判断で下端に変えた
5. pnpm check / lint / test、cargo clippy / test / fmt、実機で表示を確かめる
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装 (TASK-17)

- `src-tauri/src/format.rs`: 形式の判定 (`format::detect`) を置き、`detect_format` command で画面へ出した。判定は翻訳と同じパーサー (serde_json・html5gum・pulldown-cmark) で行う。画面側の JSON.parse や正規表現で判定すると、「判定は JSON なのに翻訳では読めない」「判定が数えるブロック要素と翻訳が区切るブロック要素が違う」というずれが起きうるためである。
- `src-tauri/src/html.rs` に `signs` (html 要素・doctype の有無と、閉じタグを持つブロック要素の数)、`src-tauri/src/markdown.rs` に `signs` (`#` の見出し・リスト・コードフェンスの数と、コードの範囲) を足した。翻訳の動きは変えていない。
- 画面: 原文パネルの下端に形式の選択欄を置き、言語の判定と同じ行に並べた。判定した形式は「自動 (Markdown)」のように選択肢の語に出す。形式は選択にかかわらず判定するので、手で選んだあとも一覧で判定を確かめられる。翻訳の開始時には言語と同じく原文から判定し直す。`src/lib/formats.ts` に選択肢を置いた。

## 着手時・実装中に決めたこと

- 形式の選択欄の位置 (2026-11-09 オーナー判断): AC #1 と doc-1 §7.2 は「原文欄の上」、§8.1 の図は「原文パネルの下端・言語の判定の横」と食い違っていた。図に合わせて下端にし、AC #1 の文言と §7.2 を直した。最小幅 480px でも収まる。
- §7.2 の規則のうち曖昧だった所を次のように詰め、doc-1 §7.2 に書いた。
  - JSON は最上位がオブジェクトか配列のものだけにした。`true`・`null`・数字だけの原文が JSON と判定されて訳されなくなるのを防ぐためである。
  - HTML の「多ければ」は、閉じタグを持つブロック要素が 2 つ以上あり、かつ Markdown の記法の数より多いこととした。`<div align="center">` で始まる README を Markdown と判定するためである。Markdown のコードフェンスとインラインコードの中、svg などの中身を訳さない要素と pre の中は数えない。
  - Markdown の見出しは `#` で書いたものだけを数える。テキストのメールで署名の前に置く `--` が、上の行を見出しにするためである。

## 確かめたこと

- AC #2: `format::tests` の 12 件で、オブジェクトと配列 / 単独の JSON の値 / JSONC と末尾のカンマ / doctype と html 要素 / ブロック要素 2 つ以上 / 1 つだけ / HTML を含む README / コードの中の HTML / svg と pre の中 / 見出し・リスト・コードフェンス / 下線の見出し / 普通の文 を確かめた。
- AC #1: Vite の開発サーバーを内蔵ブラウザで開き、Tauri の invoke をモックに替えて DOM から操作した。幅 800px と 480px で、選択欄が原文パネルの下端に判定と並び、「自動 (Markdown)」と出ることを確かめた。原文を空にすると「自動」に戻る。translate に渡る形式は、自動なら判定した形式 (markdown・html)、JSON を選べば json だった。JSON を選んだままでも一覧の「自動」は判定 (「自動 (HTML)」) を出した。
- `cargo test` 168 件 (うち新規は format 12)、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`pnpm test` 47 件 (うち新規 3)、`pnpm check`、`pnpm lint` が通る。
- 測っていないこと: 実際の Tauri のウィンドウ (WKWebView) での表示と、そこから実機のモデルで各形式を訳すこと。画面側は上のモックでの確認だけである。
<!-- SECTION:NOTES:END -->
