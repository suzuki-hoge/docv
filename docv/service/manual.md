---
mission: 利用者の使い方（CLI のコマンド、GUI の操作など）
---

# 利用マニュアル

## コマンド一覧

| コマンド                                       | 説明                                 | 実行ディレクトリ |
| ---------------------------------------------- | ------------------------------------ | ---------------- |
| [`docv init`](#docv-init)                      | `docv.yaml` を作る                   | 任意             |
| [`docv desc`](#docv-desc)                      | docv の概要を出す                    | 任意             |
| [`docv check [path...]`](#docv-check)          | 管理対象のチェックを行う             | docv ルート      |
| [`docv fix [path...]`](#docv-fix)              | 管理対象のフォーマットとリントを行う | docv ルート      |
| [`docv build`](#docv-build)                    | 人間向けの HTML を作る               | docv ルート      |
| [`docv open`](#docv-open)                      | 作った HTML をブラウザで開く         | docv ルート      |
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

対象ファイルと主な検査内容は次のとおり。

| 検査内容                                  | Slot ( md ) | Slot ( yaml )          | Free ( md ) | ADR ( md ) |
| ----------------------------------------- | ----------- | ---------------------- | ----------- | ---------- |
| YAML front matter およびタグ              | ✅          |                        | ✅          | ✅         |
| ドキュメントのリンク切れ                  | ✅          | ✅ ( markdown 部のみ ) | ✅          | ✅         |
| 見出しの数と章番号                        | ✅          |                        | ✅          | ✅         |
| Slot ファイルの名前とスキーマ             | ✅          | ✅                     |             |            |
| 太字、斜体、ドルマークの利用 ( 設定次第 ) | ✅          | ✅ ( markdown 部のみ ) | ✅          | ✅         |
| format ( 設定次第 )                       | ✅          | ✅ ( markdown 部のみ ) | ✅          | ✅         |
| textlint ( 設定次第 )                     | ✅          | ✅ ( markdown 部のみ ) | ✅          | ✅         |

Slot ( md ) の YAML front matter は、`mission` が `docv slot list` のミッションと同じかを検査する。ページに分けたサービス仕様とシステム仕様は、YAML front matter の `order` と、ページどうしの `order` と h1 の重なりも検査する。

Slot ( yaml ) の markdown 部は、名前が `_markdown` で終わる項目。値は `|` のブロックで書く。`|` で書いていない項目と、それ以外の項目を複数行で書いたものは、スキーマの検査で error にする。

パスを渡しても、検査するファイルが絞られるだけで、検査内容は変わらない。

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
  - JPG はマシンのフォントで変わる。チームで共有するなら、GitHub Actions などの決まった環境で作る
- `docv.yaml` で `dir.index` を指定すると、Slot ファイルへのリンクを並べた `docv-index.md` を書く
  - README からリンクすると、GitHub でリポジトリを開いた人が、clone しなくても Slot ファイルを読める
  - YAML の Slot は JPG へリンクするので、`dir.image` も指定し、JPG を git で管理する

| オプション   | 意味                                                         |
| ------------ | ------------------------------------------------------------ |
| `--no-image` | `dir.image` を指定していても JPG を作らない。HTML だけを作る |

### docv open

`docv build` が `dir.html` に作ったプロジェクト概要（`index.html`）を、既定のブラウザで開く。

- macOS は `open`、Windows は `explorer` で開く
- ほかの OS には対応しない。対応していないと表示し、終了コード 1 で終わる
- まだ `docv build` していなければ、何も開かずにエラーで終わる

### docv slot list

Slot の名前、作るファイル、ミッションの一覧を表示する。

### docv slot gen

Slot ファイルを、決まったパスに作る。

YAML の Slot には、1 行目にスキーマの URL を書く。yaml-language-server に対応したエディタ（VS Code の YAML の拡張など）で、項目の補完と検証が効く。

md の Slot には、YAML front matter の `mission` にミッションを書く。

| オプション | 意味                                    |
| ---------- | --------------------------------------- |
| `--print`  | Slot ファイルを作らずに、書く中身を出す |
| `--force`  | Slot ファイルがすでにあっても書き直す   |

```sh
docv slot gen manual               # docv/service/manual.md を作る
docv slot gen doc-guide            # docv/guide/doc-guide.md に、ガイドラインの雛形を作る
docv slot gen doc-guide --print    # 運用中のファイルと雛形を見比べる
```

サービス仕様とシステム仕様をページに分けるときは、`docv/service/service/` か `docv/system/system/` の下にページを手で置く（[ページ分け](service/desc.md#ページ分け)）。ページがあるときは、`docv slot gen service` と `docv slot gen system` は、1 つのファイルを作らずにエラーで終わる。

### docv slot schema

YAML の Slot のスキーマ（JSON Schema）を表示する。

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
