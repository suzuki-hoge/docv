# 利用マニュアル

## コマンド一覧

| コマンド                                       | 説明                                 | 実行ディレクトリ |
| ---------------------------------------------- | ------------------------------------ | ---------------- |
| [`docv init`](#docv-init)                      | `docv.yaml` を作る                   | 任意             |
| [`docv desc`](#docv-desc)                      | docv の概要を出す                    | 任意             |
| [`docv check [path...]`](#docv-check)          | 管理対象のチェックを行う             | docv ルート      |
| [`docv fix [path...]`](#docv-fix)              | 管理対象のフォーマットとリントを行う | docv ルート      |
| [`docv build`](#docv-build)                    | 人間向けの HTML を作る               | docv ルート      |
| [`docv slot list`](#docv-slot-list)            | Slot の一覧を出す                    | 任意             |
| [`docv slot schema <slot>`](#docv-slot-schema) | YAML の Slot のスキーマを出す        | 任意             |
| [`docv slot gen <slot>`](#docv-slot-gen)       | Slot ファイルを、決まったパスに作る  | docv ルート      |

docv ルート: `docv.yaml` のあるディレクトリ。

## コマンド詳細

### docv init

`docv.yaml` を作る。

| オプション | 意味                                   |
| ---------- | -------------------------------------- |
| `--print`  | `docv.yaml` を作らずに、書く中身を出す |
| `--force`  | `docv.yaml` がすでにあっても書き直す   |

### docv desc

docv の概要を表示する。

AI Agent にはこれを入力にして進めさせる。

### docv check

ドキュメントファイルを検査する。

| オプション      | 意味                                                                         |
| --------------- | ---------------------------------------------------------------------------- |
| `--no-textlint` | textlint だけを飛ばす ( 設定で textlint を有効化している場合のみ意味がある ) |

```sh
docv check
docv check docs/spec/order.md
```

検査内容は次のとおり。

- YAML front matter およびタグ
- ドキュメントのリンク切れ
- ADR どうしのリンク切れ
- Slot ファイルのスキーマ
- 太字、斜体、ドルマークの利用 ( 設定次第 )
- format ( 設定次第 )
- textlint ( 設定次第 )

現在の設定は [現在の設定](../../docv.yaml) を参照。

### docv fix

ドキュメントファイルの自動修正できるところを変更する。

修正内容は次のとおり。

- format ( 設定次第 )
- textlint が自動修正できる箇所 ( 設定次第 )

### docv build

人間向けの HTML を生成する。

- 出力先は `docv.yaml` で指定された `dir.html`
- `docv.yaml` で `dir.image` を指定すると、Slot のページを JPG にしてそこに出す
  - 要 Google Chrome または Chromium

### docv slot list

Slot の名前、作るファイル、ミッションの一覧を表示する。

### docv slot gen

Slot ファイルを、決まったパスに作る。

| オプション | 意味                                    |
| ---------- | --------------------------------------- |
| `--print`  | Slot ファイルを作らずに、書く中身を出す |
| `--force`  | Slot ファイルがすでにあっても書き直す   |

```sh
docv slot gen manual               # docv/service/manual.md を作る
docv slot gen doc-guide            # docv/guide/doc-guide.md に、ガイドラインの雛形を作る
docv slot gen doc-guide --print    # 運用中のファイルと雛形を見比べる
```

### docv slot schema

YAML の Slot のスキーマを表示する。

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
