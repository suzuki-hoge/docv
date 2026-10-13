---
mission: API 仕様書（system/api.yaml）には、どんな項目をどんな形で書くか
tags: [仕様]
---

# API 仕様書のスキーマ

```yaml
$schema: https://json-schema.org/draft/2020-12/schema
title: API 仕様書（system/api.yaml）
description: どんな API があり、何を受け取り、何を返すか
type: object
additionalProperties: false
required: [ref]
properties:
  ref:
    description: 外の道具で作った定義（OpenAPI など）の、ルートからの相対パスか URL。ページを作らず、`index.html` のカードから直接リンクする。説明を書きたいときは、YAML ではなく md で置く
    type: string
```
