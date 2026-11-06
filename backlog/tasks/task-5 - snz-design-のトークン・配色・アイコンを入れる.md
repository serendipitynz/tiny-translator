---
id: TASK-5
title: snz-design のトークン・配色・アイコンを入れる
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-06 22:53'
labels: []
milestone: m-1
dependencies:
  - TASK-4
references:
  - ../snz-design/tokens/README.md
  - ../snz-design/backlog/docs/doc-7 - テーマ切替と既存設定移行の共通仕様.md
  - ../snz-design/backlog/docs/doc-16 - Web系4アプリへの共通デザイン適用ガイド.md
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
見た目は snz-design に従う (R9)。トークンの写し方、配色の 2 軸、最初の描画より前の適用は snz-design の仕様が決めている。backlog-atlas は配色の適用が最初の描画より後でちらつくので、その形を避ける (doc-1 §8.3)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tokens-v0.1.1 の snz-tokens.css を vendor.mjs copy で写し、vendor.mjs verify が通る
- [x] #2 配色系統 (標準・Solarized) と明暗の選択 (Light・Dark・OS 追従) を保存値から読み、最初の描画より前に data-color-family と data-color-mode を設定している
- [x] #3 OS 追従のとき、OS の明暗の変更に再起動なしで追う
- [x] #4 lucide の座標をコピーした Icon コンポーネントがあり、lucide のライセンス表示を同梱している
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. vendor.mjs copy 0.1.1 で snz-tokens.css を src/styles/snz-tokens.css へ写し、verify を通す。Biome の対象から写しを外す。
2. index.html の head に、最初の描画より前に走るスクリプトを置く。localStorage の colorFamily (standard / solarized) と colorMode (light / dark / auto) を読み、どちらかが取り得る値でなければ初期値 (標準 + OS 追従) で描く (snz-design doc-7 §4.3・§5)。auto は prefers-color-scheme で light / dark に解決して <html> の data-color-family / data-color-mode に置く。
3. 同じスクリプトで、選択が auto のときだけ prefers-color-scheme の change に追って data-color-mode を置き直す (doc-7 §4.5)。
4. トークンの写しと、body の面・文字色を当てる基本のスタイルを index.html から読み込み、最初の描画に間に合わせる。
5. src/lib/icons/ に lucide-react v1.17.0 の __iconNode を写した lucide.ts と Icon.svelte を置く。図形は doc-1 §8.2 の操作 (履歴・設定・貼り付け・コピー・クリア・入れ替え) の分。lucide-react v1.17.0 の LICENSE を書き写さずにそのまま保存する (snz-design doc-15 §3.2)。
6. 起動スクリプトを index.html から取り出して偽の localStorage / matchMedia で走らせる vitest と、図形の属性の vitest を書く。ブラウザで 4 配色と OS 追従の追従を確かめる。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 実装の要点

- 保存値は localStorage の `colorFamily` (`standard` / `solarized`) と `colorMode` (`light` / `dark` / `auto`) の 2 キー。キー名は mallow と揃えた。欠けたキーはその軸の初期値 (標準・OS 追従) に読むので、TASK-21 の設定画面は選んだ軸だけを書いてよい (snz-design doc-16 §6.2)。どちらかの軸が取り得る値でなければ、組全体を初期値で描き、保存値は書き換えない (doc-7 §4.3・§5.1)。
- 起動スクリプトは `index.html` の head に置いたインラインの 1 か所だけで、OS の明暗の追従もここで持つ。アプリ本体に同じ規則の写しは置いていない。TASK-21 で設定画面から選び直すときは、このスクリプトが閉じ込めている選択を更新する経路が要る (serenebach のように関数を window に出す形か、写しを持つ形かは TASK-21 で決める)。
- 標準・Solarized はどちらも明暗の両側を持つので、doc-7 §4.3 の「系統が持つ側で描く」分岐は入れていない (理由をスクリプトのコメントに書いた)。
- トークンの写しと本体のスタイル (`src/styles/global.scss`。面に canvas、文字に fg) は main.ts から import せず `index.html` の `<link>` で読む。head の stylesheet は最初の描画を止めるが、モジュールから読むと開発時に最初の描画の後に届くため。本番ビルドでは 1 つの CSS にまとまり head に入る。
- Tauri の CSP (`script-src 'self'`) でインラインスクリプトが止まらないことは、tauri-codegen 2.7.4 の `inject_script_hashes` が本番ビルドで インラインスクリプトの sha256 を CSP に足すことをソースで確かめた。mallow も同じ形で動いている。
- アイコンは lucide-react v1.17.0 (decision-1 が固定した版) の `__iconNode` を `key` だけ除いてそのまま写した。backlog-atlas の `IconShape` の union と `drawnShape` の switch は持たず、`[タグ, 属性]` の組をそのまま `<svelte:element>` に広げる。属性を個別に写す層が無いので、属性の落ちを検査する試験も要らない。表示寸法は `--snz-icon-size-sm` / `-md` (16px / 20px、doc-8 §6.2) を `size` で選ぶ。図形は常に `aria-hidden` で、読み上げの語は置いた側のボタンが持つ (doc-8 §6.6)。
- 写した図形は doc-1 §8.2 の操作の分: 履歴 `history`、設定 `settings`、貼り付け `clipboard-paste`、コピー `copy`、クリア `x`、入れ替え `arrow-left-right`。入れ替えは doc-1 §8.1 の略図では ⇅ だが、入れ替える 2 つの選択欄が左右に並ぶので左右の矢印にした。TASK-6 で画面に置くときに見直してよい。
- lucide の LICENSE は同じ版の配布物から書き写さずに `src/lib/icons/LICENSE-lucide` として保存した (snz-design doc-15 §3.2)。ISC 条文に加え Feather 由来の図形の MIT 条文を持ち、`x` がそれに当たる。配布物の第三者表記へ載せるのは TASK-26 の AC#3。

## 確かめたこと

- AC#1: `node ../snz-design/tokens/vendor.mjs copy 0.1.1 snz-tokens.css src/styles/snz-tokens.css` の後、`verify` が「tokens-v0.1.1 と一致」を返した。写しは Biome の対象から外した。
- AC#2: `src/color-scheme.test.ts` (8 件) が `index.html` からスクリプトを取り出し、偽の localStorage と matchMedia で走らせる。未保存・2 軸の全組・欠けたキー・範囲外の値・読めない保存先・保存先へ書かないことを確かめる。変異を 4 つ入れて (検証を外す、change の購読先を変える、明示の選択でも OS を読む など) 意味のある 3 つが失敗することを見た。ブラウザ (Vite の開発サーバー) で 4 配色を保存値から描き、`<html>` の属性と canvas の色が各配色の値になることを見た。
- AC#4: `Icon.svelte` で 6 図形を 2 寸法 (20px・16px) で描き、Solarized Light / Dark で図形の色が `--snz-figure` になることをブラウザで見た (確認用の表示は戻した)。
- `pnpm test` / `pnpm check` / `pnpm lint` / `pnpm build` が通る。Rust 側は変更していない。

## 確かめていないこと

- AC#3 は未チェック。ブラウザで prefers-color-scheme を Light → Dark へ切り替え、再読み込みなしで `data-color-mode` と canvas が追うことは見た (Light・Dark を明示したときは追わない)。macOS の外観を実際に切り替えて Tauri の WKWebView が追うことは、システム設定を変えることになるため AI 側では行っていない。`pnpm tauri dev` で OS の外観を切り替えて確かめてほしい。
- Tauri の WebView の中での最初の描画 (保存値ありの起動でちらつかないこと) は見ていない。属性は head の同期スクリプトが body より前に置くので、構造上は最初の描画より前になる。

## レビュー対応 (PR#5 の 1 回目、P3)

- 図形の色が置いた先の文字色 (`--snz-fg`) を継いでおり、doc-8 §6.6 の既定 `--snz-figure` と違っていた。`Icon.svelte` の既定を `var(--icon-color, var(--snz-figure))` にした。面を持つ部品 (主操作のボタンなど) は `--icon-color` に面の上の色 (`--snz-on-accent` など) を渡す。holder の `color` で渡す形にしなかったのは、scoped のスタイルの内側へ `:global` で手を入れずに済むため。
- 4 配色すべてで、何も渡さない図形が `--snz-figure` の値になり、`--icon-color: var(--snz-on-accent)` を渡した面の上では `--snz-on-accent` の値になることをブラウザで確かめた (確認用の表示は戻した)。

## マージ後

- AC#3: オーナーが `pnpm tauri dev` で macOS の外観をライト・ダークに切り替え、再起動なしで背景 (canvas) が追うことを Tauri の WebView で確かめた (2026-11-07)。
- PR#5 でマージした。
<!-- SECTION:NOTES:END -->
