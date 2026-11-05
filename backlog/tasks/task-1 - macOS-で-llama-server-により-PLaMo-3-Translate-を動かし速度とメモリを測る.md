---
id: TASK-1
title: macOS で llama-server により PLaMo 3 Translate を動かし速度とメモリを測る
status: In Review
assignee: []
created_date: '2026-11-05 19:41'
updated_date: '2026-11-05 21:26'
labels: []
milestone: m-0
dependencies: []
references:
  - 'https://huggingface.co/pfnet/plamo-3-translate-2b-GGUF'
documentation:
  - backlog/docs/doc-1 - tiny-translator-の設計.md
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-1 は翻訳エンジンを llama-server の子プロセスにし、既定モデルを 2B Q4_K_M、コンテキスト長を 8,192 トークンとしたが、どれも M1/M2 での実測が無い。主対象の macOS で、PFML3 のプロンプトを /completion に渡して翻訳できるか、16 GB の機械で他のアプリを圧迫しないかを、アプリを作る前に確かめる。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Metal 版 llama-server (取得元とバージョンを記録) が PLaMo 3 Translate の GGUF を読み込み、doc-1 §4.3 の PFML3 プロンプトで英日・日英の翻訳を返す
- [x] #2 0.8B・2B・8B の Q4_K_M について、読み込み時間、常駐メモリ (RSS とアクティビティモニタのメモリ)、生成速度 (tok/s) をコンテキスト長 8,192 で測り、表にして doc-1 §4.5 と §11 へ反映している
- [x] #3 llama-server の終了後にメモリが OS へ戻ることを確かめている
- [x] #4 停止トークン <|plamo:tag|> と <|plamo:bos|> で生成が止まることを確かめている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. llama.cpp の安定版 v0.9.0 が指すビルド b12110 の macOS arm64 版 (Metal) と、0.8B・2B・8B の Q4_K_M GGUF を Hugging Face から ~/Models へ取得する (リポジトリの外)。
2. scripts/measure_engine.py (標準ライブラリのみ) を作る。アプリと同じ形 (127.0.0.1・空きポート・--parallel 1・-c 8192) で llama-server を起動し、/health が 200 になるまでを読み込み時間、ps の RSS と footprint の phys_footprint (アクティビティモニタの「メモリ」) を常駐メモリ、/completion の timings を生成速度として取る。英日・日英の PFML3 プロンプトを各 3 回送り、stop 指定の有無で stop_type / stopping_word を記録する。終了後に vm_stat の使用済みメモリが読み込み前に戻るかを見る。
3. 3 サイズを順に測り、結果を表にして doc-1 §4.5 と §11 へ反映する (backlog doc update)。測定機は M2 24 GB で、16 GB 機の判断は常駐メモリの絶対値から行う (オーナー了承済み)。

4. ファンのない MacBook Air では GPU 負荷が続くと熱で生成速度が落ちたため、起動ごとに 3 分休ませて測り直し (--cooldown)、1,024 トークンの連続生成は llama-bench で別に測る。
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 取得元とバージョン (AC#1)
- llama-server: ggml-org/llama.cpp の GitHub リリース。安定版 v0.9.0 が指すビルド b12110 (commit cd959bae9, ggml 0.29.0) の llama-b12110-bin-macos-arm64.tar.gz (Metal 版)。`llama-server --version` で確認。
- GGUF: Hugging Face の pfnet/plamo-3-translate-{0.8b,2b,8b}-GGUF の Q4_K_M。SHA-256 は HF の LFS oid と一致。置き場所は ~/Models (リポジトリの外)。
- doc-1 §4.3 の PFML3 プロンプト (翻訳元言語つき) を /completion に送り、3 サイズとも英日・日英の訳文が返った。訳文は自然で、0.8B と 2B の差は語の選び方程度。

## 計測 (AC#2)
- 計測機は MacBook Air M2 24 GB (ファンなし)。16 GB 機の判断は RSS の絶対値から行う (オーナー了承済み、2026-11-06)。
- scripts/measure_engine.py で -c 8192 --parallel 1 の llama-server を測り、結果を doc-1 §4.5 の「macOS での実測 (TASK-1)」と §11 へ反映した。
- 冷えた状態 (起動ごとに 3 分休ませた後) の生成速度: 0.8B 140 / 2B 47 / 8B 16 tok/s。RSS: 0.76 / 2.1 / 5.4 GB。アクティビティモニタの「メモリ」: 0.11〜0.14 / 0.24〜0.26 / 0.29〜0.32 GB。読み込み時間: 1.3〜1.7 / 1.4〜1.5 / 1.8〜3.3 秒。
- アクティビティモニタの「メモリ」(phys_footprint) は mmap した重みを含まず、実際の消費 (使用済みメモリの増加) は RSS とほぼ同じ。設定画面のメモリの目安は RSS を使うべき。
- 最初の計測は LM Studio の llama-server (常駐 5 GB) と Chrome などでスワップ 14 GB の状態だったため、2B が 7〜12 tok/s・8B が 3〜4 tok/s と実態より遅く出た。オーナーにアプリを終了してもらって測り直した。その後も 2B が遅く出たのは熱による速度低下で、llama-bench でも冷えた状態の 47 tok/s が連続負荷の後は 10 tok/s 前後になった。温度は root 権限が要るため測っていない (熱が原因というのは、冷却後に回復することからの推定)。
- 8B の 1,024 トークン連続生成は 3.8 tok/s。2B は 1,024 トークンでは落ちなかった (46.7 tok/s)。

## メモリの返却 (AC#3)
- 全 24 回の起動 (アプリ終了後の測り直し 18 回 + 冷えた状態での 6 回) で、SIGTERM 後に終了コード 0 でプロセスが消え、終了 3 秒後の使用済みメモリは起動前の −0.53〜+0.35 GB に戻った (読み込み中の増加は 0.8B +0.86 GB、8B +5.4 GB)。

## 停止トークン (AC#4)
- GGUF の語彙で EOG になっているのは <|plamo:bos|> (1)・<|plamo:eos|> (2)・<|plamo:file_separator|> (5)・<|plamo:tag|> (16)。llama-server の -lv 4 のログで確認。
- stop 指定なしの翻訳はすべて最後のトークンが 16 (<|plamo:tag|>) で stop_type=eos (n_probs で確認)。
- <|plamo:bos|> は通常の翻訳では出ないため、logit_bias で強制して確かめた: [[1,100]] で 1 トークン目に生成され stop_type=eos で止まる。16・2 も同様。
- EOG で止まるので、リクエストの stop 文字列は使われない (stopping_word は常に空)。ignore_eos にすると EOG が出なくなり、訳文の後に意味のない出力が続く。

## 測っていないもの
- 16 GB の機械と M1、ページキャッシュに GGUF が無い状態 (再起動直後) の読み込み時間。
- ダウンロード直後の最初の起動で初期化に約 10 秒かかった原因 (2 回目以降は 0.06 秒)。
- --cache-ram と -fit off をアプリで使うかは m-2 で決める (doc-1 §4.5 に判断材料を書いた)。
<!-- SECTION:NOTES:END -->
