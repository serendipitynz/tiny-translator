---
id: TASK-10
title: 翻訳の向き (翻訳元言語の判定と優先順位による翻訳先の決定) を作る
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-07 06:45'
labels: []
milestone: m-2
dependencies:
  - TASK-9
  - TASK-2
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R5 の中心。翻訳元は自動判定を基本にし、翻訳先は第一優先言語と第二優先言語で決める (doc-1 §6.1, §6.2)。選択欄で言語を固定することもできる。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 翻訳元の選択欄が自動判定のとき whatlang で判定し、判定した言語を画面に出す
- [x] #2 翻訳先の選択欄が「自動 (優先順位)」のとき、doc-1 §6.2 の規則で翻訳先を決める
- [x] #3 選択欄で言語を指定するとその言語に固定される
- [x] #4 入れ替えボタンで翻訳元と翻訳先の選択と、訳文と原文が入れ替わる
- [x] #5 翻訳先の決定に単体テストがある
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
着手時の判断 (2026-11-07 オーナー判断): whatlang =0.18.0 を本番依存に追加する。選択欄には PLaMo の 53 言語を並べる (日本語・英語・中国語 2 種・韓国語を先頭に、残りを五十音順)。判定なしの表示は「判定: 不明」。入れ替えは選択欄の値をそのまま入れ替える (自動は自動同士)。

1. Rust: src-tauri/src/language.rs に翻訳元言語の判定 (仮名 → 日本語、whatlang の確からしさ 0.5 未満 → 判定なし) を作り、whatlang の言語を PLaMo の言語コードへ写す。中国語は簡体字と繁体字を見分けないので "zh" で返し、PLaMo が持たない言語は判定なしにする。コマンド detect_language で画面へ出す。単体テストを付ける。
2. 画面: languages.ts を 53 言語 (PLaMo のコードは小文字の zh-hans / zh-hant) にし、優先順位による翻訳先の決定と、プロンプトへ渡す言語 (判定が中国語ならプロンプトに入れない) を純粋関数にして単体テストを付ける (AC#5)。優先言語は TASK-21 まで日本語・英語の定数。
3. App.svelte: 原文の入力に合わせて判定を呼び、原文の枠の下に「判定: 英語」/「判定: 不明」を出す (Panel に footer を足す)。翻訳を始めるときはその時点の原文で判定し直して翻訳の向きを決める。入れ替えボタンで選択欄の値と原文・訳文を入れ替える (翻訳中は押せない)。
4. doc-1 §6.1/§6.2 に今回の判断 (判定なしの表示、中国語と PLaMo が持たない言語の扱い、選択欄の言語、入れ替え) を書き足す。
5. pnpm check / lint / test、cargo clippy / test / fmt、アプリを起動して画面で確かめる。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装の判断

- 着手時の判断 (2026-11-07 オーナー判断): whatlang =0.18.0 を本番依存に追加 (新たに入る推移的依存は hashbrown 0.15.5・foldhash 0.1.5・allocator-api2 0.2.21)。選択欄は PLaMo の 53 言語。判定なしの表示は「判定: 不明」。入れ替えは選択欄の値をそのまま入れ替える。いずれも doc-1 §6.1・§6.2・§6.4 に書き足した。
- 判定は Rust (src-tauri/src/language.rs、コマンド detect_language)、翻訳先の決定とプロンプトへ渡す言語は画面 (src/lib/languages.ts の targetLanguage / promptLanguages) に置いた。whatlang は Rust にしか無く、優先言語と選択欄の値は画面が持つため。翻訳先の決定を純粋関数にして単体テストを付けた (AC#5)。
- プロンプトの言語コードはモデルカード (pfnet/plamo-3-translate-2b) の表記に合わせて小文字の zh-hans / zh-hant にした (TASK-6 の仮置きは zh-Hans / zh-Hant だった)。
- whatlang は中黒 (U+30FB) を片仮名として数え、仮名が 5% 以上の漢字の原文を日本語とする。`北京・上海之间的高速铁路` が日本語と判定されたので、whatlang へ渡す前に中黒を空白に置き換えた。doc-1 §6.1 の「中黒は数えない」を whatlang 側でも守るため。
- whatlang の中国語 (簡体字と繁体字を見分けない) は "zh" で返し、画面は「判定: 中国語」、プロンプトには翻訳元言語を入れない。優先順位の比較では zh-hans / zh-hant のどちらとも一致とみなす。PLaMo が扱わない言語 (ポルトガル語など) は判定なし。
- 判定は原文の入力が 200 ms 止まったときに行い、翻訳を始めるときは原文から判定し直す (表示の判定が遅延の途中でも、翻訳の向きは訳す原文で決まる)。最初の判定が返るまでは表示しないので、打ち始めに「判定: 不明」がちらつかない。
- 判定の表示は原文の枠の内側、原文欄の下に置いた (doc-1 §8.1 の図)。snz-design のパネルにはフッター域が無いので、Panel は変えず本体域の中に置いた。
- 入れ替えは翻訳中は理由付きで無効にし、入れ替えたことを読み上げに届ける。
- 優先言語は設定画面 (TASK-21) まで DEFAULT_PRIORITIES (日本語・英語) に固定。

## 確かめたこと (macOS 26 / M2)

- TASK-2 の samples.tsv 132 件をアプリの detect に一時的なテスト (コミットしていない) で通し、正しい 84・判定なし 29・誤り 19、翻訳先が正しいのは 113 件で、doc-1 §6.1 の実測と一致した。
- Rust の単体テスト: 仮名 (半角を含む) で日本語、中黒だけの中国語は中国語、簡体字・繁体字・韓国語、英語とフランス語の 1 文、英語の 1 語と空文字列は判定なし、ポルトガル語 (確からしさ 0.5 以上) は判定なし。
- 画面: Vite の開発サーバーを内蔵ブラウザで開き、Tauri の IPC を差し替えて確かめた。英文 42 文字の入力で detect_language は 1 回だけ呼ばれ「判定: 英語」が出た。翻訳は source en・target ja (AC#2)。入れ替えで原文と訳文が入れ替わり「判定: 日本語」、翻訳は ja → en (AC#4)。中国語は「判定: 中国語」で source なし・target ja、判定なしは「判定: 不明」で target ja。翻訳元 韓国語・翻訳先 中国語 (繁体字) に指定すると source ko・target zh-hant で、判定の表示は消えた (AC#3)。自動/韓国語の入れ替えは韓国語/自動になった。翻訳中の入れ替えは無効で「翻訳中は入れ替えられません」が読み上げに届いた。選択欄は 54 項目 (自動 + 53 言語)。幅 560px のダーク配色でも判定の表示が崩れないことを見た。
- 単体テスト (AC#5): 優先順位による翻訳先の決定 (第一優先言語なら第二、それ以外と判定なしは第一、中国語と zh-hans / zh-hant の一致、指定した翻訳先の固定)、プロンプトへ渡す言語、判定の表示、選択欄の言語の数と並び順、入れ替えの無効条件、detect_language の呼び出し。
- cargo test 35 件、cargo clippy -D warnings、cargo fmt --check、pnpm check、pnpm lint、pnpm test 38 件が通った。

## 確かめていないこと

- AC#1 を実アプリの WebView から通しで動かすこと。ネイティブのウィンドウを操作する手段がこのセッションに無い。Rust の判定 (単体テスト) と画面 (IPC を差し替え) は別々に確かめたが、実 WebView から detect_language を呼ぶ経路は未確認なので AC#1 は未チェックのまま残した。pnpm tauri dev で英文と日本文を入れ、「判定:」の表示と翻訳の向きを一度試せば確かめられる。

## マージ後の確認

- AC#1: オーナーが実アプリ (pnpm tauri dev) で確かめた (2026-11-07)。実 WebView から detect_language を呼ぶ経路もこれで確認できた。
- PR: https://github.com/serendipitynz/tiny-translator/pull/10
<!-- SECTION:NOTES:END -->
