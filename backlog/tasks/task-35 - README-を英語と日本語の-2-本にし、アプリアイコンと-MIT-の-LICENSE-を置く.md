---
id: TASK-35
title: README を英語と日本語の 2 本にし、アプリアイコンと MIT の LICENSE を置く
status: Done
assignee: []
created_date: '2026-11-11 00:59'
updated_date: '2026-11-11 02:04'
labels: []
milestone: m-5
dependencies: []
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
README.md が日本語だけで、公開リポジトリとして英語の読み手に届かない。兄弟アプリ (snz_studio・backlog-atlas) は README.md を英語、README.ja.md を日本語にし、冒頭にアプリアイコンを載せている。また、リポジトリは公開されているのに LICENSE が無く、本体のコードを他人が使えない状態にある。本体は兄弟アプリと同じ MIT にする (2026-11-11 オーナー判断)。PLaMo のモデルはリポジトリにも配布物にも入らないので、本体のライセンスとは別に扱う。
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 README.md が英語、README.ja.md が日本語で、互いへのリンクを冒頭に持ち、同じ内容を説明している
- [x] #2 両 README の冒頭に src-tauri/icons のアプリアイコンを幅 128 で表示している
- [x] #3 リポジトリのルートに MIT の LICENSE があり、package.json と src-tauri/Cargo.toml の license が MIT になっている
- [x] #4 両 README に、本体が MIT であること・同梱物のライセンスの在りか・PLaMo Community License が別に適用されることを書いている
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. git mv README.md README.ja.md し、冒頭にアイコン (src-tauri/icons/128x128@2x.png、backlog-atlas と同じ参照) と英語版へのリンクを置く
2. README.md を英語で書き起こす (Windows の警告の操作は英語版 Edge の表記で書く)
3. LICENSE (MIT, Copyright (c) 2026 Takuya Otani) を mallow と同じ文面で置き、package.json と Cargo.toml に license = MIT を足す
4. 両 README にライセンスの節を足す (本体 MIT、THIRD-PARTY-LICENSES.txt、PLaMo Community License)
5. pnpm test (THIRD-PARTY-LICENSES の鮮度検査が package.json / Cargo.toml の変更で崩れないか) と cargo の検査で確かめる
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
README.md は git mv で README.ja.md にし、英語版を新たに書いた。アイコンは backlog-atlas と同じく src-tauri/icons/128x128@2x.png (256px) を直接参照する (snz_studio は Wails の build/ に重い元画像しか無いため docs/assets に複製しているが、Tauri には 256px がすでにある)。mallow の README にはアイコンが無い。画面は日本語だけなので、英語版は画面の表記を日本語のまま引き、英語の訳を添えた。LICENSE は mallow と同じ MIT の文面。検証: pnpm test 108 件通過 (THIRD-PARTY-LICENSES の鮮度検査は package.json / Cargo.toml を入力にしないので影響なし)、cargo metadata で license = MIT を読めた、両 README の相対リンクとアイコンのパスが存在する。両 README の節の構成 (## が 6、### が 1) が一致する。
<!-- SECTION:NOTES:END -->
