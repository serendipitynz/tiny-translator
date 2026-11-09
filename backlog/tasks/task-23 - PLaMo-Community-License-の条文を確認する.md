---
id: TASK-23
title: PLaMo Community License の条文を確認する
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-09 21:11'
labels: []
milestone: m-5
dependencies: []
references:
  - 'https://plamo.preferredai.jp/info/plamo-community-license-ja'
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
  - backlog/docs/doc-2 - 先行事例と差別化.md
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
モデルを同梱するか、ダウンロードさせるか、画面に何を表示する義務があるかは条文次第である。調べた範囲では再配布でライセンスが配布先へ及び、翻訳結果の公開時に PLaMo の出力であることを示す必要がある (doc-2 §3)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 再配布、表示義務 (Built with PLaMo など)、商用利用の登録、出力の利用条件の条文を要約し、doc-1 §10 へ反映している
- [x] #2 アプリが満たすべき表示を一覧にしている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 条文 (2026-11-09 改定版) を Web 版と GGUF リポジトリの LICENSE/ja・en で読み、改定前の版との差分を取る
2. doc-1 §10 を、条文の要約 (商用利用の登録・再配布と表示義務・生成物の利用・その他)、入手方法ごとの義務、アプリが満たすべき表示の一覧に書き直す
3. 改定で古くなった doc-2 §3 のライセンスの記述を直し、doc-1 §11 の未確認の行を確認済みにする
4. 配布が商業目的に当たるか等、TASK-24/25 で決める点は判断せず論点として残す
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 読んだ条文: 2026-11-09 改定版。Web 版 (plamo-community-license-ja) と、pfnet/plamo-3-translate-{0.8b,2b,8b}-GGUF の LICENSE/ja・LICENSE/en (3 サイズで SHA-1 が一致) を突き合わせた。GGUF リポジトリは gated=false (HF API)。
- 改定前の版 (GGUF リポジトリの一つ前のコミット a634a56) と diff を取った。本アプリに効く変更は 2 つ: 第 4 条の対象から生成物が外れたこと、第 5 条 (2) で変換モデル (翻訳を含む) の生成物が明示の義務から外れたこと。タスクの説明と doc-2 §3 の「翻訳結果の公開時に PLaMo の出力であることを示す」は改定前の条文によるもので、長文の公表物を除き当たらなくなったので doc-2 §3 も直した。
- AC#1: doc-1 §10.1 に条文の要約 (商用利用の登録・再配布と表示義務・生成物の利用・その他) を、§10.2 に入手方法ごとの第 4 条の義務を書いた。
- AC#2: doc-1 §10.3 に D1〜D5 の表示を、置き場所・根拠の条文・義務になる条件つきで一覧にした。
- 判断せず残したもの: (1) 初回のダウンロードで配布者が第 4 条の義務を負わないという読みは PFN に確かめていない。そのため D1・D2 はどの入手方法でも出す形にした (§10.2)。(2) 配布が商業目的に当たり登録が要るかは TASK-25 の判断に含めた。(3) D1・D2・D5 を画面へ置く作業はどのタスクにも入っていない。
- コードの変更は無いので、テストと lint は対象外。

- レビュー (PR #23) を受けて: 改定で変わった点に第 5 条 (4) の新設 (生成物を第三者へ渡すときの表示義務。変換モデルの例外なし) を加え、3 つとした。D5 と doc-2 §3 の第 5 条 (2) の条件を「独立した長文の公表物の全部か実質的な部分」と条文どおりにした。
<!-- SECTION:NOTES:END -->
