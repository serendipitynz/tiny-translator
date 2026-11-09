---
id: TASK-18
title: 構造保持翻訳を Windows で確かめる
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-09 05:40'
labels: []
milestone: m-3
dependencies:
  - TASK-17
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
解析と再構成は Rust で OS に依存しないはずだが、改行コード (CRLF) とクリップボードの形式で差が出うる。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Windows で Markdown・JSON・HTML の構造保持翻訳が macOS と同じ結果になる
- [x] #2 CRLF の原文で構造が崩れないことを確かめている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 調査の結果: CRLF が届くのは、クリップボードを読む 2 つの経路 (貼り付けボタンの paste_clipboard と起動時のクリップボード翻訳の launch_clipboard) だけ。原文欄にキー入力・Ctrl+V で入れた文字列は textarea の仕様で LF になる。CRLF の原文でも 3 形式とも構造は崩れないが、テキストの段落と HTML の属性値ではモデルへの入力に CR が混ざり、出力は原文から写した部分が CRLF、訳文の中が LF の混在になる (一時テストで確認)
2. (オーナー判断 2026-11-09) クリップボードから取り込むときに CRLF と単独の CR を LF にしてから画面へ渡す。原文欄に入力したときと同じ状態になり、解析には LF だけが届く。訳文のコピーは LF のまま。CRLF を保って訳文も CRLF で返す案は、原文欄を編集すると LF になり一貫しないので採らない
3. clipboard.rs に改行を LF にする関数とテストを足し、lib.rs の 2 つの経路で使う。doc-1 §7.1 に記録する
4. (オーナー判断 2026-11-09) AC#1 は Windows の実測で比べる: cargo test 全件が Windows で通ることで解析と再構成が OS によらないことを示し、実機の 2B で Markdown・JSON・HTML の英日・日英を一時テストから訳して、タグの保持と訳し直しの回数を doc-1 §7.1 の macOS の実測と比べる
5. 実アプリ (tauri dev) で、CRLF の Markdown・JSON・HTML をクリップボードに置き、貼り付けボタンと起動時のクリップボード翻訳から形式の自動判定と翻訳を通し、訳文に CR が無く構造が保たれることを確かめる
6. Windows で pnpm check / lint / test、cargo fmt --check / clippy --all-targets / test を通す
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 判断
- CRLF が届く経路を調べると、クリップボードを読む 2 つのコマンド (貼り付けボタンの paste_clipboard と起動時のクリップボード翻訳の launch_clipboard) だけだった。画面はクリップボードの文字列をそのまま原文の状態に入れて翻訳する。原文欄にキー入力や Ctrl+V で入れた文字列は textarea の仕様で LF になる。
- CRLF の原文を一時テストで 4 形式に通すと、翻訳単位と出力は CR を除けば LF の原文と同じで、構造は崩れなかった。ただしテキストの段落と HTML の属性値では CR が翻訳単位に入ってモデルへ渡り、出力は原文から写した部分が CRLF、訳文の中が LF の混在になった。
- 直し方はクリップボードから取り込むときに CRLF と単独の CR を LF にする形にした (オーナー判断 2026-11-09)。原文欄に入力したときと同じ原文になり、解析には LF だけが届く。訳文は LF のままコピーする。原文が CRLF なら訳文も CRLF で返す案は、原文欄を編集すると LF になって一貫しないので採らなかった。正規化は lib.rs の read_clipboard の 1 か所で行い、起動時のクリップボード翻訳が覚える digest も LF にした文字列から取る (以前 CRLF のまま覚えた原文は、更新後に 1 回だけ訳し直される)。
- AC#1 は Windows の実測で比べた (オーナー判断 2026-11-09)。macOS で訳したときの文書は残っていないので、同じ種類の文書を Windows で訳し、タグの保持と訳し直しの回数を doc-1 §7.1 の macOS の実測と比べた。
- doc-1 は Windows のコマンドライン長の上限 (約 32K 文字) で backlog doc update --content に全文を渡せなかったため、backlog browser の API (PUT /api/docs/doc-1) で更新した。この PC の CLI (1.55.0) は updated_date を ISO 形式 ('2026-11-09T05:16:00Z') で書く。

## 確認 (この PC: Windows 11 Home 10.0.26200、Ryzen 7 5800X、RTX 5060 Ti 16 GB)
- AC#1 (OS によらない部分): cargo test 154 件が Windows で通った。うち Markdown・JSON・HTML・タグ・分割・形式の判定・document の単体テストは 118 件で、OS で分けているテストは無い (cfg(unix) は engine.rs のテストだけ)。
- AC#1 (実機): Vulkan 版 llama-server b12110 と 2B Q4_K_M をアプリと同じ引数 (-c 8192 --parallel 1 --cache-ram 0 --no-webui) で起動し、一時テストから document::translate で訳した。文書は Markdown (front matter・アラート・見出し・番号付きリスト・続きの行を持つ項目・表・コードフェンス・引用・裸の URL・ハードブレーク・取り消し線・参照リンク)、JSON (入れ子・配列・\n を含む値・数値と真偽値と null・末尾に空白のある値)、HTML (doctype・title・h1・改行を含む p・a・em・strong・kbd・code・abbr title・img alt・表・samp・br・pre・script) の英語版と日本語版で、英日・日英に訳した。
  - 翻訳単位 80 件 (Markdown 15×2、JSON 10×2、HTML 15×2)、うちタグ付き 32 件。タグ付きの 32 件すべてでタグの数と番号が保たれ、タグを外した訳し直しは 0 回だった (プロンプトの数は翻訳単位の数と abbr の title の 1 件ずつだけ)。29 件はタグの順序も原文と同じで、3 件 (en.md・ja.md・en.html で 1 件ずつ) は語順に合わせて入れ替わり、すべて元の要素へ戻せた。doc-1 の macOS の実測 (タグ付き 20 件と 62 件ですべて保持、訳し直し 0 回) と同じ結果。
  - doc-1 §7.1 の HTML の節にある入れ替わりの例 (Read <a i="1">the guide</a> before you <em i="2">install</em> the app.) は、macOS と一字一句同じ訳文になった。
  - 訳文を同じ形式で読み直し、原文から写す部分 (Markdown・HTML は翻訳単位の外の文字列、JSON はキーと値の型) が原文と同じことを確かめた。所要時間は 1 文書 0.6〜3.1 秒。
- AC#2: 上の 6 文書を CRLF にしてから取り込みと同じ正規化を通すと LF の原文と一致し、CRLF のまま形式を判定しても 6 文書とも正しい形式になった。実アプリ (pnpm tauri dev、WebView2 のリモートデバッグで画面の値を読み、ボタンを押した) では、PowerShell の Set-Clipboard で CRLF にした文書をクリップボードに置き、Markdown (英) を起動時のクリップボード翻訳で、JSON (英)・HTML (日)・Markdown (日) を貼り付けボタンで取り込んで訳した。4 件とも形式は自動で判定され (自動 (Markdown) / 自動 (JSON) / 自動 (HTML))、訳文に CR は無く、一時テストと同じ訳文になった。訳文をコピーすると、クリップボードには LF の改行 (CRLF 0 件) で入った。
- pnpm check (0 件)・pnpm lint・pnpm test (47 件)・cargo fmt --check・cargo clippy --all-targets -- -D warnings・cargo test (154 件) が Windows で通った。

## 見ていないこと
- 0.8B と 8B での構造保持翻訳。
- コピーした LF の訳文を、LF を改行として扱わない古い Windows のアプリに貼ったときの表示 (メモ帳は 2018 年から LF を読む)。
- 原文の改行を変えたことの macOS 側の動作 (macOS のクリップボードは LF なので、正規化は何もしないはず)。
<!-- SECTION:NOTES:END -->
