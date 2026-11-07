---
id: TASK-12
title: 翻訳の中核を Windows で確かめる
status: In Review
assignee: []
created_date: 2026-11-05 19:41
updated_date: '2026-11-07T23:05:00Z'
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
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## 判断
<!-- SECTION:NOTES:END -->
