---
mission: 用語集（service/words.yaml）の項目と、それぞれの形
tags: [仕様]
---

# 用語集のスキーマ

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "用語集（service/words.yaml）",
  "description": "このプロジェクトの言葉の意味",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "groups": {
      "description": "用語のまとまりの配列。HTML では、まとまりの名前を見出しにし、用語を表にする",
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": [
          "name"
        ],
        "properties": {
          "name": {
            "description": "まとまりの名前",
            "type": "string"
          },
          "items": {
            "description": "用語の配列",
            "type": "array",
            "items": {
              "type": "object",
              "additionalProperties": false,
              "required": [
                "desc_markdown"
              ],
              "anyOf": [
                {
                  "required": [
                    "ja"
                  ]
                },
                {
                  "required": [
                    "en"
                  ]
                }
              ],
              "properties": {
                "ja": {
                  "description": "日本語の名前。`ja` と `en` はどちらかがあればよい",
                  "type": "string"
                },
                "en": {
                  "description": "英語の名前。`ja` と `en` はどちらかがあればよい",
                  "type": "string"
                },
                "desc_markdown": {
                  "description": "意味。Markdown。`|` のブロックで書く。",
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
