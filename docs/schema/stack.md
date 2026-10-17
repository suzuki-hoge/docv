---
mission: 技術スタック（system/stack.yaml）の項目と、それぞれの形
tags: [仕様]
---

# 技術スタックのスキーマ

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "技術スタック（system/stack.yaml）",
  "description": "作るのに使っている言語、ライブラリ、サービス",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "stack": {
      "description": "層の配列。書いた順に出す",
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": [
          "group"
        ],
        "properties": {
          "group": {
            "description": "層の名前（フロントエンド、バックエンドなど）",
            "type": "string"
          },
          "image": {
            "description": "図の、ルートからの相対パス。あれば出す",
            "type": "string"
          },
          "items": {
            "description": "使っているものの配列",
            "type": "array",
            "items": {
              "type": "object",
              "additionalProperties": false,
              "required": [
                "name"
              ],
              "properties": {
                "name": {
                  "description": "名前",
                  "type": "string"
                },
                "version": {
                  "description": "版",
                  "type": "string"
                },
                "purpose_markdown": {
                  "description": "何に使うか。Markdown。`|` のブロックで書く。",
                  "type": "string"
                },
                "url": {
                  "description": "詳しく書いてあるページの URL",
                  "type": "string"
                }
              }
            }
          }
        }
      }
    }
  }
}
```
