---
mission: docv.yaml の項目と、それぞれの形
tags: [仕様]
---

# `docv.yaml` のスキーマ

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "docv.yaml",
  "description": "docv の設定ファイル。ツールの設定と、プロジェクト全体で共有する語彙を書く。ファイルごとの設定は YAML front matter に書く。項目の意味は `docs/spec/config.md` を参照。",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "title": {
      "description": "HTML とインデックスに出すプロジェクト名。省略するとルートのディレクトリ名。",
      "type": "string",
      "minLength": 1
    },
    "dir": {
      "description": "docv が読み書きするディレクトリ。どれもルートからの相対パス。どの 2 つも、同じディレクトリにも、一方が他方を含む関係にもできない。",
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "free": {
          "description": "管理対象の Markdown（Free ファイル）を置くディレクトリ。この下の `.md` のファイルがすべて対象。",
          "type": "string",
          "minLength": 1,
          "default": "docs"
        },
        "slot": {
          "description": "Slot ファイルを置くディレクトリ。",
          "type": "string",
          "minLength": 1,
          "default": "docv"
        },
        "html": {
          "description": "docv build が HTML を出すディレクトリ。",
          "type": "string",
          "minLength": 1,
          "default": "output/docv/html"
        },
        "image": {
          "description": "docv build が Slot のページの JPG を出すディレクトリ。書かなければ JPG を作らない。JPG を作るには Chrome が要る。",
          "type": "string",
          "minLength": 1
        }
      }
    },
    "ignore": {
      "description": "管理対象にも管理外にもしないファイルの glob。",
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 1
      }
    },
    "tags": {
      "description": "タグの一覧。YAML front matter の `tags` には、ここにあるタグだけを書ける。",
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": [
          "name",
          "color"
        ],
        "properties": {
          "name": {
            "type": "string",
            "minLength": 1
          },
          "color": {
            "description": "HTML でのタグの色。",
            "enum": [
              "blue",
              "green",
              "yellow",
              "red",
              "purple",
              "orange",
              "teal",
              "pink"
            ]
          }
        }
      }
    },
    "format": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "enabled": {
          "description": "check で、整形が要るファイルを error にするか。",
          "type": "boolean",
          "default": false
        }
      }
    },
    "lint": {
      "description": "check の規則。項目の意味は `docs/spec/config.md` を参照。",
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "bold": {
          "description": "太字を使ってよいか。",
          "enum": [
            "allow",
            "deny"
          ],
          "default": "allow"
        },
        "italic": {
          "description": "斜体を使ってよいか。",
          "enum": [
            "allow",
            "deny"
          ],
          "default": "allow"
        },
        "dollar": {
          "description": "コードの外の 1 行に `$` を 2 つ以上書いてよいか。",
          "enum": [
            "allow",
            "deny"
          ],
          "default": "allow"
        },
        "textlint": {
          "description": "textlint で文章を校正するか。使うルールは docv が決める。",
          "enum": [
            "enable",
            "disable"
          ],
          "default": "disable"
        }
      }
    }
  }
}
```
