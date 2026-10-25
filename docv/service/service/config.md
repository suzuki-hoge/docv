---
order: 5
mission: 外から見た機能と、それぞれの振る舞いと決まり
---

# 設定ファイル

リポジトリのルートに置く設定ファイル（`docv.yaml`）。型は [`assets/schema/docv.schema.json`](../../../assets/schema/docv.schema.json) が正本。最初の章で項目を一覧にし、続く章で、一覧に書ききれない解釈の決まりと、`docv.yaml` で使うと決めたときだけ動く道具（書式の規則、textlint、整形）を書く。

---

## 1. 例

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/suzuki-hoge/docv/v1.2.1/assets/schema/docv.schema.json
title: shop-app

dir:
  free: docs
  image: output/docv/image
ignore:
  - docs/tmp/**

tags:
  - name: 仕様
    color: blue
  - name: 手順
    color: green
  - name: 規約
    color: yellow

lint:
  bold: deny
  textlint: enable
format:
  enabled: true
```

| 項目             | 必須 | 既定               | 意味                                                                                                            |
| ---------------- | ---- | ------------------ | --------------------------------------------------------------------------------------------------------------- |
| `title`          |      | ルートの名前       | HTML に出すプロジェクト名                                                                                       |
| `dir.free`       |      | `docs`             | 管理対象の Markdown（Free ファイル）を置くディレクトリ（「管理対象と管理外」の章）                              |
| `dir.slot`       |      | `docv`             | Slot ファイルを置くディレクトリ                                                                                 |
| `dir.html`       |      | `output/docv/html` | docv build が HTML を出すディレクトリ                                                                           |
| `dir.image`      |      |                    | docv build が Slot のページの JPG を出すディレクトリ。書かなければ JPG を作らない。JPG を作るには Chrome が要る |
| `ignore`         |      |                    | 管理対象にも管理外にもしないファイルの glob の配列                                                              |
| `tags`           |      |                    | 使うタグの一覧。タグごとに `name` と、HTML の色の `color` を書く（[Free](free.md) のページの「タグ」の章）      |
| `lint.bold`      |      | allow              | 太字を使ってよいか（「書式の規則」の章）                                                                        |
| `lint.italic`    |      | allow              | 斜体を使ってよいか（「書式の規則」の章）                                                                        |
| `lint.dollar`    |      | allow              | コードの外の 1 行に `$` を 2 つ以上書いてよいか（「書式の規則」の章）                                           |
| `lint.textlint`  |      | disable            | textlint で文章を校正するか（「textlint」の章）                                                                 |
| `format.enabled` |      | false              | check で、整形が要るファイルを error にするか（「整形との連動」の章）                                           |

---

## 2. 書くことと書かないこと

- `docv.yaml` には、ツールの設定（`lint`、整形、出力）と、プロジェクト全体で共有する語彙（管理対象の範囲、タグの一覧、Slot の置き場所）だけを書く
- ファイルごとの設定（ミッション、タグ）は、そのファイルの YAML front matter に書く（[Free](free.md) のページの「YAML front matter」の章）

---

## 3. 管理対象と管理外

- `dir.free` に書いたディレクトリの下の `.md` のファイルが、管理対象になる。check、fix、HTML の対象になる。`dir.free` は 1 つだけ書け、ルートは書けない
- `dir.free`、`dir.slot`、`dir.html`、`dir.image`（書いたとき）は、どの 2 つも同じディレクトリにできず、一方が他方を含むこともできない。ルートも書けない。違反すると、`docv.yaml` が正しくないとして、check、fix、build などの `docv.yaml` を読むコマンドが、何もせずに終了コード 2 で終わる
- `dir.free` の外にあり、`ignore` にも当てはまらない `.md` のファイルは管理外になる。中身は見ない
- `ignore` に当てはまるファイルは、check と HTML のどちらにも出さない
- `.gitignore` に書かれたファイルと、`dir.html` と `dir.image` の中は、`ignore` に書かなくても常に除く
- docv build の出力を git で管理するかは、プロジェクトの `.gitignore` で決める。docv は `.gitignore` を書かない
- パスと glob は、すべてルートからの相対パスで書く。`*` は `/` をまたがず、`**` はまたぐ

---

## 4. 書式の規則（`lint.bold`、`lint.italic`、`lint.dollar`）

太字などを使ってよいかを、プロジェクトごとに決める。`deny` にした書き方を、管理対象のファイルで error にする。

| キー     | ルール ID    | `deny` のとき確かめること                                                                   |
| -------- | ------------ | ------------------------------------------------------------------------------------------- |
| `bold`   | style-bold   | 太字（`**`、`__`）を使っていない                                                            |
| `italic` | style-italic | 斜体（`*`、`_`）を使っていない                                                              |
| `dollar` | style-dollar | コードの外の 1 行に `$` が 2 つ以上ない。GitHub は `$` で挟んだ部分を数式として表示するため |

- 既定はどれも `allow`（確かめない）
- コードブロックとインラインコードの中は見ない
- 章番号は、設定によらず常に確かめる（[Free](free.md) のページの「check のルール」の章）
- 管理対象のファイルのほか、md の Slot、ADR、YAML の Slot の Markdown の項目にもかける

---

## 5. textlint（`lint.textlint`）

文章の校正に textlint を使う仕組み。AI Agent が書く文章のぶれをなくすのが目的なので、使うルールとその値は docv が決める。利用者は、使うかどうか（`enable`、`disable`）だけを決める。

- 既定では使わない。`enable` にしたときだけ、`docv check` が textlint をかけ、`docv fix` が自動修正できるところを直す。かけるのは、管理対象のファイル、`dir.slot` の中の Markdown（md の Slot と ADR）、YAML の Slot の Markdown の項目
- YAML の Slot の Markdown の項目には、`ja-no-mixed-period`（文末を「。」で終える）をかけない。項目の多くは HTML で表のセルに出す短い説明で、textlint が段落として扱わない Free ファイルの表のセルと、扱いを揃えるため
- textlint は Node.js で動くので、使うときだけ Node.js（npx）が必要になる。`enable` なのに Node.js が見つからないときは、終了コード 2 で終わる
- 結果は、ほかのルールと同じ形の error で出す。ルール ID は `textlint/<プリセット>/<ルール>`（例: textlint/ja-technical-writing/sentence-length）。下の表では、プリセットごとにルールの名前だけを書く
- `--no-textlint` で、textlint だけを飛ばせる
- プロジェクトに `package.json` はいらない。初めて実行するときは、textlint を取ってくるのにネットワークが必要
- 使うルールとその値は、docv の版ごとに固定する。同じ docv なら同じ結果になる
- docv が textlint をどう動かすかは、[システム仕様](../../system/system.md) の「textlint とのやりとり」の章に書く

### 検査項目

#### ja-technical-writing

| ルール                               | 確かめること                             |
| ------------------------------------ | ---------------------------------------- |
| `sentence-length`                    | 一文を 100 文字以内にする                |
| `max-comma`                          | 一文のカンマ（,）を 3 つまでにする       |
| `max-ten`                            | 一文の読点（、）を 3 つまでにする        |
| `no-mix-dearu-desumasu`              | 「である」と「ですます」を混ぜない       |
| `ja-no-mixed-period`                 | 文末を「。」で終える                     |
| `arabic-kanji-numbers`               | 算用数字と漢数字を使い分ける             |
| `no-doubled-conjunction`             | 同じ接続詞を続けない                     |
| `no-doubled-conjunctive-particle-ga` | 逆接の「が」を一文に 2 回使わない        |
| `no-double-negative-ja`              | 二重否定を使わない                       |
| `no-doubled-joshi`                   | 同じ助詞を続けない                       |
| `no-dropping-the-ra`                 | ら抜き言葉を使わない                     |
| `no-nfd`                             | UTF8-MAC の濁点を使わない                |
| `no-zero-width-spaces`               | ゼロ幅スペースを使わない                 |
| `ja-no-abusage`                      | よくある誤用を避ける                     |
| `ja-no-redundant-expression`         | 冗長な表現を避ける                       |
| `ja-unnatural-alphabet`              | 入力ミスで紛れたアルファベットを残さない |
| `no-unmatched-pair`                  | 括弧を対にする                           |

#### ja-spacing

| ルール                                 | 確かめること                                      |
| -------------------------------------- | ------------------------------------------------- |
| `ja-space-between-half-and-full-width` | 半角の文字と全角の文字の間にスペースを入れる      |
| `ja-space-around-code`                 | インラインコードの前後にスペースを入れる          |
| `ja-space-around-link`                 | リンクの前後にスペースを入れる                    |
| `ja-space-around-emphasis`             | 斜体の前後にスペースを入れる                      |
| `ja-space-around-strong`               | 太字の前後にスペースを入れる                      |
| `ja-no-space-around-parentheses`       | 全角の括弧と [ ] の内側と外側にスペースを入れない |

### 非検査項目

docv は、次のルールを使わない。

#### ja-technical-writing

| ルール                         | 確かめないこと                 |
| ------------------------------ | ------------------------------ |
| `max-kanji-continuous-len`     | 漢字が続く長さ                 |
| `no-invalid-control-character` | 制御文字                       |
| `no-exclamation-question-mark` | 感嘆符と疑問符を使うこと       |
| `no-hankaku-kana`              | 半角カナ                       |
| `ja-no-weak-phrase`            | 「かもしれない」などの弱い表現 |
| `ja-no-successive-word`        | 同じ語の連続                   |

#### ja-spacing

| ルール                                            | 確かめないこと                 |
| ------------------------------------------------- | ------------------------------ |
| `ja-no-space-around-slash`                        | スラッシュの前後のスペース     |
| `ja-no-space-between-full-width`                  | 全角の文字どうしの間のスペース |
| `ja-nakaguro-or-halfwidth-space-between-katakana` | カタカナ語の区切りの文字       |
| `ja-space-after-exclamation`                      | 文末の感嘆符の後のスペース     |
| `ja-space-after-question`                         | 文末の疑問符の後のスペース     |

---

## 6. 整形との連動（`format.enabled`）

整形の形は docv が決め、設定では変えない。設定で決めるのは、check を整形と連動させるかだけ。整形は、管理対象のファイル、`dir.slot` の中の Markdown（md の Slot と ADR）、YAML の Slot の Markdown の項目にかける。YAML の Slot は、Markdown の項目の中身を整形し、字下げを付け直して書き戻す。`docv.yaml` と YAML の Slot の末尾の改行は、1 つに整える。

- 表は、列ごとに縦の線の位置を揃える。幅は、全角の文字（Unicode の East Asian Width が W か F の文字）を 2、それ以外の文字を 1 と数える
- エディタの表の揃え方は、この数え方と違うことがある。手で揃えず、`docv fix` の結果を正とする
- 整形の形は docv の版ごとに固定する。同じ docv なら、同じ入力から同じ結果になる

| コマンド     | `format.enabled` が true のときの動き                             |
| ------------ | ----------------------------------------------------------------- |
| `docv check` | 整形が必要なファイルを、ルール ID format-diff の error として出す |

- format-diff には、整形した結果と最初に違う行を出す
- false でも、`docv fix` を直接実行すれば整形できる

---

## 7. 推奨する配置

新しく始めるプロジェクトでは、次の配置をすすめる。`docv init` は、この配置を前提にした `docv.yaml` を作る。

```
README.md        # 管理外
docs/            # Free ファイル
  *.md           # 分類はディレクトリではなくタグで行う
  tmp/           # メモを置く。ignore に入れる
docv/            # Slot ファイル
  service/       # サービス。ほかに system/、guide/、operation/
    service.md
  adr/           # ADR
output/docv/     # docv build の出力
  html/
```

- `docs/` と `docv/` を分けるのは、プロジェクトがミッションと形を決める Free ファイルと、docv が決める Slot ファイルを、置き場所で見分けられるようにするため

配置はプロジェクトが決める。既存のディレクトリに合わせて `dir` を書けばよい。

---

## 8. スキーマの参照

- `docv.yaml` の先頭に、yaml-language-server の書式でスキーマの URL を書くと、エディタで補完と検証が効く
- URL には docv の版を含める。`docv init` は、実行した CLI の版の URL を書く
- `docv init --print` で、すべての項目を既定の値でコメントにした `docv.yaml` を出す。スキーマの写しは `docs/schema/config.md` に置く
