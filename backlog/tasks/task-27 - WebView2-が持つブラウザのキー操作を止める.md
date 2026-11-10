---
id: TASK-27
title: WebView2 が持つブラウザのキー操作を止める
status: Done
assignee: []
created_date: '2026-11-07 00:44'
updated_date: '2026-11-10 19:28'
labels: []
milestone: m-2
dependencies:
  - TASK-6
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-7 で Windows (WebView2) の pnpm tauri dev を確かめたところ、F5 と Ctrl+R で画面が再読み込みされ、Ctrl+P で印刷プレビュー、Ctrl+F で検索バーが開いた。macOS の WKWebView には無い挙動である。今は入力しか状態が無いので実害は小さいが、翻訳ができるようになると F5 一つで原文・訳文・進行中の翻訳が消える。Tauri 2.12 は WebView2 の AreBrowserAcceleratorKeysEnabled を設定から変えられないので、止め方も含めて決める。mallow (TASK-27) は Ctrl+P を、印刷できない画面でもアプリ全体の keydown で常に握りつぶすことで止めている。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Windows で F5・Ctrl+R・Ctrl+Shift+R を押しても画面が再読み込みされず、入力中の原文が残る
- [x] #2 Windows で Ctrl+P を押しても印刷プレビューが開かない
- [x] #3 原文の入力欄で編集のキー (Ctrl+C・Ctrl+V・Ctrl+X・Ctrl+Z・Ctrl+A) と右クリックの切り取り・コピー・貼り付けがそのまま使える
- [x] #4 Ctrl+F の検索バーと右クリックメニューのそれ以外の項目を残すか消すかを決め、理由を Implementation Notes に残している
- [x] #5 macOS でも同じ操作 (⌘R・⌘P) で再読み込みや印刷が起きないことを確かめている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 止め方は Rust 側で WebView2 の AreBrowserAcceleratorKeysEnabled を false にする (2026-11-11 オーナー判断)。Tauri 2.12 の設定にも WebviewWindowBuilder にも入口が無い (wry の with_browser_accelerator_keys は Tauri から渡せない) ので、setup で main ウィンドウの with_webview から ICoreWebView2Settings3 を取って切る。JS の keydown で列挙して止める案は、知らないブラウザ用のキーが漏れるので採らない。webview2-com 0.39 と windows-core 0.62 を Windows 限定の直接依存に足す (どちらも wry 経由で既にロックファイルにある版)。
2. Ctrl+F・F3 の検索バーは消す (同上の設定で一緒に止まる)。右クリックメニューは文字を入力・選択できる欄 (textarea と文字入力の input) の中だけ残し、それ以外では contextmenu を preventDefault する (2026-11-11 オーナー判断)。入力欄の外のメニューには再読み込みがあり、F5 と同じく原文が消えるため。AreDefaultContextMenusEnabled を切ると入力欄の編集メニューも消えて AC#3 に反するので、こちらは JS で行う。判定は src/lib の純粋関数にして vitest で試す。
3. MS のドキュメントのサンプルには、設定の変更は次のナビゲーションから効くとある。setup の時点で最初のナビゲーションは始まっているので、起動直後の画面に効くかは Windows で確かめ、効かなければ止め方を見直す。
4. Windows の確認 (AC#1〜#3、F12 が dev でも効かなくなることを含む) はオーナーが手元の Windows で pnpm tauri dev を動かして行う。確認表を用意する。macOS の ⌘R・⌘P と右クリック (AC#5) はこの Mac で確かめる。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 止め方 (2026-11-11 オーナー判断)

- Windows では起動時に WebView2 の `AreBrowserAcceleratorKeysEnabled` を false にする (`src-tauri/src/lib.rs` の `disable_browser_keys`)。Tauri 2.12.5 は設定からも `WebviewWindowBuilder` からもこの値を渡せない (wry 0.57.1 には `with_browser_accelerator_keys` があるが、tauri-runtime-wry 2.12.3 は渡していない)。そこで setup で main ウィンドウの `with_webview` から `ICoreWebView2Settings3` を取り、直接切る。
- 画面の keydown で F5・Ctrl+R・Ctrl+P を列挙して止める案 (mallow の Ctrl+P と同じ形) は採らなかった。MS のドキュメントはこの設定が止めるキーを "including but not limited to" と書いており、列挙では Alt+← (戻る) のような知らないキーが漏れるため。
- 依存に `webview2-com` 0.39 と `windows-core` 0.62 を Windows 限定で足した。どちらも wry 経由で既に Cargo.lock にある版なので、取り込むコードは増えない (Cargo.lock の差分はこの 2 つを直接依存に並べた 2 行だけ)。

## AC#4 の判断 (2026-11-11 オーナー判断)

- Ctrl+F・F3 の検索バーは消す。macOS の WKWebView には無く、doc-1 §8 の画面にも無い。上の設定で一緒に止まる。doc-1 §8.4 に「初版で置かないもの」として書いた。
- 右クリックメニューは、文字を持つ入力欄 (`textarea` と `type="text"` の `input`) の中だけ残す。それ以外の場所では `contextmenu` を `preventDefault` する (`src/lib/contextmenu.ts`)。入力欄の外のメニューには再読み込み・戻る・印刷があり、再読み込みは F5 と同じく原文を消すため。`AreDefaultContextMenusEnabled` を切ると入力欄の切り取り・コピー・貼り付けも消えて AC#3 に反するので、こちらは画面側で止める。
- doc-1 §8.1 に両方の判断を書いた。

## 確かめたこと (macOS、2026-11-11)

- `pnpm run check`・`pnpm run lint`・`biome check src`・`pnpm run test` (103 件)、`cargo fmt --check`・`cargo clippy --all-targets`・`cargo test` (218 件) が通った。
- この Mac では Windows ターゲットのアプリ全体をビルドできない (rusqlite が同梱する C コードに Windows SDK のヘッダーが要る)。そこで `disable_browser_keys` だけを tauri 2.12.5・webview2-com 0.39.1 に依存する別のクレートへ写し、`cargo clippy --target x86_64-pc-windows-msvc` で警告ゼロを確かめた。わざと引数の型を誤らせるとエラーになったので、Windows 向けに型が見られていることも確かめた。

## まだ確かめていないこと

- **設定がいつ効くか**: MS のドキュメントのサンプルには「変更は次のナビゲーションから効く」とある。setup の時点で最初のナビゲーションは始まっている可能性があるので、起動直後の画面で F5 が止まるかを Windows で確かめる。止まらなければ止め方を見直す。
- AC#1〜#3 は Windows の実機、AC#5 は macOS の実機で、オーナーが次の確認表で確かめる。macOS の確認を AI が osascript のキー送信で試したところ、別のアプリのウィンドウが重なっていてキーがそちらへ渡ったので、自動操作はやめた。

## 確認表

Windows (`pnpm tauri dev`)。原文欄に文字を入れて、末尾に目印の語を足してから始める:

1. F5・Ctrl+R・Ctrl+Shift+R を押す: 画面が再読み込みされず、目印の語が原文に残る (AC#1)。起動直後に最初に押す回も含める
2. Ctrl+P: 印刷プレビューが開かない (AC#2)
3. Ctrl+F・F3: 検索バーが開かない (AC#4)
4. 原文欄で Ctrl+A・Ctrl+C・Ctrl+X・Ctrl+V・Ctrl+Z が効く (AC#3)
5. 原文欄の右クリック: メニューが開き、切り取り・コピー・貼り付けが効く。メニューに並ぶ項目を記録する (AC#3・AC#4)
6. 訳文欄の右クリック: メニューが開き、コピーが効く
7. 入力欄の外 (見出し・余白・ボタン) の右クリック: メニューが開かない (AC#4)
8. Ctrl+Enter の翻訳と Ctrl+Shift+C の訳文コピーがこれまでどおり効く
9. 参考: F12 で DevTools が開かなくなる (dev でも同じ)。入力欄の右クリックメニューの「検査」から開けるかを記録する

macOS (`pnpm tauri dev`):

10. ⌘R・⌘⇧R を押しても原文の目印が残り、⌘P で印刷のシートが出ない (AC#5)
11. 入力欄の外の右クリックでメニューが開かず、原文欄の右クリックでは切り取り・コピー・貼り付けがあるメニューが開く

## 実機での確認 (2026-11-11、オーナー)

ブランチ TASK-27-webview-browser-keys (c2c13c0) を `pnpm tauri dev` で動かし、確認表の 1〜11 を試した。

- Windows: 1〜8 はすべて期待どおりだった。F5・Ctrl+R・Ctrl+Shift+R を押しても再読み込みされず、目印の語が原文に残った。起動直後に最初に押した回も同じだった。そのため、「まだ確かめていないこと」に挙げた「次のナビゲーションから効く」懸念は当たらず、setup で切る形のままでよい。Ctrl+P の印刷プレビュー、Ctrl+F・F3 の検索バーは開かなかった。原文欄の編集キーと右クリックの切り取り・コピー・貼り付け、訳文欄の右クリックのコピーは効いた。入力欄の外ではメニューが開かなかった。Ctrl+Enter と Ctrl+Shift+C もこれまでどおり効いた。
- Windows の 9: dev でも F12 で DevTools は開かなくなった。入力欄の右クリックメニューの「検査」からは開ける。
- macOS: ⌘R・⌘⇧R で原文が残り、⌘P で印刷のシートは出なかった (10)。入力欄の外の右クリックではメニューが開かず、原文欄では開いた (11)。
- macOS の原文欄のメニューに並んだのは OS 標準の文字入力欄の項目である: 調べる・翻訳・Google で検索・切り取り・コピー・貼り付け・スペルと文法・自動置換・変換・フォント・スピーチ・段落の方向・選択の方向・共有・自動入力・サービス、dev では Inspect Element も出る。再読み込み・戻る・印刷は無いので原文を消す経路にはならず、このまま残す。項目を一つずつ削るには WKWebView のメニューをネイティブ側で書き換える処理が要り、「調べる」「翻訳」は翻訳アプリではむしろ役に立つためである。Inspect Element は `tauri` に `devtools` 機能を付けていないのでリリースビルドには出ない。Windows の原文欄のメニューの項目は一覧として記録していない。
<!-- SECTION:NOTES:END -->
