# 利用マニュアル

## コマンド一覧

| コマンド                                | 説明                                                                       | 実行ディレクトリ               |
| --------------------------------------- | -------------------------------------------------------------------------- | ------------------------------ |
| [`docv init`](#docv-init)               | `docv.yaml` を作る                                                         | 任意                           |
| [`docv add <slot>`](#docv-add)          | Slot ファイルを、決まったパスに作る                                        | `docv.yaml` のあるディレクトリ |
| [`docv check [path...]`](#docv-check)   | 管理対象のファイルと Slot がルールに沿っているかを確かめる                 | `docv.yaml` のあるディレクトリ |
| [`docv fix [path...]`](#docv-fix)       | 管理対象のファイルと `docv/` の md を整形し、textlint で直せるところを直す | `docv.yaml` のあるディレクトリ |
| [`docv build-html`](#docv-build-html)   | 人間向けの HTML を作る                                                     | `docv.yaml` のあるディレクトリ |
| [`docv build-image`](#docv-build-image) | Slot のページを JPG にする                                                 | `docv.yaml` のあるディレクトリ |
| [`docv desc`](#docv-desc)               | docv の概要を出す                                                          | 任意                           |
| [`docv schema [slot]`](#docv-schema)    | `docv.yaml` か、YAML の Slot のスキーマを出す                              | 任意                           |

## コマンド詳細

### docv init

`docv.yaml` を作る。

### docv add

Slot ファイルを、決まったパスに作る。

| オプション | 意味                     |
| ---------- | ------------------------ |
| `--list`   | 作れる Slot の一覧を出す |

```sh
docv add --list        # 作れる Slot の名前と、作るファイルの一覧を出す
docv add manual        # docv/service/manual.md を作る
docv add api-md        # docv/system/api.md を作る
```

- Slot ファイルの名前は変更できない
- `docv/` は `slots.dir` で変更可能

### docv check

ドキュメントファイルを検査する。

| オプション      | 意味                                                             |
| --------------- | ---------------------------------------------------------------- |
| `--no-textlint` | textlint だけを飛ばす ( 設定で有効化している場合のみ意味がある ) |

```sh
docv check
docv check docs/spec/order.md
```

- 対象は `docv/` と `docs/` のファイル ( それぞれ設定で変更可能 )
- 検査内容は YAML front matter やリンク切れ、設定次第で textlint や format も検査する

### docv fix

ドキュメントファイルの自動修正できるところを変更する。

### docv build-html

人間向けの HTML を生成する。

- 出力先は `docv.yaml` で指定された `output.dir`

### docv build-image

HTML を JPG にする。

- 実行には Google Chrome または Chromium が必要

### docv desc

docv の概要を表示する。

- `docv.yaml` がなくても動く

### docv schema

`docv.yaml` か、Slot のスキーマを表示する。

```sh
docv schema           # 設定ファイル ( docv.yaml ) のスキーマを表示
docv schema words     # 用語集 Slot のスキーマを表示
```

## スキーマ一覧

- [`docv.yaml`](../../docs/schema/config.md)
- [インストール](../../docs/schema/install.md)
- [用語集](../../docs/schema/words.md)
- [技術スタック](../../docs/schema/stack.md)
- [API 仕様書](../../docs/schema/api-yaml.md)
- [DB 仕様書](../../docs/schema/db-yaml.md)
- [ディレクトリ](../../docs/schema/directory-tree.md)
- [環境構築](../../docs/schema/setup.md)
- [環境変数](../../docs/schema/env.md)
- [開発コマンド](../../docs/schema/develop.md)
- [費用](../../docs/schema/cost.md)
