---
mission: ディレクトリ（guide/directory-tree.yaml）には、どんな項目をどんな形で書くか
tags: [仕様]
---

# ディレクトリのスキーマ

```yaml
$schema: https://json-schema.org/draft/2020-12/schema
title: ディレクトリ（guide/directory-tree.yaml）
description: どこに何があるか
type: object
additionalProperties: false
properties:
  entries:
    description: パスと説明の配列。書いた順に、パスの深さで字下げして出す
    type: array
    items:
      type: object
      additionalProperties: false
      required: [path, description]
      properties:
        path:
          description: ルートからの相対パス
          type: string
        description:
          description: そこに何があるか
          type: string
  body:
    description: Slot のページの最後に出す Markdown
    type: string
  include:
    description: ページに埋め込む Markdown の、ルートからの相対パスの配列。`パス` はファイル全体を（YAML front matter と h1 を除く）、`パス#見出し` はその見出しの章を埋め込む
    type: array
    items:
      type: string
```
