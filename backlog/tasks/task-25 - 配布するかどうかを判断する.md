---
id: TASK-25
title: 配布するかどうかを判断する
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-10 09:18'
labels: []
milestone: m-5
dependencies:
  - TASK-23
documentation:
  - backlog/docs/doc-2 - 先行事例と差別化.md
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
先行事例 tanakh/plamo-translate-desktop が macOS に対応すると、配布の価値は小さくなる (doc-2 §3)。判断の時点で先行事例を調べ直す。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 先行事例の macOS 対応と機能の状況を調べ直し、doc-2 を更新している
- [x] #2 配布する・しない・先行事例へ機能を提案する、のどれにするかをオーナーが決め、記録している
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-11-10 に先行事例を調べ直し、doc-2 を書き換えた。tanakh 版は README・docs/design.md・全リリースノートを GitHub API で読み、v0.4.0 (2026-11-07) で macOS (Apple Silicon) の dmg を配布し始めたことを確かめた。設計書 §1 は macOS を Non-goals に残したままだが、実態は 3 OS 対応である。
- 他のアプリは GitHub のリポジトリ検索 (2026-11-01 以降に作成、「plamo translate」で 75 件) から形態ごとに主なものを doc-2 §1.4 に挙げた。macOS で単体で動くものは README を読んで機能を確かめた。配布物を実際に動かしてはいない。
- doc-2 §2 は 11-06 と 11-10 の評価を並べる形にした。macOS 単体で動く点は失い、残る差は構造保持翻訳・履歴・翻訳先の優先順位・起動時のクリップボード翻訳・自動解放の機能差だけ。
- AC#2: オーナーが「配布する」と判断した (2026-11-10)。自分用に作るアプリを配布するのは snz-studio・mallow と同じ方針で、先行事例との差の大小は理由にしない。doc-2 §3 に記録した。後続は TASK-26。

- 配布物には macOS の署名・公証 (TASK-26 AC#1 にある) と自動更新を入れるとオーナーが決めた。自動更新はどのタスクにも無かったので TASK-30 を作った (m-5、TASK-26 に依存)。
<!-- SECTION:NOTES:END -->
