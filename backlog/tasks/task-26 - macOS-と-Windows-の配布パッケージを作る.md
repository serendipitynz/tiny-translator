---
id: TASK-26
title: macOS と Windows の配布パッケージを作る
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-10 10:04'
labels: []
milestone: m-5
dependencies:
  - TASK-24
  - TASK-25
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
配布すると決めた場合だけ行う。macOS は公証が無いと Gatekeeper で開けない。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 macOS の .dmg を署名・公証して作れる
- [ ] #2 Windows のインストーラーを作れる
- [ ] #3 サードパーティのライセンス表示 (llama.cpp・lucide・依存クレートと npm パッケージ) を同梱している
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
ビルドは GitHub Actions、アイコンは添付画像に余白を付けて使う (2026-11-10 オーナー判断)。公開時にリポジトリは public にする。backlog-atlas のリリースの仕組み (release.yml・署名スクリプト・generate-third-party-licenses.mjs) を移し、tiny-translator の事情 (sidecar の llama-server、Apple Silicon だけ、更新の仕組みは TASK-30) に合わせる。
1. アイコン: 添付画像 (772px、端まで角丸) を 1024px の透明な画像の中央に 824px で置いた src-tauri/app-icon.png を作り、pnpm tauri icon で .icns・.ico・png を作る (モバイル用は消す)。bundle.icon を差し替える
2. llama-server のビルドから llama.cpp の Web UI を外す (LLAMA_USE_PREBUILT_UI=OFF)。アプリは --no-webui で起動しており、今のビルドはビルド時に Hugging Face から UI を取得して埋め込んでいる。外せばバイナリが小さくなり、ライセンス表示の対象から UI の npm パッケージが消える。.sh と .ps1 の両方
3. ライセンス表示: backlog-atlas の generate-third-party-licenses.mjs を移し、THIRD-PARTY-LICENSES.txt を生成してコミットする。npm は本番依存、クレートは aarch64-apple-darwin と x86_64-pc-windows-msvc の和。THIRD-PARTY-NOTICES.md に llama.cpp (ggml を含む) と llama-server に入る vendor のライブラリ (cpp-httplib・nlohmann/json・stb_image・miniaudio・subprocess.h・sha1・sha256・xxhash・rotate-bits)、Windows の Vulkan-Headers、lucide を書き、条文は固定したコミットのファイルから書き写さずに取る。bundle.resources で同梱し、古くなったら落ちるテストを足す。.gitattributes で LF に固定する
4. macOS の署名・公証: scripts/macos-sign-build.sh・macos-verify-gatekeeper.sh・setup-ci-signing-secrets.sh と .env.signing.example を移す。.dmg の公証と staple は Tauri がしないのでスクリプトで行う。この Mac で署名・公証した .dmg を作り、Gatekeeper の判定と、署名したアプリで翻訳エンジン (Metal) が動くことを確かめる
5. .github/workflows/release.yml: v* のタグで下書きのリリースを作り、macOS (aarch64、署名・公証) と Windows (NSIS、署名なし) をビルドして置く。tauri-action は使わず pnpm tauri build と gh release upload で行う (更新物は TASK-30 で足す)。Windows のランナーには Vulkan SDK を固定の版と SHA-256 で入れ、両 OS で build-llama-server を走らせる。バージョンの一致・ライセンス表示の鮮度・同梱物 (アイコン・ライセンス) を検査する
6. targets を app・dmg・nsis に絞る。Windows はコード署名しない (backlog-atlas・mallow と同じ。SmartScreen の警告は README に書く)
7. README に配布物の作り方とリリースの手順、doc-1 に配布の節を書く
8. pnpm check / lint / test、cargo test / clippy。Actions の実行と Windows の実機での確認 (インストール・起動・翻訳・ライセンスファイル・アイコン) はオーナーと行う
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 判断 (2026-11-10 オーナー): 配布物は GitHub Actions で作る。アイコンは渡された画像に余白を付けて使う。リポジトリは公開時に public にする。Windows の作業は必要 (Vulkan 版の翻訳エンジンとインストーラーは Windows でしかビルドできず、インストーラーの実機確認も CI では代わりにならない) と回答した。
- 判断 (AI、先例に従った): Windows はコード署名しない (backlog-atlas・mallow と同じ)。インストーラーは NSIS だけ (bundle.targets を app・dmg・nsis に絞った)。tauri-action は使わず、検査が通ってからアップロードする。理由は doc-1 §13。
- llama-server を Web UI 無しでビルドする形に変えた (.sh と .ps1)。既定のビルドは Hugging Face から Web UI (約 3 MB、npm パッケージ多数) を取得して埋め込んでおり、アプリは --no-webui で使っていない。llama.cpp はビルドディレクトリに残った dist/ を埋め込むので、それを消してからビルドし、ui.cpp に asset が残っていれば失敗させる。macOS 版は 23.3 MB → 20.0 MB。--no-webui で起動し /health・/completion が応答し、/ は 404 になることを確かめた。
- ライセンス表示: backlog-atlas の生成スクリプトを移した。npm 側は --prod ではなくビルドのソースマップから数える形に変えた (svelte は devDependencies だが実行時コードが画面に入るため)。結果は npm 4 件 (svelte・esm-env・clsx・@tauri-apps/api)、クレート 302 件 (aarch64-apple-darwin と x86_64-pc-windows-msvc の和)、688 KB。2 回生成してバイト同一。SPDX の標準の条文 6 件は spdx/license-list-data から取得 (5 件は backlog-atlas の写しとバイト同一、BSL-1.0 は clipboard-win のため新規)。THIRD-PARTY-NOTICES.md の条文は b12110 のチェックアウトと Vulkan-Headers の vulkan-sdk-1.4.363.0 タグから書き写さずに取った。Vulkan-Headers の年は推測で書かず、ヘッダーの実物 (2015-2026) を見て直した。
- 古くなった通知を検出するテスト (src/lib/third-party-licenses.test.ts) は、scripts/spdx/MIT.txt を 1 バイト変えると失敗することを確かめた。.gitattributes で LF に固定し、renormalize で既存ファイルの中身が変わらないことを確かめた。
- macOS: Developer ID (公証なし) で署名した pnpm tauri build で、.app・本体・llama-server の 3 つに Developer ID と hardened runtime (flags=0x10000) が付き、entitlements 無しで、署名した llama-server が Metal (Apple M2) を認識して生成できた。署名した .app を起動し、起動時のクリップボード翻訳が英→日の訳文を返して履歴に残った (テストの訳文が履歴に 1 件増えている)。Contents/Resources の icon.icns と THIRD-PARTY-LICENSES.txt はリポジトリのものと一致。.dmg は 12.9 MB。
- 未確認 (AC はチェックしていない): 公証と staple (AC#1)、Windows の llama-server とインストーラー (AC#2)、Windows の同梱物 (AC#3 の Windows 側)。release.yml は未実行。.ps1 の Web UI を外す変更も Windows では未実行。
- テスト: pnpm check 0 件、pnpm lint、pnpm test 100 件、cargo fmt --check、cargo clippy --all-targets -D warnings、cargo test 218 件 (無視 1)。

- AC#1 (2026-11-10、この Mac、M2): .env.signing (backlog-atlas と同じ Developer ID Application: 9EYB4D9GGQ) で scripts/macos-sign-build.sh を実行した。Tauri が .app を公証 (Accepted)、スクリプトが .dmg を公証 (Accepted) して staple した。macos-verify-gatekeeper.sh で .app・.dmg・隔離属性を付けた .dmg の中の .app の 3 つが「notarized Developer ID」として受け入れられた。CI での署名・公証はシークレットの登録後に試行する。
- レビュー 1 回目 (Codex) は P3 が 2 件: 生成スクリプトが Windows のバックスラッシュのパスで npm パッケージを見つけられない点と、ワークフローがインストールした llama-server.exe --version の終了コードを見ていない点。どちらも直した。
<!-- SECTION:NOTES:END -->
