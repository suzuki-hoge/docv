---
mission: インストール（service/install.yaml）の項目と、それぞれの形
tags: [仕様]
---

# インストールのスキーマ

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "インストール（service/install.yaml）",
  "description": "利用者がこれを入れる手順",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "requirements": {
      "description": "前提とされるもの",
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
            "description": "要る版",
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
    },
    "steps": {
      "description": "手順の配列。書いた順に出す",
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": [
          "id",
          "name"
        ],
        "properties": {
          "id": {
            "description": "手順の ID。手順ごとに重ねない",
            "type": "string"
          },
          "name": {
            "description": "手順の名前",
            "type": "string"
          },
          "items": {
            "description": "手順の中身。1 つなら配列にしなくてよい。複数あるときは HTML で `os` ごとのタブにし、どの `item` にも `os` を書く",
            "oneOf": [
              {
                "$ref": "#/$defs/item"
              },
              {
                "type": "array",
                "items": {
                  "$ref": "#/$defs/item"
                }
              }
            ]
          }
        }
      }
    },
    "appendix_markdown": {
      "description": "項目のあとに続けて、ページに出す文章。Markdown。`|` のブロックで書く。",
      "type": "string"
    }
  },
  "$defs": {
    "item": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "os": {
          "description": "タブの名前。windows、macos、linux など。1 つの手順の中で重ねない",
          "type": "string"
        },
        "body_markdown": {
          "description": "手順の中身（コマンド、説明など）。Markdown。`|` のブロックで書く。",
          "type": "string"
        }
      }
    }
  }
}
```
