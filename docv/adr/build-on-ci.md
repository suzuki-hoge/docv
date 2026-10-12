---
group: 配布
date: 2026-10-11
---

# GitHub Actions で各プラットフォーム向けにビルドする

## 決めたこと

v で始まるタグの push で、GitHub Actions が各プラットフォーム向けにビルドし、GitHub Release に置く。

## 選ばなかった案

| 案 | 選ばなかった理由 |
| --- | --- |
| 手元の Mac で全プラットフォーム向けにビルドし、gh release でアップロードする | public リポジトリなら macOS と Windows のランナーも無料で使え、タグを push するだけでリリースできる |
| Linux のランナーで cargo-zigbuild を使い、macOS 向けもクロスコンパイルする | 各 OS のランナーで素直にビルドできるので、クロスコンパイルの手間をかける理由がない |
