---
question: docs/spec/config.md から外した内容と、外した理由は何か
source: docs/spec/config.md
---

# spec/config.md の経緯

## 2026-10-10 本文の中の型付きブロックで構造化する案

- 移した元: [spec/config.md](../../spec/config.md)
- 理由: 構造化は frontmatter と docv.yaml に限る。個々のファイルの本文に細かい制約を入れない

費用やコマンドのように HTML で集計したいものを、本文の中の YAML のコードブロックで書く案だった。
