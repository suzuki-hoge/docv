---
mission: 利用者の使い方（CLI のコマンド、GUI の操作など）
---

# 利用マニュアル

## コマンド一覧

| コマンド                                                   | 説明                                                                         | 実行ディレクトリ |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------- |
| [`docv init`](#docv-init)                                  | `docv.yaml` を作る                                                           | 任意             |
| [`docv desc`](#docv-desc)                                  | docv の概要を出す                                                            | 任意             |
| [`docv list`](#docv-list)                                  | Slot と Free ファイルの一覧を出す                                            | 任意             |
| [`docv update-guide --from <version>`](#docv-update-guide) | docv を更新したときの変更点と対応を出す                                      | 任意             |
| [`docv review-guide`](#docv-review-guide)                  | ドキュメントをレビューする手順を出す                                         | docv ルート      |
| [`docv check [path...]`](#docv-check)                      | 管理対象のチェックを行う                                                     | docv ルート      |
| [`docv fix [path...]`](#docv-fix)                          | 管理対象のフォーマットとリントを行う                                         | docv ルート      |
| [`docv build`](#docv-build)                                | 人間向けの HTML を作る                                                       | docv ルート      |
| [`docv open`](#docv-open)                                  | 作った HTML をブラウザで開く                                                 | docv ルート      |
| [`docv diff-open [--from <rev>]`](#docv-diff-open)         | git のリビジョンから変わったファイルに印を付けた HTML を作り、ブラウザで開く | docv ルート      |
| [`docv gen <slot>`](#docv-gen)                             | Slot ファイルを、決まったパスに作る                                          | docv ルート      |

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

### docv list

Slot と Free ファイルの一覧を表示する。AI Agent が、どこに何が書いてあり、何をどこに書くかを決めるのに使う。

```sh
docv list
```

```
Slot
* overview         docv/slot/service/overview.md          サービス概要  : 利用者に向けた、これが何で、誰の何を解決し、何ができるかの説明
* service          docv/slot/service/service/             サービス仕様  : 外から見た機能と、それぞれの振る舞いと決まり
                                                                          desc.md（概要）
                                                                          slot.md（Slot）
  deploy           docv/slot/operation/deploy.md          デプロイ      : 作ったものを環境に反映する手順

Free
  docv/free/schema/config.md  docv.yaml の項目と、それぞれの形（タグ: スキーマ）
```

- Slot は、プロジェクトに置いたかどうかによらず、docv の Slot をすべて、[Slot](service/slot.md) のページの「一覧」の章の順に出す
  - 置いた Slot には、行の頭に `*` を付ける
  - Slot ごとに、名前（`docv gen` に渡す名前）、パス、Slot の名前とミッションを出す。Slot の名前とミッションの間の `:` は、縦に揃える
  - ページに分けた Slot は、パスにディレクトリを出し、その下に、ページのファイルとページの名前を `order` の順に出す。ページの行は、ミッションの頭に揃える
  - 置いていない Slot のパスは、分けないときのファイルを出す
- Free ファイルは、パス、ミッション、タグを出す。`docv_exclude` に当たるファイルは出さない
  - `mission` のないファイルは、ミッションの代わりに「（mission がない）」と出す
  - Free ファイルがなければ、「なし」と出す
- ADR と PDR は出さない
- パスは、ルートからの相対パスで出す。開くときに、そのまま使えるようにするため
- `docv.yaml` のないディレクトリでは、Slot をすべて `*` なしで出し、Free ファイルは出さない
  - docv を入れる前に、どんな Slot があるかを見るのにも使える

### docv update-guide

docv を更新したときに、版ごとの変更点と対応を表示する。

| オプション         | 意味                                                                                   |
| ------------------ | -------------------------------------------------------------------------------------- |
| `--from <version>` | 更新する前に使っていた docv の版。`docv.yaml` の `docv_version` を渡す。省略はできない |

- `--from` の版より後の版から、今の docv の版までを、古い版から順に出す。対応のない版も出す
- 変更 1 つを 1 行にし、何が変わったかと、それへの対応を書く。対応は、要るもの、選ぶもの、要らないものがある
- どの行も 100 文字以内にする。横に長いと読みにくいため
- 最後に、`docv_version` と、`docv.yaml` の 1 行目にあるスキーマの URL の版を上げるよう案内する
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
docv.yaml の 1 行目にあるスキーマの URL の版も、v1.5.0 にする
最後に docv check で、残りがないかを確かめる
```

### docv review-guide

溜まったドキュメントをレビューする手順と、ADR と PDR の一覧を表示する。Slot ファイルと Free ファイルは、手順の中で `docv list` を実行して読む。docv は指摘を出さない。出力を AI Agent に渡し、AI Agent が指摘を出す。

```sh
docv review-guide
```

1. AI Agent に「`docv review-guide` を実行し、その手順でドキュメントをレビューして」と頼む
2. AI Agent が、ドキュメントだけで見るか、コードやインフラの定義も調べるかを確かめる
   - 頼むときに言っておけば、聞かれない
3. AI Agent が、番号付きの指摘と、選べる直し方を出す
   - この時点では、ファイルを書き換えない
4. 指摘ごとに、番号で直し方を選ぶ
   - 直さないものは「このままでいい」と答える
5. AI Agent が、選んだ直し方で直す

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

| 検査内容                                  | Slot | Free | ADR / PDR |
| ----------------------------------------- | ---- | ---- | --------- |
| YAML front matter およびタグ              | ✅   | ✅   | ✅        |
| ドキュメントのリンク切れ                  | ✅   | ✅   | ✅        |
| 見出しの数と章番号                        | ✅   | ✅   | ✅        |
| Slot ファイルの名前                       | ✅   |      |           |
| 太字、斜体、ドルマークの利用 ( 設定次第 ) | ✅   | ✅   | ✅        |
| format ( 設定次第 )                       | ✅   | ✅   | ✅        |
| textlint ( 設定次第 )                     | ✅   | ✅   | ✅        |

Slot の YAML front matter は、`mission` が `docv list` のミッションと同じかを検査する。ページに分けた Slot は、`order` とページの名前（h1）があるかと、同じ Slot のページどうしの重なりも検査する。中身の形（雛形の見出しや表）は検査しない。

YAML の Slot（`service/words.yaml` など）が残っていれば error にし、`docv gen <slot> --print` の雛形を見て md に書き直すよう案内する。

あわせて、`docv.yaml` の `docv_version` を docv の版と比べる。`docv.yaml` の 1 行目にあるスキーマの URL の版も、`docv_version` と比べる（[設定ファイル](service/config.md) の「docv の版」の章）。

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
- `document_root` の直下に、Slot ファイルへのリンクを並べたインデックス（`README.md`）を毎回書く
  - ルートの README からリンクすると、GitHub でリポジトリを開いた人が、clone しなくても Slot ファイルを読める

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
  - docv の版を上げただけの行（`docv.yaml` の 1 行目のスキーマの URL と `docv_version`）は、差分に数えない。版のほかに変わっていないファイルには、印を付けない
- ページの中には差分を出さない
  - 元のファイルのパスの代わりに `git diff` のコマンドを出すので、詳しい差分はそれを実行して見る
- HTML は OS のキャッシュのディレクトリに書き、`dir.output` には書かない。実行するたびに作り直す
- 差分がなければ、何も作らずに、差分がないと表示して終わる
- git の管理下にないときと、`--from` がリビジョンでないときは、エラーで終わる
- `docv open` と同じく、macOS と Windows だけに対応する

| オプション     | 意味                                                                            |
| -------------- | ------------------------------------------------------------------------------- |
| `--from <rev>` | 比べる git のリビジョン（ブランチ、タグ、コミット）。省略すると `HEAD` と比べる |

### docv gen

Slot ファイルを、決まったパスに作る。

YAML front matter の `mission` にミッションを書く。用語集、技術スタック、環境変数、費用、インストール、環境構築、開発コマンド、Doc ガイドは、見出しと表の形を示した雛形を書く。ほかの Slot は、中身の形が自由なので、`mission` だけを書く。

| オプション | 意味                                    |
| ---------- | --------------------------------------- |
| `--print`  | Slot ファイルを作らずに、書く中身を出す |
| `--force`  | Slot ファイルがすでにあっても書き直す   |

```sh
docv gen manual               # docv/slot/service/manual.md を作る
docv gen doc-guide            # docv/slot/guide/doc-guide.md に、ガイドラインの雛形を作る
docv gen doc-guide --print    # 運用中のファイルと雛形を見比べる
```

Slot をページに分けるときは、`docv/slot/service/service/` などのページのディレクトリの下にページを手で置く（[ページ分け](service/desc.md#ページ分け)）。ページがあるときは、`docv gen service` などは、1 つのファイルを作らずにエラーで終わる。

## スキーマ一覧

- [`docv.yaml`](../../free/schema/config.md)
