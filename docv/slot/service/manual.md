---
mission: 利用者の使い方（CLI のコマンド、GUI の操作など）
---

# 利用マニュアル

## コマンド一覧

| コマンド                                                   | 説明                                                                         | 実行ディレクトリ |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------- |
| [`docv init`](#docv-init)                                  | `docv.yaml` を作る                                                           | 任意             |
| [`docv desc`](#docv-desc)                                  | docv の概要を出す                                                            | 任意             |
| [`docv update-guide --from <version>`](#docv-update-guide) | docv を更新したときの変更点と対応を出す                                      | 任意             |
| [`docv review-guide`](#docv-review-guide)                  | ドキュメントをレビューする手順を出す                                         | docv ルート      |
| [`docv check [path...]`](#docv-check)                      | 管理対象のチェックを行う                                                     | docv ルート      |
| [`docv fix [path...]`](#docv-fix)                          | 管理対象のフォーマットとリントを行う                                         | docv ルート      |
| [`docv build`](#docv-build)                                | 人間向けの HTML を作る                                                       | docv ルート      |
| [`docv open`](#docv-open)                                  | 作った HTML をブラウザで開く                                                 | docv ルート      |
| [`docv diff-open [--from <rev>]`](#docv-diff-open)         | git のリビジョンから変わったファイルに印を付けた HTML を作り、ブラウザで開く | docv ルート      |
| [`docv slot list`](#docv-slot-list)                        | Slot の一覧を出す                                                            | 任意             |
| [`docv slot schema <slot>`](#docv-slot-schema)             | YAML の Slot のスキーマを出す                                                | 任意             |
| [`docv slot gen <slot>`](#docv-slot-gen)                   | Slot ファイルを、決まったパスに作る                                          | docv ルート      |

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

### docv update-guide

docv を更新したときに、版ごとの変更点と対応を表示する。

| オプション         | 意味                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------- |
| `--from <version>` | 更新する前に使っていた docv の版。`docv.yaml` の `docv_version` を渡す。省略はできない |

- `--from` の版より後の版から、今の docv の版までを、古い版から順に出す。対応のない版も出す
- 変更 1 つを 1 行にし、何が変わったかと、それへの対応を書く。対応は、要るもの、選ぶもの、要らないものがある
- どの行も 100 文字以内にする。横に長いと読みにくいため
- 最後に、`docv_version` と、`docv.yaml` と YAML の Slot の 1 行目にあるスキーマの URL の版を上げるよう案内する
- `--from` には `1.4.0` の形のほか、`v1.4.0` の形も書ける。最初の版（1.0.0）より前か、今の docv の版より新しければ、終了コード 2 で終わる
- docv を更新した後は、`docv check` が `docv_version` の違いを指摘し、`--from` に渡す版を案内する（[設定ファイル](service/config.md) の「docv の版」の章）

```sh
docv update-guide --from 1.4.0
```

docv 1.5.0 で実行したときの出力は次のとおり（変更の一部を省いた）。

```
1.5.0 変更点
- 開発コマンドとインフラと環境のページ分けを追加: 長いなら、docv desc の「ページ分け」に従って分けられる。開発コマンドのページには order と title を書く
- インストールと環境構築の手順を、h3 の見出しで出すように変更: 対応は要らない

対応が終わったら、docv.yaml の docv_version を 1.5.0 にする
docv.yaml と YAML の Slot の 1 行目にあるスキーマの URL の版も、v1.5.0 にする
最後に docv check で、残りがないかを確かめる
```

### docv review-guide

溜まったドキュメントをレビューする手順と、レビューするファイルの一覧を表示する。docv は指摘を出さない。出力を AI Agent に渡し、AI Agent が指摘を出す。

```sh
docv review-guide
```

1. AI Agent に「`docv review-guide` を実行し、その手順でドキュメントをレビューして」と頼む
2. AI Agent が、番号付きの指摘と、選べる直し方を出す
   - この時点では、ファイルを書き換えない
3. 指摘ごとに、番号で直し方を選ぶ
   - 直さないものは「このままでいい」と答える
4. AI Agent が、選んだ直し方で直す

手順は、`docv build` が作る HTML の Review Guide のページでも読める。

### docv check

ドキュメントファイルを検査する。

| オプション      | 意味                                                                         |
| --------------- | ---------------------------------------------------------------------------- |
| `--no-textlint` | textlint だけを飛ばす ( 設定で textlint を有効化している場合のみ意味がある ) |

```sh
docv check
docv check docv/free/spec/order.md
```

対象ファイルと主な検査内容は次のとおり。

| 検査内容                                  | Slot ( md ) | Slot ( yaml )          | Free ( md ) | ADR / PDR ( md ) |
| ----------------------------------------- | ----------- | ---------------------- | ----------- | ---------------- |
| YAML front matter およびタグ              | ✅          |                        | ✅          | ✅               |
| ドキュメントのリンク切れ                  | ✅          | ✅ ( markdown 部のみ ) | ✅          | ✅               |
| 見出しの数と章番号                        | ✅          |                        | ✅          | ✅               |
| Slot ファイルの名前とスキーマ             | ✅          | ✅                     |             |                  |
| 太字、斜体、ドルマークの利用 ( 設定次第 ) | ✅          | ✅ ( markdown 部のみ ) | ✅          | ✅               |
| format ( 設定次第 )                       | ✅          | ✅ ( markdown 部のみ ) | ✅          | ✅               |
| textlint ( 設定次第 )                     | ✅          | ✅ ( markdown 部のみ ) | ✅          | ✅               |

Slot ( md ) の YAML front matter は、`mission` が `docv slot list` のミッションと同じかを検査する。ページに分けた Slot は、`order` とページの名前（md は h1、YAML は `title`）があるかと、同じ Slot のページどうしの重なりも検査する。

Slot ( yaml ) の markdown 部は、名前が `_markdown` で終わる項目。値は `|` のブロックで書く。`|` で書いていない項目と、それ以外の項目を複数行で書いたものは、スキーマの検査で error にする。

あわせて、`docv.yaml` の `docv_version` を docv の版と比べる。`docv.yaml` と YAML の Slot の 1 行目にあるスキーマの URL の版も、`docv_version` と比べる（[設定ファイル](service/config.md) の「docv の版」の章）。

パスを渡しても、検査するファイルが絞られるだけで、検査内容は変わらない。

現在の設定は [現在の設定](../../../docv.yaml) を参照。

### docv fix

ドキュメントファイルの自動修正できるところを変更する。

修正内容は次のとおり。

- format ( 設定次第 )
- textlint が自動修正できる箇所 ( 設定次第 )

### docv build

人間向けの HTML を生成する。

- 出力先は `docv.yaml` の `dir.output` の下の `html/`
- `docv.yaml` で `generate.image` を true にすると、YAML の Slot のページを JPG にして `dir.output` の下の `image/` に出す
  - 要 Google Chrome または Chromium
  - JPG はマシンのフォントで変わる。チームで共有するなら、GitHub Actions などの決まった環境で作る
- `document_root` の直下に、Slot ファイルへのリンクを並べたインデックス（`README.md`）を毎回書く
  - ルートの README からリンクすると、GitHub でリポジトリを開いた人が、clone しなくても Slot ファイルを読める
  - YAML の Slot は JPG へリンクするので、`generate.image` を true にし、JPG を git で管理する

| オプション   | 意味                                                                        |
| ------------ | --------------------------------------------------------------------------- |
| `--no-image` | `generate.image` が true でも JPG を作らない。HTML とインデックスだけを作る |

### docv open

`docv build` が `dir.output` の下の `html/` に作った Slot のページ（`index.html`）を、既定のブラウザで開く。

- macOS は `open`、Windows は `explorer` で開く
- ほかの OS には対応しない。対応していないと表示し、終了コード 1 で終わる
- まだ `docv build` していなければ、何も開かずにエラーで終わる

### docv diff-open

`--from` のリビジョンと作業ツリーを git で比べ、変わったファイルに印を付けた HTML を作って、既定のブラウザで開く。このままコミットしたら、ドキュメントのどこが変わるかを見るために使う。

- 比べるのは、`docv.yaml`、ルートの `README.md`、`document_root` の中のファイル
  - インデックス（`document_root` の `README.md`）と、`docv_exclude` に当たるファイルは比べない
  - まだ git に追加していないファイルも、新しく足したファイルとして比べる
  - docv の版を上げただけの行（`docv.yaml` と YAML の Slot の 1 行目のスキーマの URL、`docv_version`）は、差分に数えない。版のほかに変わっていないファイルには、印を付けない
- ページの中には差分を出さない
  - 元のファイルのパスの代わりに `git diff` のコマンドを出すので、詳しい差分はそれを実行して見る
- HTML は OS のキャッシュのディレクトリに書き、`dir.output` には書かない。実行するたびに作り直す
- 差分がなければ、何も作らずに、差分がないと表示して終わる
- git の管理下にないときと、`--from` がリビジョンでないときは、エラーで終わる
- `docv open` と同じく、macOS と Windows だけに対応する

| オプション     | 意味                                                                            |
| -------------- | ------------------------------------------------------------------------------- |
| `--from <rev>` | 比べる git のリビジョン（ブランチ、タグ、コミット）。省略すると `HEAD` と比べる |

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
docv slot gen manual               # docv/slot/service/manual.md を作る
docv slot gen doc-guide            # docv/slot/guide/doc-guide.md に、ガイドラインの雛形を作る
docv slot gen doc-guide --print    # 運用中のファイルと雛形を見比べる
```

Slot をページに分けるときは、`docv/slot/service/service/` などのページのディレクトリの下にページを手で置く（[ページ分け](service/desc.md#ページ分け)）。ページがあるときは、`docv slot gen service` などは、1 つのファイルを作らずにエラーで終わる。

### docv slot schema

YAML の Slot のスキーマ（JSON Schema）を表示する。

```sh
docv slot schema words # 用語集のスキーマを表示
```

## スキーマ一覧

- [`docv.yaml`](../../free/schema/config.md)
- [インストール](../../free/schema/install.md)
- [用語集](../../free/schema/words.md)
- [技術スタック](../../free/schema/stack.md)
- [環境構築](../../free/schema/setup.md)
- [環境変数](../../free/schema/env.md)
- [開発コマンド](../../free/schema/develop.md)
- [費用](../../free/schema/cost.md)
