---
id: TASK-11
title: 起動時のクリップボード翻訳と貼り付け・コピー・クリアを作る
status: Done
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-07 08:09'
labels: []
milestone: m-2
dependencies:
  - TASK-9
  - TASK-3
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R3 と R4。起動したらクリップボードの文字列をすぐ訳したい。2 回目の起動でも既存のウィンドウで同じことをする (doc-1 §5, §6.3)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 起動直後にクリップボードがテキストを持てば原文欄へ入れて翻訳を始める。前回の起動時と同じ文字列なら翻訳しない
- [x] #2 単一インスタンスで、2 回目の起動は既存のウィンドウを前面に出して起動時のクリップボード翻訳を行う
- [x] #3 原文の貼り付け・コピー・クリア、訳文のコピーができ、コピーしたことを一時的な通知で伝える
- [x] #4 ウィンドウの位置と大きさが再起動後も保たれる
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. 依存を足す (オーナー承認 2026-11-07): tauri-plugin-single-instance・tauri-plugin-window-state・tauri-plugin-clipboard-manager と、macOS だけ objc2-app-kit (NSPasteboard)。JS のプラグインパッケージは入れず、読み書きは Rust のコマンドにする。
2. src-tauri/src/clipboard.rs: 起動時に読んでよいかの判定 (macOS 15.4 以降は accessBehavior が AlwaysAllow のときだけ、それより前と Windows は常に)、前回の起動時に翻訳した文字列との比較 (本文ではなくハッシュをアプリデータディレクトリに置く)、空白だけの文字列を空と扱う処理。単体テストを付ける。
3. lib.rs: launch_clipboard (起動時のクリップボード翻訳用。text / none / not-allowed を返す)、paste_clipboard ([貼り付け] 用。判定せずに読む = オーナー判断 A)、copy_text を足す。single-instance の callback と macOS の RunEvent::Reopen で窓を前面に出し、画面へ launched-again を送る。window-state は位置・大きさ・最大化だけ保存する。
4. 画面: 起動時と launched-again で launch_clipboard を呼び、文字列があれば原文欄を置き換えて翻訳する。not-allowed のときは原文欄が空の間、システム設定での許可・一覧に無ければ [貼り付け] を一度押すこと・Cmd+V を案内する。[貼り付け] は原文を置き換えるだけで翻訳しない (オーナー判断)。コピー・クリアを繋ぎ、コピーは doc-9 §6.5 の一時的な通知 (Toast.svelte) で伝える。§8.1 の Ctrl/Cmd+Shift+C も足す。
5. doc-1 §6.3 に [貼り付け] の扱いと案内の決定を書き、テスト・lint を通し、macOS で動かして確かめる。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 着手時に決めたこと (2026-11-07 オーナー判断)
- 本番依存として tauri-plugin-single-instance・tauri-plugin-window-state・tauri-plugin-clipboard-manager・objc2-app-kit (macOS のみ) の 4 つを承認。実装中に objc2 (macOS のみ) も直接の依存にした。accessBehavior が無い macOS 15.4 より前で呼ぶと未知のセレクタでアプリが落ちるので、objc2::available! で確かめるため。objc2 は tauri 経由ですでにビルドに入っていた版 (0.6.5) で、ビルドされるものは増えない。承認の 4 つに含まれていないので、レビューで確認してほしい。
- [貼り付け] ボタンは accessBehavior を調べずに常に読む (案 A)。原文欄を置き換えるだけで、翻訳は始めない。
- Default のアプリがシステム設定の一覧に並ぶかは実機で確かめていない。objc2-app-kit が写している Apple のヘッダーの説明では、一覧で変えられるのは過去に確認を出したアプリだけ。案内は Default と Ask で同じ文にし、「一覧に無ければ先に [貼り付け] を一度押して確認に答える」を入れた。doc-1 §5・§6.3 に書いた。

## 実装で選んだこと
- 前回の起動時の文字列は、本文ではなく DefaultHasher のハッシュをアプリデータディレクトリの launch-clipboard に置く。クリップボードのパスワードなどをディスクに残さないため。Rust の版でハッシュの値が変わっても、同じ文字列を一度訳し直すだけで済むので、ハッシュのクレートは足していない。
- macOS では、動いているアプリを Finder・Dock・Spotlight (open コマンド) から開き直しても 2 つ目のプロセスは起動せず RunEvent::Reopen が届くだけなので、single-instance の callback と同じ処理 (前面に出して launched-again を画面へ送る) を Reopen にも付けた。Dock のアイコンを押したときにも起動時のクリップボード翻訳が走るが、同じ文字列なら訳さない。
- コピーの通知は snz-design doc-9 §6.5 の一時的な通知 (src/lib/ui/Toast.svelte)。消えるまでの時間は Storybook の実例と同じ 1 文字 90 ms で、--snz-motion-toast-min/max (1.8〜6 秒) に収める。翻訳ボタンを覆わないよう、画面の下端の始端側に出す。コピー・貼り付けの失敗は通知ではなく、既存の失敗の表示に出す。
- doc-1 §8.1 の Ctrl/Cmd+Shift+C (訳文をコピー) も足した。

## 確かめたこと (macOS 26.7.1、MacBook Air M2、pnpm tauri build --debug --bundles app の .app)
- AC#1: pbcopy で英文を置いて起動すると原文欄に入り、翻訳され訳文が出た (画面の撮影)。同じ文字列のまま終了して起動し直すと、原文欄は空のままで launch-clipboard も書き換わらなかった。
- AC#2: 同じ文字列のまま実行ファイルをもう一度起動すると、2 つ目のプロセスはすぐ終わり、tiny-translator が最前面になり、訳し直さなかった。文字列を変えて同じことをすると新しい文字列を訳した。open コマンドでの開き直し (Reopen) でも最前面になり、新しい文字列を訳した。
- AC#3: AppleScript (System Events) でボタンを押して確かめた。[貼り付け] で原文欄がクリップボードの文字列になった。[原文をコピー]・[訳文をコピー]・Cmd+Shift+C でクリップボードがそれぞれの文字列になり、「訳文をコピーしました」の通知が出た (画面の撮影)。[クリア] で原文欄が空になった。クリップボードが空のときの [貼り付け] は「クリップボードに貼り付けられる文字列がありません」を出した。
- AC#4: AppleScript でウィンドウを (300, 150)、800×560 にして終了し、起動し直すと同じ位置と大きさだった。.window-state.json はアプリ設定ディレクトリにできる。
- not-allowed の経路: EnablePasteboardPrivacyDeveloperPreview をこのアプリの defaults に入れて起動すると、確認は出ず、原文欄に許可の案内が出て、クリップボードは読まれなかった。確かめた後に設定は消した。
- pnpm check・pnpm lint・pnpm test (44 件)・cargo fmt --check・cargo clippy --all-targets -D warnings・cargo test (40 件) が通る。

## 確かめていないこと
- 一時的な通知の、ポインタを乗せている間・フォーカスが内側にある間に消えないこと、閉じるボタンでフォーカスが戻ることは、目では確かめていない。
- 開発者プレビューを有効にした状態で [貼り付け] を押したときの確認と、その後システム設定の一覧に並ぶこと (押すとこの Mac の設定に許可の状態が残るので押していない)。
- Windows での動作は TASK-12 で確かめる。

## レビューでの修正 (PR #11)
- Codex の 1 回目のレビューで [P3] が 1 件出た。閉じるボタンにフォーカスがある通知が次の通知に置き換わると、フォーカスが失われる。d787333 で、フォーカスを新しい通知の閉じるボタンへ移し、最初の通知が出る前の戻り先と止めた時間を保つようにした。
- AppleScript で確かめた。原文欄にフォーカス、Cmd+Shift+C、閉じるボタンにフォーカス、もう一度 Cmd+Shift+C とすると、新しい閉じるボタンにフォーカスがあり、7 秒後も通知は出たままで、閉じると原文欄へフォーカスが戻った。フォーカスの無い通知は 3.5 秒以内に消えた。上の「確かめていないこと」のうち、フォーカスの間は消えないことと閉じたときに戻ることは、これで確かめた。ポインタを乗せている間に止まることは、まだ目で見ていない。
- 2 回目のレビューで APPROVE された。
<!-- SECTION:NOTES:END -->
