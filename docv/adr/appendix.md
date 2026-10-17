---
group: 構造化
date: 2026-10-17
supersedes: [no-body-include]
---

# YAML の Slot に appendix_markdown を書けるようにする

## 背景

YAML の Slot の `body` と `include` をなくし、項目にない文章は Free ファイルに書いてリンクすることにした。しかし、項目に収まらない短い補足のために Free ファイルを作り、Slot からリンクするのは手間が大きい。

## 決めたこと

どの YAML の Slot にも、`appendix_markdown` を書けるようにする。HTML では、スキーマで決めた項目のあとに続けて出し、見出しは h2 から始まるように段を付け直す。ほかの Markdown の項目と同じく `|` のブロックで書き、同じ検査をかける。`include` はなくしたままにする。

## 結果

項目に収まらない補足を、Slot の中に書ける。ほかのファイルを埋め込む仕組みは持たない。
