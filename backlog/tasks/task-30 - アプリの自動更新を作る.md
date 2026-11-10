---
id: TASK-30
title: アプリの自動更新を作る
status: To Do
assignee: []
created_date: '2026-11-10 08:10'
updated_date: '2026-11-10 08:15'
labels: []
milestone: m-5
dependencies:
  - TASK-26
documentation:
  - backlog/docs/doc-2 - 先行事例と差別化.md
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
配布すると決めたとき (TASK-25)、オーナーは配布物に自動更新を入れると決めた。配布したアプリは利用者が新しい版を手で入れ直さない限り古いまま残り、不具合の修正もモデルの扱いの変更も届かない。tanakh 版は v0.5.0 で、GitHub に新しい版を 1 日 1 回まで問い合わせ、SHA-256 を照合して入れ替える自動更新を入れた (doc-2 §1.1)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 macOS と Windows の配布物で、新しい版が公開されるとアプリが画面で知らせ、利用者の操作で更新して再起動できる
- [ ] #2 更新物の真正性を検証し、検証できないものは適用しない
- [ ] #3 新しい版の確認を設定で止められ、ネットワークが無いときは翻訳を妨げず、エラーも表示しない
- [ ] #4 新しい版を公開する手順 (更新物の作成と置き場所) を README か doc-1 に書いている
- [ ] #5 新しい版を確かめる通信を、R1 (モデル取得後はネットワークを使わない) の例外として、既定で有効か無効かと合わせて doc-1 に記録している
<!-- AC:END -->
