---
id: TASK-28
title: JSON の文字列のプレースホルダーを訳さずに残す
status: Done
assignee: []
created_date: '2026-11-08 21:36'
updated_date: '2026-11-10 20:53'
labels: []
milestone: m-3
dependencies:
  - TASK-15
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
i18n のリソースファイルの文字列は `{name}`・`{{count}}`・`%s`・`%1$s` などのプレースホルダーを含む。TASK-15 の JSON の構造保持翻訳はこれらを文字列の一部としてそのままモデルへ渡すので、プレースホルダーの中の語 (`{name}` の name) が訳されたり、記号が崩れたりすると、訳したリソースファイルがアプリで使えなくなる。TASK-15 の着手時に、守る範囲を決める作業を別タスクに分けた (2026-11-09 オーナー判断)。Markdown のインラインコードと同じ「そのまま」のタグ (doc-1 §7.1、TASK-14) で守る形が候補。ICU の plural・select (`{count, plural, one {# item} other {# items}}`) のように中に訳すべき文を持つ記法をどう扱うかも決める必要がある。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 守るプレースホルダーの記法を決め、doc-1 §7.1 の JSON の節に書いている
- [x] #2 決めた記法のプレースホルダーは、訳文に何が書かれても原文のとおりに戻る
- [x] #3 プレースホルダーを戻せない訳文の扱い (訳し直し・原文のまま残す) が決まっていて、テストで確かめている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 着手時の判断 (2026-11-11 オーナー判断): 守る記法は 3 系統 — 波括弧 1 つ ({name}・{0}・{0:.2f}・{n, number}、前置きの $・%・# を含む)、二重波括弧 ({{count}}・{{- x}})、printf (%s・%d・%1$s・%@・%.2f・%ld・%%。空白フラグは除く)。常に有効で設定は作らない。ICU の plural・select・selectordinal は骨組みを原文のまま残し、分岐の文を 1 つずつ翻訳単位にする (# と分岐の中のプレースホルダーはタグで守る。入れ子も同じ)。戻せないときは Markdown のコードと同じく、タグを外して訳し直し、プレースホルダーが書いたとおり同じ数だけ含まれなければ原文のまま残す。
2. src-tauri/src/placeholders.rs (新規): エスケープを解いた文字列から、プレースホルダーと ICU の複合引数の位置を見つける。
3. json.rs の unit(): 値を ICU の骨組み (Kept) と分岐・前後の文 (Unit) に分け、各翻訳単位はプレースホルダーを tags::Builder のそのままの要素 (code) にする。原文の位置 (エスケープ済み) を要素の原文に、エスケープを解いた文字列をモデルへ渡す文字列とタグを外した文字列に使う。プレースホルダーだけの値は送らない。
4. tags::Builder に、タグを外した文字列へ入れる文字列を原文と別に渡せるそのままの要素を足す (JSON ではモデルへ渡すのはエスケープを解いた文字列、戻すのは原文の書き方のため)。
5. テスト: placeholders の検出 (誤検出しない例を含む)、json の分け方、document で訳文が何を書いても原文のとおりに戻ること (タグを崩した訳文・タグを外した訳文がプレースホルダーを変えた場合は原文のまま)。
6. doc-1 §7.1 の JSON の節に記法・ICU・戻せないときの扱いを書く。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装 (TASK-28)

- `src-tauri/src/placeholders.rs` (新規): エスケープを解いた文字列値から、3 系統のプレースホルダーと ICU の選択の引数 (`plural`・`select`・`selectordinal`) を読み、訳す部分 (分岐の文と、引数の前後の文) をプレースホルダーの位置付きで返す。正規表現のクレートは入れず、手書きの走査にした (依存を増やさないため)。ICU の引数はいったん木に読んでから平らにする。読めなかったときに途中まで書いた結果を捨てる必要がないためである。
- `src-tauri/src/json.rs`: 値の中身を `Value` (エスケープを解いた文字列と、各文字の原文での位置) にし、分岐の文ごとに `tags::Builder` で翻訳単位を作る。プレースホルダーはそのままの要素 `code` にした。要素の原文は原文に書いたとおり (エスケープ済み) で、モデルへ渡す文字列とタグを外した文字列はエスケープを解いたもの。骨組みは翻訳単位の間の原文としてそのまま写る。
- `src-tauri/src/tags.rs`: `Builder::kept_as` を足した。そのままの要素の原文とタグを外した文字列を分けて渡せるようにするためで、JSON ではモデルにエスケープを解いた文字列を見せ、戻すのは原文の書き方にする。既存の `kept` はこれを呼ぶ形にしたので、Markdown・HTML の動作は変わらない。
- 戻せないときの扱いは document.rs の既存の段階 (タグを外して訳し直す → `keeps_originals` で確かめて、満たさなければ原文のまま) にそのまま乗る。document.rs の本体は変えていない。

## 着手時に決めたこと (2026-11-11 オーナー判断)

- 守る記法は 3 系統 (波括弧 1 つ・二重波括弧・printf)。常に有効で設定は無い。`:name`・`$t(key)`・`@:key`・`<0>…</0>` は対象外。
- ICU の選択の引数は骨組みを原文のまま残し、分岐の文を 1 つずつ翻訳単位にする。
- タグを戻せないときは Markdown のインラインコードと同じ扱い (タグを外して訳し直し、プレースホルダーが書いたとおり同じ数だけ無ければ原文のまま)。
- doc-1 §7.1 の表の JSON の行と「JSON (TASK-15)」の節に、記法・ICU の扱い・戻せないときの扱い・残る制約・実測を書いた。

## 確かめたこと

- `cargo test` 227 件 (うち新規は placeholders 5・json 2・document 2)、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`pnpm test` 103 件、`pnpm check`、`pnpm lint` が通る。
- AC #1: doc-1 §7.1 に書いた。
- AC #2: `document::tests::json_placeholders_come_back_as_written_whatever_the_translation_holds`。偽の翻訳エンジンが、タグの順序を入れ替え、タグの中身を書き換える (`{名前}`・`{{種類}}`・`%1$d` を `%d` に・`# ` に空白を足す) 訳文を返す。それでも、各プレースホルダーと ICU の骨組みが原文の文字列のまま戻り、訳文が JSON として読めることを確かめる。`json::tests::a_placeholder_comes_back_as_the_original_escapes_it` は、原文がエスケープして書いたプレースホルダー (`{name}`・`{{ \"x\" }}`) が、書いたとおりに戻ることを確かめる。
- AC #3: `document::tests::a_json_value_is_made_again_without_tags_and_kept_when_that_changes_a_placeholder`。タグを落とした訳文はタグを外して訳し直す。その訳文がプレースホルダーを保てば使い、書き換えたり落としたりしたら値を原文のまま残し、次の翻訳単位に対訳を入れない。
- 実機 (MacBook Air M2・2B Q4_K_M、アプリと同じ引数の llama-server に一時テストから接続): i18n 風の 10 キーの英日 (翻訳単位 21、うちタグ付き 14) と 7 キーの日英 (翻訳単位 8、うちタグ付き 7) で、タグ付きの翻訳単位はすべてタグが保たれ、訳し直しは 0 回だった。訳文は JSON として読め、キーの順序も原文と同じだった。所要時間は英日 9.8 秒、日英 5.5 秒。
- 実機で見えた制約 (doc-1 に記録): 位置指定の無い printf が 2 つある値で、訳文が順序を入れ替えた (`Uploading %d of %d files` → `%d個のファイルのうち%d個…`)。文字列は原文のとおりだが、意味は入れ替わる。また、分岐の文を単独で訳すので短い句の訳が不自然になる (`# item in your cart` → `# カート内の商品`)。
- 測っていないこと: アプリの画面での操作 (command の引数は変えていないので、画面側の変更は無い)、0.8B と 8B、Windows、数百キーの大きな JSON の所要時間。

## レビューでの修正 (1 回目、Codex)

- [P2] タグを外した訳し直しの確認が「少なくとも同じ数」だったので、`%s left` を `%s / %s 残り` と訳すと通ってしまい、引数が 1 つの書式で例外になる。プレースホルダーは「ちょうど同じ数」を求めるようにした (`tags::Tagged::holds_placeholders_exactly`)。長い記法から数えて取り除くので、`{{name}}` が `{name}` としても数えられることはない。
- 指摘の外で直したこと: タグを戻せた訳文でも、タグの外にプレースホルダーを書き足すと同じことが起きるので、`restore` でも同じ確認をし、通らなければ戻せない訳文として扱う。
- `Builder::kept_as` を `Builder::placeholder` に改めた (「ちょうど同じ数」の確認を受ける要素であることを名前に出すため)。Markdown・HTML の「少なくとも同じ数」は変えていない。
- 確かめたこと: `cargo test` 229 件 (新規は tags 1・document 1)、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。doc-1 の JSON の節に、ちょうど同じ数を求める理由を書き足した。
<!-- SECTION:NOTES:END -->
