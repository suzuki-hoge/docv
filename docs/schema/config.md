---
mission: docv.yaml には、どんな項目をどんな形で書くか
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
  "required": [
    "docs"
  ],
  "properties": {
    "title": {
      "description": "HTML とインデックスに出すプロジェクト名。省略するとルートのディレクトリ名。",
      "type": "string",
      "minLength": 1
    },
    "docs": {
      "description": "管理対象のファイルの glob。ルートからの相対パス。`.md` のファイルだけが対象。",
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 1
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
      "description": "タグの一覧。書いたときだけ、YAML front matter の `tags` をこの中に限る。",
      "type": "array",
      "items": {
        "type": "string",
        "minLength": 1
      },
      "uniqueItems": true
    },
    "slots": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "dir": {
          "description": "Slot ファイルを置くディレクトリ。",
          "type": "string",
          "default": "docv"
        }
      }
    },
    "output": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "dir": {
          "description": "docv build の出力先。",
          "type": "string",
          "default": "docv/build"
        },
        "commit": {
          "description": "出力を git で管理するか。false なら出力先に `.gitignore` を置く。",
          "type": "boolean",
          "default": true
        },
        "image": {
          "description": "docv build で、Slot のページを JPG にするか。true なら Chrome が要る。",
          "type": "boolean",
          "default": false
        }
      }
    },
    "overrides": {
      "description": "パスごとに、道具の設定を上書きする。後に書いたものが優先する。",
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["paths"],
        "properties": {
          "paths": {
            "description": "当てるファイルの glob の配列。",
            "type": "array",
            "items": { "type": "string" }
          },
          "textlint": {
            "description": "textlint をかけるか。",
            "type": "boolean"
          },
          "style": {
            "description": "書式の規則（`lint.style`）をかけるか。",
            "type": "boolean"
          },
          "format": {
            "description": "整形するか。",
            "type": "boolean"
          },
          "mission": {
            "description": "YAML front matter の `mission` を必須にするか。",
            "enum": ["required", "optional"]
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
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "style": {
          "$ref": "#/$defs/style"
        },
        "textlint": {
          "$ref": "#/$defs/textlint"
        },
        "words": {
          "description": "過去の言い回しとして警告する語の調整。",
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "add": {
              "type": "array",
              "items": {
                "type": "string",
                "minLength": 1
              }
            },
            "remove": {
              "type": "array",
              "items": {
                "type": "string",
                "minLength": 1
              }
            }
          }
        },
        "tags": {
          "description": "タグごとに、そのタグを持つファイルにかけるルール。",
          "type": "object",
          "additionalProperties": {
            "type": "array",
            "items": {
              "enum": [
                "no-past-wording",
                "no-done-task"
              ]
            },
            "uniqueItems": true
          }
        }
      }
    }
  },
  "$defs": {
    "style": {
      "description": "書式の規則。項目の意味は `docs/spec/config.md` の 5 章を参照。書いた規則だけを確かめる。false を書くと使わない。",
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "bold": {
          "enum": [
            "deny",
            false
          ]
        },
        "italic": {
          "enum": [
            "deny",
            false
          ]
        },
        "html": {
          "description": "使ってよい HTML タグの配列。[] ならすべて禁止。",
          "oneOf": [
            {
              "type": "array",
              "items": {
                "type": "string",
                "pattern": "^[a-z][a-z0-9-]*$"
              },
              "uniqueItems": true
            },
            {
              "const": false
            }
          ]
        },
        "dollar": {
          "enum": [
            "deny",
            false
          ]
        },
        "heading_number": {
          "enum": [
            "require",
            false
          ]
        },
        "section_separator": {
          "enum": [
            "require",
            false
          ]
        },
        "link_text": {
          "oneOf": [
            {
              "type": "object",
              "additionalProperties": false,
              "required": [
                "base"
              ],
              "properties": {
                "base": {
                  "description": "表示文字のパスの起点のディレクトリ。",
                  "type": "string",
                  "minLength": 1
                }
              }
            },
            {
              "const": false
            }
          ]
        },
        "image_alt": {
          "enum": [
            "require",
            false
          ]
        }
      }
    },
    "textlint": {
      "description": "textlint の設定。項目の意味は `docs/spec/config.md` の 6 章を参照。",
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "enabled": {
          "type": "boolean",
          "default": false
        },
        "level": {
          "enum": [
            "warn",
            "error"
          ],
          "default": "warn"
        },
        "presets": {
          "type": "array",
          "items": {
            "enum": [
              "ja-technical-writing",
              "ja-spacing",
              "jtf-style"
            ]
          },
          "uniqueItems": true
        },
        "rules": {
          "description": "`<プリセット>/<ルール>` をキーに、false か、ルールのオプションを書く。",
          "type": "object",
          "propertyNames": {
            "pattern": "^(ja-technical-writing|ja-spacing|jtf-style)/[a-z0-9-]+$"
          },
          "additionalProperties": {
            "oneOf": [
              {
                "const": false
              },
              {
                "type": "object"
              }
            ]
          }
        },
        "prh": {
          "description": "表記ゆれの辞書のパス。",
          "type": "array",
          "items": {
            "type": "string",
            "minLength": 1
          }
        },
        "config": {
          "description": "持ち込みの使い方で使う、textlint の設定ファイルのパス。",
          "type": "string",
          "minLength": 1
        }
      },
      "not": {
        "required": [
          "config"
        ],
        "anyOf": [
          {
            "required": [
              "presets"
            ]
          },
          {
            "required": [
              "rules"
            ]
          },
          {
            "required": [
              "prh"
            ]
          }
        ]
      }
    }
  }
}
```
