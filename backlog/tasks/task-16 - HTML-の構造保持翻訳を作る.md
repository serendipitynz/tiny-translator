---
id: TASK-16
title: HTML の構造保持翻訳を作る
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-09 00:15'
labels: []
milestone: m-3
dependencies:
  - TASK-13
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
  - backlog/docs/doc-2 - 先行事例と差別化.md
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R8 のうち HTML。インライン要素を含む文はタグごと訳す必要がある。PLaMo 3 Translate はタグを保つ学習をしているが、2B で保たれる割合は未確認 (doc-1 §11)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 HTML パーサーを選定し、本番依存としてオーナーの承認を得ている
- [x] #2 ブロック要素の中身と alt・title 属性を翻訳し、script・style・code・pre とその他の属性を原文のまま残す
- [x] #3 インライン要素を番号付きのタグで渡し、訳文で元の要素と属性へ戻す
- [x] #4 2B でタグの数と順序が保たれた割合を測り、doc-1 §11 へ反映している
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 本番依存 html5gum 0.9.1 (default-features のまま。必須の依存は無い) を追加する (2026-11-09 オーナー承認)。原文のバイト位置が全トークンと属性値に付くので、Markdown・JSON と同じく原文の範囲へ訳文を差し込む方式にできる。
2. src-tauri/src/html.rs: html5gum の位置付きトークン (naive state switching で script・style・textarea・title を生テキストとして読む) を走査し、インラインの並び (文字列・インライン要素・空要素) をブロック要素の境界で区切って翻訳単位にする。pre・listing はその終わりまで、script・style・textarea・svg・math・iframe・video・audio・canvas・object・picture・map などは要素ごと原文のまま残す。翻訳単位の文字列は原文に書いたとおり (実体参照も) 切り出し、改行を含む空白は改行 1 つにする。文字を含むかは実体参照を解いた文字で判定する。
3. インライン要素のタグ: 中身を訳す要素 (a・b・em・span など。不明な要素は span) は対、code・kbd・samp はそのまま (要素ごと原文へ戻す)、br・img・input・wbr・embed・コメント・中身を訳さない要素は空要素。開始タグ・終了タグは原文のまま戻すので属性は保たれる。対にならない開始・終了タグは空要素にする。空要素は、タグを外した訳し直しで原文のマークアップを渡し、訳文に書いたとおり残っていなければ翻訳単位を原文のまま残す (tags.rs に印を足す。Markdown の動きは変えない)。
4. alt・title: 翻訳単位の外のタグにあるものは、その値だけの翻訳単位にする (引用符に合わせてエスケープ。引用符の無い値は \" で囲む)。翻訳単位の中のインライン要素にあるものは、その翻訳単位の前に履歴無しで 1 つずつ訳し、訳した値でタグを組み直してから翻訳単位を訳す。
5. document::Format に Html を足し、translate コマンドと画面の Format 型に html を足す (形式の選択は TASK-17)。
6. 単体テストと、代表的な HTML の入出力を固定したテスト。実機 (M2・2B) で英日・日英の HTML を訳し、タグ付きの翻訳単位でタグの数と順序が保たれた割合を数える。
7. doc-1 §3・§7.1 (HTML の節)・§11 を改める。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装 (TASK-16)

- 本番依存 html5gum 0.9.1 を追加した (2026-11-09 オーナー承認。必須の依存は無く、新しく入るクレートは html5gum だけ)。候補は html5ever (原文の位置を持たず、ツリーから書き直すと原文が変わる)・tl (WHATWG 非準拠で実体参照を解かない)・lol_html (依存が 10 以上) と比べた。
- `src-tauri/src/html.rs`: 位置付きのトークンを走査し、インライン要素でないタグを境界にして、文字列とインライン要素の並びを翻訳単位にする。タグ・属性・コメント・script などは原文から写す。ツリーは作らない。
- `src-tauri/src/tags.rs`: `Builder::markup` を足した。HTML の空要素 (img・br・コメント・script など) は、タグを外した訳し直しで原文のマークアップのまま渡し、訳文に書いたとおり残っていなければ翻訳単位を原文のまま残す (インラインコードと同じ確認)。Markdown の空要素の動きは変えていない。
- `src-tauri/src/document.rs`: `Format::Html`、属性値を書き戻す `Lines::Attribute`、`Unit::attributes` を足した。翻訳単位の中のインライン要素にある alt・title は、その翻訳単位の前に履歴無しで訳し、`html::retag` で開始タグを組み直してから翻訳単位を訳す。翻訳単位の並びは原文の位置で決まり、文の中の値を後ろの翻訳単位にできないためである。
- `translate` command と画面の `Format` 型に "html" を足した。形式の選択欄は TASK-17 なので、画面は当面 "text" を送る。

## 着手時・実装中に決めたこと

- `code`・`samp` は原文のまま残し、`kbd` は対にして中身を訳す。最初は kbd も残したが、実機で `<kbd>保存</kbd>` が英訳でも `保存` のまま残ったため変えた。samp はプログラムの出力で、表示と照らし合わせるので残す。
- 訳す属性は AC のとおり alt と title だけにした (placeholder・aria-label などは訳さない)。
- 文字列は実体参照を残したまま (書いたとおり) 渡す (Markdown と同じ)。文字を含むかは実体参照を解いた文字で判定する。
- `/>` は svg・math だけ自己終了と読む (ブラウザと同じ)。
- doc-1 §3・§7.1 (HTML の表の行と「HTML (TASK-16)」の節)・§11 を改めた。

## 確かめたこと

- `cargo test` 155 件 (うち新規は html 18・document 4)、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`pnpm test` 44 件、`pnpm check`、`pnpm lint` が通る。
- AC #2・#3: `document::tests::html_keeps_its_structure_and_translates_only_its_text_alt_and_title` が、doctype・head の title と style・見出し・属性付きの段落・インラインの code と kbd と a (title 付き)・ブロックの位置の img alt・リスト・表 (文字を含まないセルと &nbsp;)・pre・コメント・script を含む文書の訳文を完全一致で比べ、各翻訳単位がモデルへ渡る形も確かめる。html の単体テストで、対にならないタグ・交差するタグ・引用符の無い値・CRLF・svg と math の自己終了などを確かめた。
- AC #4 (実機、MacBook Air M2・2B Q4_K_M、アプリと同じ引数の llama-server に一時テストから接続): 英日・日英それぞれ、利用ガイド風の文書 (タグ付き 21 件) と語順が入れ替わりやすい文の文書 (10 件) を訳した。
  - タグ付きの翻訳単位 62 件すべてでタグの数と番号が保たれ、タグを外した訳し直しは 0 回。
  - タグの順序が原文と同じだったのは 52 件 (英日 21+4、日英 21+6)。残る 10 件は語順に合わせて入れ替わり、すべて元の要素と属性へ戻った。
  - 所要時間: 利用ガイド風は英日 21 秒・日英 44〜80 秒、語順の文書は英日 36 秒・日英 38 秒 (日英の回は機械側の負荷でぶれた。速度はこのタスクの対象外)。
- 測っていないこと: アプリの画面での HTML の翻訳 (形式の選択欄が TASK-17 のため)、大きなページ (数百の翻訳単位) の所要時間、タグが合わない訳文が実機で出る頻度 (今回の 62 件では 0)、0.8B と 8B、Windows (TASK-18)、`title` の中に `<` を書いた原文 (翻訳単位を組み直すときにタグとして読まれる)。

## レビューでの修正 (1 回目、Codex)

- [P2] `<title><textarea>の使い方</title>` のように `title` の中にタグに見える文字列があると、翻訳単位を組み立てるときに切り出した部分を普通の HTML として読み直すので、`<textarea>` が中身を訳さない要素になり、タイトルが訳されなかった。`title` の中身は読み直さず、タグの無い翻訳単位にした (`html::tests::a_title_is_text_whatever_it_holds`)。上の「測っていないこと」の `title` の項目はこれで直った。doc-1 §7.1 の HTML の節に 1 行足した。`cargo test` は 156 件。
<!-- SECTION:NOTES:END -->
