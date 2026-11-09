---
id: doc-2
title: 先行事例と差別化
type: other
created_date: '2026-11-05 19:39'
updated_date: '2026-11-09 20:11'
---
# 先行事例と差別化

PLaMo 3 Translate の重み公開 (2026-11-05) の翌日に調べた。調査には RustXNG (ローカルのメタ検索) と GitHub API を使った。
先行事例の状況は変わりやすいので、m-5 で配布を判断する前に調べ直す。

## 1. 先行事例

### 1.1 tanakh/plamo-translate-desktop (最も近い)

- https://github.com/tanakh/plamo-translate-desktop 。2026-11-05 作成、Apache-2.0、v0.1.0 (Windows x86_64 と Linux x86_64 の配布物)。
- Rust + GPUI。llama.cpp を `llama-cpp-2` でプロセス内に組み込み、GGUF を CPU (AVX2 / AVX-512) か Vulkan で動かす。
- 設計書 (`docs/design.md`) の §1 は主対象を Windows、副対象を Linux とし、**macOS・モバイル・GGUF 以外の形式 (MLX など) を「Non-goals for now」としている。**
- 持っている機能:
  - 53 言語の翻訳。既定の翻訳先は「日本語 ⇄ 英語 (自動)」
  - 貼り付けで翻訳、入力が止まったら翻訳、ストリーミング表示、停止
  - 日本語の常体・敬体、文脈、用語集
  - 長文の段落分割と、前の段落の対訳を履歴としてプロンプトに入れる方式。出力の繰り返しを検知して小さく分けて訳し直す
  - モデル管理 (サイズと量子化の選択、再開できるダウンロード、SHA-256 の照合、Hugging Face から新しいファイルを見つける)
  - Chrome / Edge 拡張 (選択範囲の翻訳、ページ翻訳でリンクと書式を保つ)。拡張用のホストは 5 分使わないと終了してメモリを解放する
  - 英語と日本語の UI、Light / Dark
- 持っていない機能: 翻訳履歴、起動時のクリップボード翻訳 (グローバルホットキーでの翻訳は「Possible later work」)、アプリでの Markdown・JSON・HTML の構造保持、
  優先順位を設定できる翻訳先の決定、アプリ本体でのモデルの自動解放
- 参考になる実測と知見:
  - 2B Q4_K_M を Ryzen 9 9950X の CPU で動かしたとき、生成は約 19.5 tok/s (メモリ帯域が律速)
  - インライン要素を番号付きのタグ (`<a i="2">`) にして渡すと、0.8B と 2B でタグの位置と順序の入れ替えが保たれた
  - 小さいモデルは長い出力で繰り返しや抜けを起こすので、翻訳単位を小さく保つ

### 1.2 PLaMo 公式のローカル向けツール

| 名前 | 形態 | 対象 | 備考 |
|---|---|---|---|
| pfnet/plamo-translate-cli | CLI (Python, `pip install plamo-translate`) | Apple Silicon の macOS (MLX) | PLaMo 2 Translate 向け。MCP サーバーとしても動く |
| PLaMo Translate (Firefox 拡張) | ブラウザ拡張 | | ページ翻訳 |
| PLaMo 翻訳 (Web サービス) | Web | | 本アプリの画面の手本。オフラインでは使えない |

### 1.3 汎用の翻訳アプリ (外部の LLM 実行環境を使う)

| 名前 | 対象 | ローカル翻訳の方式 |
|---|---|---|
| Easydict | macOS | Ollama、Apple の Translation フレームワーク (macOS 15 以降、システムの翻訳モデル) |
| Pot | macOS / Windows / Linux (Tauri) | Ollama などのプラグイン |
| local-translator (dwain-barnes) | デスクトップ | ローカル LLM |
| LTEngine (LibreTranslate) | 自前ホストの API (Rust) | LLM。LibreTranslate 互換 |
| beko-translate | macOS の CLI (mlx-lm) | PDF の見開き翻訳 |

これらは Ollama などを別に導入する必要があり、本アプリの R2 (他のアプリに依存しない) を満たさない。

## 2. 本アプリが差別化できる点

| 差別化の軸 | 根拠 | 先行アプリが追いつく見込み |
|---|---|---|
| macOS (Apple Silicon) で単体で動く | tanakh 版は macOS を対象外にしている。公式 CLI は Python 環境が要り、PLaMo 2 向け | **高い**。Rust + GPUI は macOS で動き、llama.cpp は Metal を持つ |
| 翻訳履歴と全文検索 | どの先行事例も持たない | 中 |
| Markdown・JSON・HTML の構造保持翻訳 | tanakh 版はブラウザ拡張の HTML だけ | 中 |
| 優先順位による翻訳先の決定 | tanakh 版は日本語 ⇄ 英語の自動だけ | 高い (設定を足すだけ) |
| 起動時のクリップボード翻訳 | tanakh 版は計画のみ | 高い |
| 翻訳エンジンの自動解放でメモリを返す | tanakh 版はアプリ本体では解放しない | 中 |
| snz-design の見た目 | オーナーの他のアプリとそろう | — (オーナー固有) |

## 3. 判断

- **自分用には作る価値がある** (2026-11-06 オーナー判断で継続)。主に使う M1/M2 で動く先行アプリが無く、§2 の機能はどれも先行アプリに無い。
- **配布の価値は限定的で、m-5 で判断する。** tanakh 版が macOS に対応すると、残る差は履歴・構造保持翻訳・自動解放などの機能だけになる。
  配布を判断するときは、tanakh 版の macOS 対応の有無を調べ直し、対応していれば、機能を tanakh 版へ提案する道と比べる。
- ライセンス: PLaMo Community License は、モデルを再配布するとライセンスが配布先へ及ぶ。翻訳結果を公表するときに PLaMo の出力であることを示す義務
  (tanakh 版の README が第 5 条として挙げている) は、2026-11-09 の改定で書籍・論文などの長文に使うときだけになった。
  条文の要約とアプリが満たすべき表示は doc-1 §10 にある (TASK-23)。
