---
id: TASK-15
title: JSON の構造保持翻訳を作る
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-08 21:38'
labels: []
milestone: m-3
dependencies:
  - TASK-13
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R8 のうち JSON。i18n のリソースファイルなどを訳す用途を想定し、キーと値の型を保つ (doc-1 §7.1)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 文字列の値だけを翻訳し、キー・数値・真偽値・null と入れ子の構造を保つ
- [x] #2 翻訳しないキーを設定で指定できる
- [x] #3 訳文が JSON として読め、キーの順序が原文と同じであることをテストで確かめている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. src-tauri/src/json.rs を作る: serde_json で JSON として読めるか確かめたうえで、原文を位置付きで走査し、文字列の値の中身を翻訳単位、それ以外 (キー・数値・真偽値・null・記号・空白) を原文のまま残す区間にする。キーの順序と書式は原文を写すので保たれる。
2. 文字列の値はエスケープを解いてモデルへ渡し、訳文は Lines::Json で JSON の文字列としてエスケープして書き戻す。値の前後の空白は翻訳単位に入れず原文のまま残す。文字を含まない値は送らない (Markdown と同じ規則)。
3. 翻訳しないキー: キー名の完全一致で、どの深さでも一致したキーの値 (配列・オブジェクトなら中の文字列すべて) を訳さない (2026-11-09 オーナー判断)。translate コマンドが一覧を受け取り、画面側は既定値 (空) を 1 か所に持つ。設定画面での指定は TASK-21 に AC を足す。
4. document::Format に Json を足し、画面の Format 型と要求に json と翻訳しないキーを足す (形式の選択は TASK-17)。
5. 単体テスト: 構造・型・キー順の保持、エスケープの往復、翻訳しないキー、壊れた JSON の拒否、訳文が JSON として読めること。
6. doc-1 §7.1 に JSON の節を足す。プレースホルダーの保護は別タスクにする (2026-11-09 オーナー判断)。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装 (TASK-15)

- `src-tauri/src/json.rs`: serde_json (既存の依存) で JSON として読めることを確かめてから原文を位置付きで走査し、文字列の値の中身を翻訳単位、それ以外 (キー・数値・真偽値・null・記号・空白) を原文のまま残す区間にする。値を serde_json の Value に読んで書き直す方式は採らなかった。キーの順序 (Value は既定で並べ替える)・字下げ・数値の書き方 (`1.50`) が変わるためで、Markdown と同じく原文の範囲へ訳文を差し込む方式にした。
- `document.rs`: `Format::Json { untranslated_keys }` と、訳文を JSON の文字列としてエスケープして書く `Lines::Json` を足した。流れてくる間の訳文・文の境界で分けた部分の間の改行も同じ経路でエスケープされる。JSON として読めない原文はモデルへ何も送らずにエラー (`not JSON: …`) にする。
- 値の前後の空白は翻訳単位に入れず原文のまま残す (`"Name: "` の末尾の空白を、モデルが落とすため)。文字を含まない値は送らない (Markdown と同じ規則)。
- `translate` command に `untranslated_keys` を足し、画面側は `DEFAULT_UNTRANSLATED_KEYS` (空) を送る。形式の選択欄は TASK-17 なので、画面は当面 "text" を送る。

## 着手時に決めたこと (2026-11-09 オーナー判断)

- AC #2 の「設定で指定できる」は、仕組み (translate が一覧を受け取り、画面側が既定値を 1 か所に持つ) までをこのタスクで作り、設定画面での指定は TASK-21 に AC #5 として足した。
- 翻訳しないキーはキー名の完全一致で、どの深さにも一致する。一致したキーの値が配列・オブジェクトなら中の文字列もすべて訳さない。照合はエスケープを解いたキー名で行う。
- プレースホルダー (`{name}`・`%s` など) の保護は TASK-28 に分けた。
- doc-1 §7.1 に「JSON (TASK-15)」の節を足した。

## 確かめたこと

- `cargo test` 133 件 (うち新規は json 11・document 4)、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`pnpm test` 44 件、`pnpm check`、`pnpm lint` が通る。
- AC #1・#3: `document::tests::json_keeps_its_structure_and_key_order_and_translates_only_string_values` が、入れ子・配列・数値 (`1.50`)・真偽値・null・空文字列・数字だけの文字列・翻訳しないキーを含む文書の訳文を原文と同じ並びの期待文字列と完全一致で比べ (キーの順序を含む)、serde_json で読めて値の型が保たれることを確かめる。`"`・`\`・改行・タブを含む訳文のエスケープも同じテストで見ている。
- AC #2: `json::tests::untranslated_keys_keep_their_values_at_any_depth` ほか 2 件 (値の中の入れ子・エスケープしたキー名・大文字小文字と部分一致を区別すること)。
- 実機 (MacBook Air M2・2B Q4_K_M、アプリと同じ引数の llama-server に一時テストから接続、翻訳しないキーは id と key):
  - 英日: i18n 風の 11 翻訳単位を 2.9 秒、日英: 5 翻訳単位を 1.5 秒で訳した。どちらも訳文が JSON として読め、キーの順序・型・入れ子が原文と同じで、`id`・`key` の値は原文のまま残った。改行を含む値は `\n` のまま戻った。
  - `{name}`・`%d`・`{minutes}` は残ったが、日英で「{minutes} 分たったら」が「after {minutes} from」になり、単位の語が落ちた (TASK-28 の材料)。
- 測っていないこと: アプリの画面での JSON の翻訳 (形式の選択欄が TASK-17 のため、command と画面の受け渡しは単体テストのみ)、大きな JSON (数百キー) の所要時間、0.8B と 8B、Windows (TASK-18)。
<!-- SECTION:NOTES:END -->
