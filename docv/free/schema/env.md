---
mission: 環境変数（guide/env.yaml）の項目と、それぞれの形
tags: [スキーマ]
---

# 環境変数のスキーマ

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "環境変数（guide/env.yaml）",
  "description": "システムを動かすのに要る環境変数と、それぞれを設定している場所",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "envs": {
      "description": "環境変数の配列",
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": [
          "key",
          "desc_markdown"
        ],
        "properties": {
          "key": {
            "description": "環境変数の名前",
            "type": "string"
          },
          "desc_markdown": {
            "description": "何に使うか。Markdown。`|` のブロックで書く。",
            "type": "string"
          },
          "secret": {
            "description": "秘密の値か",
            "type": "boolean",
            "default": false
          },
          "local_from_markdown": {
            "description": "ローカル環境で、どこで設定しているか（Docker Compose、`.env` など）。Markdown。`|` のブロックで書く。",
            "type": "string"
          },
          "release_from_markdown": {
            "description": "ステージングや本番で、どこで設定しているか（ECS のタスク定義、SSM など）。Markdown。`|` のブロックで書く。",
            "type": "string"
          }
        }
      }
    },
    "appendix_markdown": {
      "description": "項目のあとに続けて、ページに出す文章。Markdown。`|` のブロックで書く。",
      "type": "string"
    }
  }
}
```
