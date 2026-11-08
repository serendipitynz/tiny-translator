---
id: TASK-12
title: 翻訳の中核を Windows で確かめる
status: In Review
assignee: []
created_date: 2026-11-05 19:41
updated_date: '2026-11-08T01:45:00Z'
labels: []
milestone: m-2
dependencies:
  - TASK-10
  - TASK-11
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Windows の Vulkan 版 llama-server で、m-2 の機能が macOS と同じに動くかを確かめる。対象機は Ryzen Z1 と VRAM 8 GB の GPU 機 (doc-1 §2)。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Vulkan 版 llama-server を sidecar として同梱し、Windows で翻訳できる
- [x] #2 Ryzen Z1 (または手元の Windows 機) で 0.8B・2B の読み込み時間・常駐メモリ・生成速度を測り doc-1 §4.5 へ反映している
- [x] #3 自動解放と起動時のクリップボード翻訳が Windows でも動く
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. sidecar は macOS と同じくソースから 1 つの静的 exe にビルドする。GitHub リリースの win-vulkan 版は llama.dll / ggml-*.dll に分かれ、bundle.externalBin の 1 ファイルに収まらない (macOS のリリース版を採らなかったのと同じ理由)
2. scripts/build-llama-server.ps1 を作る。.sh と同じ b12110 (commit cd959bae9)、出力先 src-tauri/binaries/llama-server-<host triple>.exe、取得先はリポジトリ外のキャッシュ (~/.cache/tiny-translator/)。CMake 引数: GGML_VULKAN=ON, BUILD_SHARED_LIBS=OFF, GGML_NATIVE=OFF (AVX2 までにし、Zen 4 の AVX-512 を使わない), GGML_OPENMP=OFF (vcomp140.dll を避ける), CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded (VC++ ランタイムの DLL を避ける。llama.cpp の cmake_minimum_required が 3.14...3.28 なので CMP0091 は NEW で効く), GGML_RPC=OFF, LLAMA_CURL=OFF, LLAMA_OPENSSL=OFF, LLAMA_BUILD_TESTS=OFF, LLAMA_BUILD_EXAMPLES=OFF。CMake は PATH に無ければ Visual Studio 同梱のものを vswhere で探す。dumpbin /dependents で OS の DLL と vulkan-1.dll 以外を読み込んでいたら失敗させる
3. scripts/build-llama-server.sh の未対応トリプルのメッセージを .ps1 へ向ける
4. scripts/measure_engine.py に Windows の分岐を足す (標準ライブラリだけ)。プロセスの working set・private bytes (GetProcessMemoryInfo)、プロセスごとの GPU 専用・共有メモリ (PDH を PdhAddEnglishCounterW で。カウンター名が日本語化されているため)、OS の利用可能な物理メモリ (GlobalMemoryStatusEx) を読み込み前・読み込み後・生成後・終了後に測る。macOS の出力キーは変えない
5. この PC (Ryzen 7 5800X / RTX 5060 Ti) でビルドし、0.8B・2B を測る。Ally (Z1 Extreme / 780M) はオーナーが exe・GGUF・スクリプトを持ち込んで測り、結果を受けて doc-1 §4.5 に Windows の実測を足し、§11 の Z1 の行を更新する。§4.2 にビルドの判断を記録する
6. AC#1・#3 をこの PC の実アプリで確かめる: tauri dev とインストーラー (nsis) で翻訳、自動解放 (10 分) とメモリの戻り、起動時のクリップボード翻訳 (初回と単一インスタンスの 2 回目)、異常終了後に残った翻訳エンジンの終了 (Windows の tasklist / taskkill の経路)
7. README の sidecar の節に Windows のビルド (Vulkan SDK・CMake・.ps1) を足す。Windows で pnpm check / lint / test、cargo clippy / test / fmt --check を通す (engine.rs のテストは cfg(unix) で Windows では走らないことを明記する)

8. (オーナー判断 2026-11-08) Windows の process_is を、tasklist のイメージ名ではなく実行ファイルのフルパスで判定する形に直す。Windows だけで走る kill_leftover のテストを足し、別のパスの llama-server.exe を止めないことを実アプリで確かめる
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 判断
- sidecar は macOS と同じくソースから 1 つの静的 exe にビルドした (オーナー承認 2026-11-08)。GitHub リリースの win-vulkan 版は llama.dll / ggml-*.dll に分かれ、bundle.externalBin の 1 ファイルに収まらない。scripts/build-llama-server.ps1 を足し、引数と理由は doc-1 §4.2 に記録した。
- CMake の引数は、llama.cpp b12110 の cmake_minimum_required が 3.14...3.28 なので CMAKE_MSVC_RUNTIME_LIBRARY (CMP0091) が効くことを確かめてから使った。GGML_STATIC は MSVC では何もしない (ggml/src/CMakeLists.txt が NOT MSVC のときだけ見る) ので使っていない。LLAMA_CURL は b12110 では非推奨 (警告だけ) だが、.sh と揃えて残した。
- 読み込む DLL の検査は、System32 にあるかだけでは VC++ 再頒布パッケージの DLL (vcruntime140.dll など。製品名が「Microsoft Visual Studio」) を通してしまうので、製品名が Windows のものと vulkan-1.dll と api-ms-win-* だけを通す形にした。vcruntime140 / msvcp140 / vcomp140 が落ちることを確かめた。
- measure_engine.py は Windows の分岐を足した (標準ライブラリだけ)。GPU のメモリはパフォーマンスカウンターを PdhAddEnglishCounterW で読む (日本語の Windows ではカウンター名が日本語になり、英語名の Get-Counter が通らないため)。macOS の出力キーは変えていない。
- Windows の process_is (異常終了後に残った翻訳エンジンかどうかの判定) を、tasklist のイメージ名ではなく実行ファイルのフルパスで比べる形に直した (オーナー判断 2026-11-08)。イメージ名だけでは、engine.pid の PID が別の llama-server.exe (LM Studio が同梱するものなど) に使い回されたときに止めてしまい、doc-1 §5 の「sidecar と同じパスで起動されたものであるときだけ」を満たさないため。パスは OpenProcess と QueryFullProcessImageNameW で取り、大文字・小文字と \\?\ の接頭辞を無視して比べる。kernel32 の 3 関数は std がすでにリンクしているので、windows-sys を足さず extern "system" で宣言した。
- engine.rs の set_idle_release の expect(dead_code) の条件を not(test) から not(all(test, unix)) に直した。テストのモジュールが cfg(all(test, unix)) なので、Windows の cargo clippy --all-targets がテストのビルドで dead_code になっていた (TASK-7 では Windows で clippy を走らせていなかった)。

## 確認 (この PC: Windows 11 Home 10.0.26200、Ryzen 7 5800X、RTX 5060 Ti 16 GB、NVIDIA 32.0.16.1088、Vulkan SDK 1.4.363.0、VS 2022 17.14 / MSVC 19.44)
- AC#1: build-llama-server.ps1 で 54 MB の exe ができ (約 4 分)、dumpbin の結果は vulkan-1.dll・KERNEL32・ADVAPI32・WS2_32 だけだった。CMakeCache で GGML_AVX2=ON・GGML_AVX512=OFF・GGML_OPENMP=OFF・BUILD_SHARED_LIBS=OFF を確かめた。Windows PowerShell 5.1 (-ExecutionPolicy Bypass -File) でも通った。pnpm tauri build --bundles nsis のインストーラー (12.4 MiB) を入れると %LOCALAPPDATA%\tiny-translator\ に tiny-translator.exe と llama-server.exe が並び、インストールしたアプリで英日・日英の翻訳ができた (起動時のクリップボード翻訳の画面で確認)。
- AC#2: この PC と ROG Ally (Z1 Extreme / 780M、16 GB、UMA 4 GB、Armoury Crate は「パフォーマンス」、AC 電源、AMD ドライバー 32.0.12011.7001) で、アプリと同じ引数 (-c 8192 --parallel 1 --cache-ram 0 --no-webui) の 0.8B・2B を各 3 回起動して測り、doc-1 §4.5 に「Windows での実測 (TASK-12)」として足し、§11 の Z1 の行を確認済みにした。Ally はオーナーが exe・GGUF・スクリプトを持ち込んで測った (ビルド環境なし、Python 3.12.3)。主な値: GPU 機は 0.8B 約 465 tok/s・2B 約 158 tok/s、Ally は 0.8B 約 120 tok/s・2B 約 39 tok/s、読み込みは両機とも 2.3〜4.4 秒。終了後は両機とも利用可能メモリと GPU のメモリが読み込み前に戻った。参考として -lm none でも測った (GPU 機は RAM の消費が 2.1 → 0.4 GB に減り、Ally は変わらない)。
- AC#3 (インストールしたアプリ、5 秒ごとにプロセス・engine.pid・利用可能メモリ・GPU メモリを記録):
  - 起動時のクリップボード翻訳: 初回の起動で英文が日本語に、2 回目の起動 (単一インスタンスでプロセスは 1 つのまま) で和文が英語に訳された。
  - 自動解放: 最後の翻訳 (11:25:31) から 10 分後の 11:35:32 に翻訳エンジンが終了し、engine.pid が消えた。利用可能メモリは 34.33 → 36.53 GB、GPU 全体の専用メモリは 3.31 → 1.38 GB に戻った (2B)。その後の翻訳で翻訳エンジンが再び起動し (読み込み 3.3 秒)、訳文が返った。
  - 異常終了後の後始末: Stop-Process -Force でアプリだけを止めると llama-server (PID 24316) と engine.pid が残り、アプリを起動し直すと taskkill の経路で 24316 が終了し、新しい翻訳エンジンが起動した。
  - パスで判定する形に直した後 (デバッグビルドのアプリ): 別のパスの llama-server.exe (Ally 用キットのもの) を起動してその PID を engine.pid に書いてからアプリを起動すると、そのプロセスは止まらず、アプリは自分の翻訳エンジンを起動した。続けてアプリだけを強制終了して起動し直すと、残った翻訳エンジン (target\debug\llama-server.exe) だけが終了した。同時に動いていた LM Studio の llama-server.exe にも触れていない。
  - ウィンドウを閉じるとアプリと翻訳エンジンが終了し、engine.pid が消えた。
- pnpm check (0 件)・pnpm lint・pnpm test (44 件)・cargo fmt --check・cargo clippy --all-targets -- -D warnings・cargo test (66 件) が Windows で通った。engine.rs の既存のテストは cfg(all(test, unix)) (偽の翻訳エンジンに sleep / false を使う) なので Windows では走っておらず、66 件に含まれない。66 件のうち 1 件は Windows だけで走る kill_leftover のテスト (同じファイル名で別のパスの PING.EXE は止めず、大文字・小文字の違う同じパスなら止める) で、イメージ名だけの判定なら前半で落ちる。

## 見ていないこと・気づいたこと
- GPU 機では mmap した GGUF が RAM にも残る (2B で利用可能メモリが 2.1 GB 減る)。-lm none で 0.4 GB に減り、速度は変わらなかった (doc-1 §4.5)。Windows で -lm none を採るかはオーナーの判断を待つ。
- exe を置いた直後の最初の起動では読み込みと最初の翻訳が数秒遅い (doc-1 §4.5)。原因は確かめていない。
- 開発環境の注意: この PC では pnpm が fnm のシェル設定からしか入らず、Tauri の beforeBuildCommand から見えなかった (corepack の pnpm を PATH に足して回避)。
<!-- SECTION:NOTES:END -->
