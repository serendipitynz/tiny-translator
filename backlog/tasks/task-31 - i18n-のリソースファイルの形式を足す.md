---
id: TASK-31
title: i18n のリソースファイルの形式を足す
status: Done
assignee: []
created_date: '2026-11-10 09:59'
updated_date: '2026-11-11 03:10'
labels: []
dependencies:
  - TASK-28
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
先行事例と差をつけるため、構造保持翻訳 (m-3) を開発者が自分のアプリを多言語化する作業へ広げる (2026-11-10 オーナー判断、doc-2 §2)。PLaMo 3 Translate を使う他のアプリはブラウザ拡張・画面への重ね表示・選択範囲の翻訳など「読む」用途に寄っていて、リソースファイルを壊さずに訳せるものは無い。今の構造保持は Markdown・JSON・HTML だけで、i18n では YAML、Apple の .strings / .xcstrings、gettext の .po、Flutter の ARB、Android の strings.xml もよく使われる。どの形式を対象にするかは着手時に決める。プレースホルダーの保持は TASK-28 で決める記法に従う。形式が多いので、着手時に形式ごとのタスクへ分けてもよい。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 対象にする形式を決め、doc-1 §7 に形式ごとの訳す部分と訳さない部分を書いている
- [x] #2 決めた形式それぞれで、キー・コメント・構造を変えずに、訳すべき文字列だけを訳したファイルを出力できる
- [x] #3 決めた形式それぞれで、TASK-28 で守ると決めたプレースホルダーが原文のとおりに残る
- [x] #4 決めた形式が形式の自動判定 (doc-1 §7.2) で判定され、形式の選択欄からも選べる
- [x] #5 形式ごとに、訳したファイルをその形式のパーサーで読み戻せることをテストで確かめている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
対象形式: ARB と Apple .strings (2026-11-11 オーナー判断)。Android strings.xml・YAML・gettext .po・.xcstrings は形式ごとの別タスクに分ける。
1. JSON の値をエスケープを解いて翻訳単位にする処理 (Value と message) を values.rs に切り出し、JSON・ARB・.strings で共有する
2. ARB: JSON の翻訳に「最上位の @ で始まるキー (@@locale・@key のメタデータ) の値は訳さない」を足す。判定は JSON のうち最上位に @@locale か、ほかのキーに対応する @key があるもの
3. .strings: strings.rs に手書きのパーサーを作る。キー・コメント (// と /* */)・= と ; と空白は原文から写し、引用符で囲んだ値だけを訳す。エスケープ (\" \\ \n \t \r \U 8 進) を解いて渡し、訳文は書き戻すときにエスケープする。引用符の無い値は訳さない
4. 翻訳しないキーを ARB と .strings にも効かせ、設定画面の見出しと説明を直す
5. 形式の判定 (format.rs) と選択欄 (formats.ts・translate.ts) に arb と strings を足す
6. テスト: 形式ごとに訳した結果を serde_json (ARB) と plist の ASCII リーダー (.strings、dev-dependency) で読み戻す。macOS では plutil でも読み戻す
7. doc-1 §7.1・§7.2 に形式ごとの訳す部分と訳さない部分を書く。残りの形式の別タスクを作る
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 対象形式は ARB と Apple .strings にした (2026-11-11 オーナー判断)。どちらも依存を増やさずに作れる。Android strings.xml・YAML・gettext .po・.xcstrings は TASK-36〜39 に分けた。.po と .xcstrings は「訳文の欄を埋める」形で、構造を変えずに訳すこのタスクの方式と合わないため別の設計判断が要る。
- JSON の値をエスケープを解いて翻訳単位にする処理 (Value と message) を values.rs に切り出し、JSON・ARB・.strings で共有した。プレースホルダー・ICU・訳し直しの段階は 3 形式で同じ動きになる。
- ARB は JSON の走査に「最上位の @ で始まるキーの値は訳さない」を足しただけ。@@locale は書き換えない (翻訳先の言語のコードと Flutter のロケールの書き方が違うため。doc-1 に記載)。
- .strings は Core Foundation と同じ文法の手書きパーサー。引用符の無い値 (CFBundleName = MyApp;) は名前として訳さない。plutil で文法の境界 (a/*x*/ は名前 a/ になる、; は省けない) を確かめて合わせた。
- 翻訳しないキーを ARB と .strings にも効かせ、設定画面の見出しを「JSON の翻訳しないキー」から「翻訳しないキー」に改めた。
- 読み戻しのテスト (AC #5): ARB は serde_json、.strings は plist クレート 1.10 の古い形式のリーダー (dev-dependency。Tauri が既に依存しているので新しいクレートは入らない) と、macOS では plutil。Cargo.lock の変化に合わせて THIRD-PARTY-LICENSES.txt を生成し直した (digest の 1 行だけ変わる)。
- テスト: cargo test 252 件 (うち新規: strings 9・format 2・document 5)、clippy 警告 0、pnpm test 108 件、biome lint・svelte-check エラー 0。
- 実測 (M2 の 2B Q4_K_M、アプリと同じ引数の llama-server に一時テストから接続): ARB 英日 9 メッセージ (翻訳単位 16、うちタグ付き 6) 20.4 秒、.strings 英日 10 エントリ (翻訳単位 9、うちタグ付き 5) 15.2 秒、.strings 日英 8 エントリ (翻訳単位 7、うちタグ付き 4) 11.3 秒。タグ付きの翻訳単位はすべてタグが保たれ、タグを外した訳し直しは 0 回。語順に合わせたプレースホルダーの入れ替えも戻せた。
- 確かめていないこと: 実アプリ (pnpm tauri dev) での選択欄の見え方 (480px 幅で「自動 (.strings)」が収まるか) と、Windows での実行。

- レビュー (Codex、2 回): 1 回目に [P2] 2 件。.strings の小文字の \u をエスケープとして読んでいた点と、8 進を Latin-1 で読んでいた点 (Apple は NeXTSTEP の文字コード) で、翻訳しないキーの照合が Xcode の読むキーとずれる。plutil で再現を確かめ、Core Foundation と同じ読み方に直した (0xFE・0xFF と数字の無い \U は NUL、\400 以上は下位 1 バイト)。8 進の \0〜\777 と \U の書き方を plutil と突き合わせる macOS のテストを足した。2 回目は指摘なしで APPROVE。修正後の cargo test は 253 件。
- PR#32 でマージした。
<!-- SECTION:NOTES:END -->
