---
group: 構造化
date: 2026-10-17
---

# YAML の Slot の body と include をなくす

## 背景

YAML の Slot には、決まった項目のほかに、ページの最後に出す Markdown（`body`）と、ほかの Markdown を埋め込む指定（`include`）を書けた。このリポジトリでも、テンプレでも、どちらも使っていなかった。項目の外に文章を書けると、Slot の形を決めた意味が薄れる。

## 決めたこと

`body` と `include` をなくす。項目にない文章は、Free ファイルに書いてリンクする。

## 結果

YAML の Slot のページは、決まった項目だけで組み立てる。埋め込みの範囲を決める規則と、埋め込み先を確かめる check（`slot-include-missing`）が要らなくなる。
