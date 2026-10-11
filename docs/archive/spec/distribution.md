---
question: docs/spec/distribution.md から外した内容と、外した理由は何か
source: docs/spec/distribution.md
---

# spec/distribution.md の経緯

## 2026-10-11 手元の Mac でビルドする案と、Linux でクロスコンパイルする案

- 移した元: [spec/distribution.md](../../spec/distribution.md)
- 理由: public リポジトリなら GitHub Actions の macOS と Windows のランナーが無料で使え、タグを push するだけでリリースできる

macOS の CI の費用を避けるため、手元の Mac で全プラットフォーム向けにビルドして gh release でアップロードする案と、Linux のランナーで cargo-zigbuild を使って macOS 向けもクロスコンパイルする案だった。

## 2026-10-11 Go で書く案

- 移した元: [spec/distribution.md](../../spec/distribution.md)
- 理由: GitHub Actions で各プラットフォーム向けにビルドするなら、Go のクロスコンパイルの手軽さは決め手にならない

Go なら GOOS と GOARCH を変えるだけで、1 台から macOS 向けも含めてクロスコンパイルできる、という理由で挙げた案だった。
