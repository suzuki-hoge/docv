---
group: コマンド
date: 2026-10-12
---

# AI Agent 向けの一覧を出すコマンドを持たない

## 背景

AI Agent が読むファイルを選べるように、枠、原本、ADR の一覧と、それぞれが答える問いを出す `docv index` を持っていた。

## 決めたこと

`docv index` をなくす。AI Agent は、docv/ の枠と、docs/ の原本の frontmatter の mission を見て、読むファイルを選ぶ。

## 結果

コマンドが 1 つ減る。CLAUDE.md などには `docv desc` の実行だけを書けばよい。

## 選ばなかった案

| 案 | 選ばなかった理由 |
| --- | --- |
| `docv index` を残す | 枠は docv/ に、原本は docs/ に分かれていて、どこに何があるかは置き場所で自明。一覧を出すコマンドに価値がない |
