# 利用マニュアル

## コマンド一覧

| コマンド                                       | 説明                                                                       | 実行ディレクトリ               |
| ---------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------ |
| [`docv init`](#docv-init)                      | `docv.yaml` を作る                                                         | 任意                           |
| [`docv desc`](#docv-desc)                      | docv の概要を出す                                                          | 任意                           |
| [`docv check [path...]`](#docv-check)          | 管理対象のファイルと Slot がルールに沿っているかを確かめる                 | `docv.yaml` のあるディレクトリ |
| [`docv fix [path...]`](#docv-fix)              | 管理対象のファイルと `docv/` の md を整形し、textlint で直せるところを直す | `docv.yaml` のあるディレクトリ |
| [`docv build`](#docv-build)                    | 人間向けの HTML を作る                                                     | `docv.yaml` のあるディレクトリ |
| [`docv slot list`](#docv-slot-list)            | Slot の一覧を出す                                                          | 任意                           |
| [`docv slot gen <slot>`](#docv-slot-gen)       | Slot ファイルを、決まったパスに作る                                        | `docv.yaml` のあるディレクトリ |
| [`docv slot schema <slot>`](#docv-slot-schema) | YAML の Slot のスキーマを出す                                              | 任意                           |

## コマンド詳細

### docv init

`docv.yaml` を作る。

| オプション | 意味                                   |
| ---------- | -------------------------------------- |
| `--print`  | `docv.yaml` を作らずに、書く中身を出す |
| `--force`  | `docv.yaml` がすでにあっても書き直す   |

- すべての項目を、既定の値でコメントに載せる。`docv init --print` で、使える項目を確認できる
- 作った後に、`docv slot gen doc-guide` でガイドラインの雛形を作るよう案内する

### docv desc

docv の概要を表示する。

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

### docv build

人間向けの HTML を生成する。

- 出力先は `docv.yaml` で指定された `dir.html`
- `dir.image` を書いていれば、Slot のページを JPG にして、そこに出す。Google Chrome または Chromium が必要

### docv slot list

Slot の名前、作るファイル、ミッションの一覧を表示する。

### docv slot gen

Slot ファイルを、決まったパスに作る。

| オプション | 意味                                    |
| ---------- | --------------------------------------- |
| `--print`  | Slot ファイルを作らずに、書く中身を出す |
| `--force`  | Slot ファイルがすでにあっても書き直す   |

```sh
docv slot gen manual            # docv/service/manual.md を作る
docv slot gen doc-guide         # docv/guide/doc-guide.md に、ガイドラインの雛形を作る
docv slot gen doc-guide --print # 今の版の雛形を出す。手元のガイドラインと見比べる
```

- yaml の Slot には、項目のサンプルを書く
- md の Slot は空のファイルを作り、中身の形は自由で、Slot のミッションに沿って Markdown を書けばよいことを表示する
- Doc ガイド（`doc-guide`）だけは、ガイドラインの雛形を書く
- Slot ファイルの名前は変更できない。`docv/` は `dir.slots` で変更可能

### docv slot schema

YAML の Slot のスキーマを表示する。md の Slot はスキーマを持たない。

```sh
docv slot schema words # 用語集のスキーマを表示
```

## スキーマ一覧

- [`docv.yaml`](../../docs/schema/config.md)
- [インストール](../../docs/schema/install.md)
- [用語集](../../docs/schema/words.md)
- [技術スタック](../../docs/schema/stack.md)
- [ディレクトリ](../../docs/schema/directory-tree.md)
- [環境構築](../../docs/schema/setup.md)
- [環境変数](../../docs/schema/env.md)
- [開発コマンド](../../docs/schema/develop.md)
- [費用](../../docs/schema/cost.md)
