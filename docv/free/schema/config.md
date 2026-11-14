---
mission: docv.yaml の項目と、それぞれの形
tags: [スキーマ]
---

# `docv.yaml` のスキーマ

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "docv.yaml",
  "description": "docv の設定ファイル。ツールの設定と、プロジェクト全体で共有する語彙を書く。ファイルごとの設定は YAML front matter に書く。項目の意味は `docv/slot/service/service/config.md` を参照。",
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "docv_version": {
      "description": "ドキュメントを合わせた docv の版（`1.8.0` の形）。docv の版と違えば、docv check が error にする。docv を更新したら、`docv update-guide --from <この版>` で変わったことと対応を確かめ、対応してから書き換える。",
      "type": "string",
      "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+$"
    },
    "title": {
      "description": "HTML とインデックスに出すプロジェクト名。省略するとルートのディレクトリ名。",
      "type": "string",
      "minLength": 1
    },
    "dir": {
      "description": "docv が読み書きするディレクトリ。どちらもルートからの相対パスで、ルートは書けない。`output` は、`document_root` と同じにも、その中にもできない。`document_root` も、`output` の中にはできない。",
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "document_root": {
          "description": "ドキュメントを置くディレクトリ。この下に、`free/`（Free ファイル）、`slot/`（Slot ファイル）、`adr/`、`pdr/` と、docv build が作るインデックスの `README.md` を置く。直下にほかのものは置けない。",
          "type": "string",
          "minLength": 1,
          "default": "docv"
        },
        "output": {
          "description": "docv build の出力先。この下の `html/` に HTML を、`image/` に JPG を出す。AI Agent が `document_root` を読むときに同じ内容が倍入らないように、`document_root` の外に置く。",
          "type": "string",
          "minLength": 1,
          "default": "output/docv"
        }
      }
    },
    "docv_exclude": {
      "description": "docv が読まないファイルの glob。ルートからの相対パス。当てはまるファイルは、check（置き場所の検査も含む）、fix、整形、textlint、HTML、検索、インデックスのどれの対象にもならない。メモや下書きの置き場所に使う。",
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 1
      }
    },
    "generate": {
      "description": "docv build が作るもの。",
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "image": {
          "description": "YAML の Slot のページを JPG にして、`dir.output` の下の `image/` に出すか。JPG を作るには Chrome が要る。",
          "type": "boolean",
          "default": false
        }
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
      "description": "check の規則。項目の意味は `docv/slot/service/service/config.md` を参照。",
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
