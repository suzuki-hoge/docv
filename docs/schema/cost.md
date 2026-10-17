---
mission: 費用（operation/cost.yaml）の項目と、それぞれの形
tags: [仕様]
---

# 費用のスキーマ

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "費用（operation/cost.yaml）",
  "description": "かかっている費用と、費用を理由にやっていないこと",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "accepted_items": {
      "description": "費用をかけて使っているもの",
      "type": "array",
      "items": {
        "$ref": "#/$defs/item"
      }
    },
    "rejected_items": {
      "description": "費用を理由にやっていないこと。予算が変われば、`accepted_items` との間で項目を移す",
      "type": "array",
      "items": {
        "$ref": "#/$defs/item"
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
      "required": [
        "name"
      ],
      "properties": {
        "name": {
          "description": "名前",
          "type": "string"
        },
        "desc_markdown": {
          "description": "何のためか、なぜやらないか。Markdown。`|` のブロックで書く。",
          "type": "string"
        },
        "monthly": {
          "description": "1 か月あたりの金額。数でも文字列（「3 ドル」など）でもよい",
          "type": [
            "number",
            "string"
          ]
        },
        "ref": {
          "description": "料金のページなどの、ルートからの相対パスか URL",
          "type": "string"
        }
      }
    }
  }
}
```
