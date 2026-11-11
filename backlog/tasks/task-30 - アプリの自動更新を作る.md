---
id: TASK-30
title: アプリの自動更新を作る
status: In Review
assignee: []
created_date: '2026-11-10 08:10'
updated_date: '2026-11-11 00:16'
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
- [x] #2 更新物の真正性を検証し、検証できないものは適用しない
- [x] #3 新しい版の確認を設定で止められ、ネットワークが無いときは翻訳を妨げず、エラーも表示しない
- [x] #4 新しい版を公開する手順 (更新物の作成と置き場所) を README か doc-1 に書いている
- [x] #5 新しい版を確かめる通信を、R1 (モデル取得後はネットワークを使わない) の例外として、既定で有効か無効かと合わせて doc-1 に記録している
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
2026-11-11 オーナー承認: 本番依存は tauri-plugin-updater (Rust) だけ足す。更新の確認は既定で有効。署名鍵 (minisign) はオーナーが手元で作って保管し、秘密鍵はリポジトリシークレットに置く。mallow の作りを先例にするが、画面は Rust のコマンドだけを使う形に合わせる。
1. Rust: tauri-plugin-updater を登録し、update.rs に確認の間隔 (起動時に 1 日 1 回まで)・状態 (なし・あり・ダウンロード中・入れ替え中・失敗) を置く。コマンドは update_state・update_check (起動時か手動か)・update_install・update_dismiss。署名の検証はプラグインのダウンロード内で行われ、検証できないものは入れ替えない。Windows では on_before_exit で翻訳エンジンを止める (インストーラーが llama-server.exe を置き換えられるように)
2. 設定: settings.json に updateCheck (既定 true) を足す。設定画面に「更新」の区画 (今の版・自動で確認する・今すぐ確認)
3. 画面: 新しい版があればメインのお知らせで知らせ、「更新して再起動」で更新する。ダウンロードの進捗・失敗 (もう一度試す・閉じる) も同じ場所に出す。起動時の確認の失敗は何も出さない
4. release.yml: tauri.release.conf.json で createUpdaterArtifacts を有効にし (手元のビルドに秘密鍵を要求しないため)、署名済みの .app.tar.gz と NSIS とその .sig を下書きへ置く。最後のジョブで minisign により tauri.conf.json の公開鍵で署名を検証してから latest.json を書く (scripts/write-updater-manifest.py)
5. README にリリース手順 (鍵の作成・シークレット・公開)、doc-1 に自動更新の節と R1 の例外を書く
6. THIRD-PARTY-LICENSES.txt を作り直す。pnpm check / lint / test、cargo fmt / clippy / test
7. 手元の macOS で、テスト用の鍵とローカルの更新サーバーで 0.1.0 → 0.1.1 の更新と、改ざんした更新物を拒むことを確かめる (クリックはオーナー)。Windows での確認と GitHub 上の実物での確認はオーナーと行う
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 本番の依存に足したのは Rust のクレート tauri-plugin-updater (2.14.1) だけ (2026-11-11 オーナー承認)。画面は Rust のコマンドしか呼ばないので、npm パッケージと tauri-plugin-process は足していない。プラグインが reqwest・rustls を持ち込み、クレートは 302 から 358 に増えた。モデルの取得は curl のまま (doc-1 §14.5)。
- 起動時の確認は既定で有効 (2026-11-11 オーナー判断)。最後に確認が決着してから 24 時間は行わない。決着の時刻 (`update-checked`) を書くのは、新しいバージョンが無いと分かったときと、案内を閉じたときだけにした。見つかったときにも書くと、再起動した後の 24 時間は案内が消えてしまうことが手元の試験で分かったため。
- `requireSignedVersion` を有効にした。バンドラーが署名の信頼済みコメントに `version:` を書くことは、tauri-cli 2.12.4 のソース (`sign_updaters` → `sign_file(.., Some(version))`) と実際の署名で確かめた。`scripts/write-updater-manifest.py` も、署名のバージョンがタグと一致しなければ `latest.json` を書かない。
- `createUpdaterArtifacts` はリリースのワークフローだけが `src-tauri/tauri.release.conf.json` で有効にする。手元のビルドに秘密鍵を要らなくするため。
- Windows では、インストーラーに切り替わる直前 (`on_before_exit`) に翻訳エンジンを止め、モデルの入手も取り消す。翻訳エンジンはジョブオブジェクトに入っていないので、アプリの終了だけでは `llama-server.exe` が残り、上書きできないため。
- 「版」と書いていた箇所 (画面・README・doc-1・doc-2) は、ソフトウェアのバージョンを指すものを「バージョン」に直した (2026-11-11 オーナー指摘)。
- 検証 (2026-11-11、macOS M2、テスト用の鍵・識別子とローカルの HTTP サーバー):
  - AC#1 の macOS 側: 0.1.0 が起動時の確認で 0.1.1 を案内し、「更新して再起動」で 0.1.1 に入れ替わって再起動した。
  - AC#2: `latest.json` のバージョンを 0.1.2 と偽ると「署名のバージョンと一致しない」として拒み、更新物の 1 バイトを書き換えると「署名の検証に失敗」として拒んだ。どちらも 0.1.0 のまま。
  - AC#3: `updateCheck: false` で起動するとサーバーへの要求は無く、true では 1 回あった。サーバーを止めて起動すると、画面には何も出ず翻訳もできた (オーナーが画面で確認)。間隔の記録は書かれなかった。
  - AC#4: README のリリースの節と doc-1 §14.4。AC#5: doc-1 §2 の R1 の例外の段落と §14.1。
  - `write-updater-manifest.py` を minisign 0.12 で試した。正しい鍵なら両 OS 分の `latest.json` を書き、鍵違い・バージョン違い・バージョンの無い署名では失敗した。
  - pnpm test (108)、pnpm check、pnpm lint、cargo clippy -D warnings、cargo test (236) が通る。
- 未確認 (AC#1 を未チェックにしている理由): Windows での更新と、GitHub の公開済みのリリースからの更新。リリースのワークフローの更新物の作成と latest.json の生成も、最初のタグで初めて動く。前提として、オーナーが `TAURI_SIGNING_PRIVATE_KEY`・`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` を登録し、リポジトリを public にしてから最初のタグを打つ。
<!-- SECTION:NOTES:END -->
