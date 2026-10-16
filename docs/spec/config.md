---
mission: docv.yaml には何を書き、各項目はどう解釈され、check と fix の道具をどう使うか
tags: [仕様, 設定]
---

# 設定ファイル（`docv.yaml`）

リポジトリのルートに置く設定ファイル。型は [`assets/schema/docv.schema.json`](../../assets/schema/docv.schema.json) が正本。1 章は項目の一覧、2 章以降は一覧に書ききれない解釈の決まりと、`docv.yaml` で使うと決めたときだけ動く道具（書式の規則、textlint、整形）。

---

## 1. 例

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/suzuki-hoge/docv/v1.0.0/assets/schema/docv.schema.json
title: shop-app

docs:
  - docs/**/*.md
ignore:
  - docs/tmp/**
  - CLAUDE.md

tags: [仕様, 手順, 計画, 未決, 注文, 在庫, 決済]

slots:
  dir: docv
output:
  dir: docv/build

lint:
  tags:
    仕様: [no-past-wording]
    手順: [no-past-wording]
    計画: [no-done-task]
  style:
    bold: deny
  textlint:
    enabled: true
    presets: [ja-technical-writing]
format:
  enabled: true
```

| 項目             | 必須 | 既定         | 意味                                                                                  |
| ---------------- | ---- | ------------ | ------------------------------------------------------------------------------------- |
| `title`          |      | ルートの名前 | HTML に出すプロジェクト名                                                             |
| `docs`           | 必須 |              | 管理対象の Markdown の glob の配列                                                    |
| `ignore`         |      |              | 管理対象にも管理外にもしないファイルの glob の配列                                    |
| `tags`           |      |              | タグの一覧。書いたときだけ、YAML front matter の `tags` をこの中に限る                |
| `slots.dir`      |      | docv         | Slot ファイルを置くディレクトリ                                                       |
| `output.dir`     |      | `docv/build` | docv build の出力先                                                                   |
| `output.commit`  |      | true         | 出力を git で管理するか（8 章）                                                       |
| `output.image`   |      | false        | docv build で、Slot のページを JPG にするか。true なら Chrome が要る                  |
| `lint.tags`      |      |              | タグごとに、そのタグを持つファイルにかけるルール（[`spec/free.md`](free.md) の 2 章） |
| `lint.style`     |      |              | 太字、章番号などの書式の規則（5 章）                                                  |
| `lint.words`     |      |              | 過去の言い回しとして警告する語の足し引き（`add`、`remove`）                           |
| `lint.textlint`  |      |              | textlint を使うか、どのプリセットとルールを使うか（6 章）                             |
| `format.enabled` |      | false        | check で、整形が要るファイルを error にするか（7 章）                                 |
| `overrides`      |      |              | パスごとに、道具の設定を上書きする（3 章）                                            |

---

## 2. 書くことと書かないこと

- `docv.yaml` には、ツールの設定（`lint`、整形、出力）と、プロジェクト全体で共有する語彙（管理対象の範囲、タグの一覧、Slot の置き場所）だけを書く
- ファイルごとの設定（問い、タグ）は、そのファイルの YAML front matter に書く（[`spec/free.md`](free.md) の 1 章）

---

## 3. 管理対象と管理外

- `docs` に当てはまる `.md` のファイルが管理対象になる。check、fix、HTML の対象になる
- `docs` と `ignore` のどちらにも当てはまらない `.md` のファイルは管理外になる。中身は見ない
- `ignore` に当てはまるファイルは、check と HTML のどちらにも出さない
- `.gitignore` に書かれたファイルと、`output.dir` の中は、`ignore` に書かなくても常に除く
- パスと glob は、すべてルートからの相対パスで書く。`*` は `/` をまたがず、`**` はまたぐ

### パスごとの上書き（`overrides`）

管理対象のまま、一部のパスだけ道具の設定を変える。docv が一切扱わない `ignore` とは違い、リンク切れの check や HTML には残る。

```yaml
overrides:
  - paths: [docs/tmp/**]
    textlint: false       # textlint をかけない
    style: false          # 書式の規則をかけない
    format: false         # fix で整形しない、format-diff を出さない
    mission: optional     # YAML front matter の mission を求めない
```

- 上書きできるのは、道具の設定（textlint、書式の規則、整形、YAML front matter を求めるか）だけ。ファイルごとの中身の設定は YAML front matter に書く
- 書かなかった項目は、上書きしない。1 つのファイルに複数の `overrides` が当たれば、後に書いたものが優先する

---

## 4. タグとルール

- `tags` を書いたときだけ、YAML front matter の `tags` をこの中に限る。書かなければ、どんなタグでも付けられる
- `lint.tags` のキーには、タグを書く。`tags` を書いたときは、`tags` にあるタグだけを書ける
- `lint.tags` の値には、ルールの配列を書く。使えるルールは [`spec/free.md`](free.md) の 2 章

---

## 5. 書式の規則（`lint.style`）

太字や HTML タグを使ってよいかなど、プロジェクトごとに決める Markdown の書式を、`docv check` で機械的に確かめるための規則。

- 書式はプロジェクトが決める。docv は規則を用意するだけで、既定ではどの規則も使わない
- `docv.yaml` に書いた規則だけを、管理対象のファイルで確かめる。規則に合わない書き方は error にする
- コードブロックとインラインコードの中は見ない
- docv のバイナリだけで動く。Node.js などはいらない
- どの規則も、値に false を書くと使わない。`overrides` で `style` を false にしたパスには、かけない

| キー                | 値                         | ルール ID               | 確かめること                                                                                |
| ------------------- | -------------------------- | ----------------------- | ------------------------------------------------------------------------------------------- |
| `bold`              | deny                       | style-bold              | 太字（`**`、`__`）を使っていない                                                            |
| `italic`            | deny                       | style-italic            | 斜体（`*`、`_`）を使っていない                                                              |
| `html`              | 使ってよいタグの配列       | style-html              | 配列にない HTML タグを使っていない。`[]` ならすべての HTML タグを禁止する                   |
| `dollar`            | deny                       | style-dollar            | コードの外の 1 行に `$` が 2 つ以上ない。GitHub は `$` で挟んだ部分を数式として表示するため |
| `heading_number`    | require                    | style-heading-number    | h2 の見出しが `1. ` のような章番号で始まり、1 から順に並んでいる                            |
| `section_separator` | require                    | style-section-separator | 2 つ目以降の h2 の直前に `---` がある                                                       |
| `link_text`         | `{ base: <ディレクトリ> }` | style-link-text         | 管理対象のファイルへのリンクの表示文字が、そのファイルのパスになっている                    |
| `image_alt`         | require                    | style-image-alt         | 画像に代替テキストがある                                                                    |

```yaml
lint:
  style:
    bold: deny
    html: [details, summary]
    dollar: deny
    heading_number: require
    section_separator: require
    link_text: { base: docs }
```

<details>
<summary>link_text のパス</summary>

- リンク元が base の中にあるときは、表示文字を base からのパスにする
- リンク元が base の外にあるとき（`README.md` など）は、表示文字をルートからのパスにする
- 表示文字は、パスと完全に一致していなければならない。ファイル名だけでは、どのディレクトリのファイルか分からないため

| base   | リンク元              | リンク先                | 表示文字            |
| ------ | --------------------- | ----------------------- | ------------------- |
| `docs` | `docs/spec/config.md` | `docs/spec/slot.md`     | `spec/slot.md`      |
| `docs` | `docs/spec/config.md` | `docs/schema/config.md` | `schema/config.md`  |
| `docs` | `README.md`           | `docs/spec/slot.md`     | `docs/spec/slot.md` |

</details>

---

## 6. textlint（`lint.textlint`）

文章の校正に textlint を使う仕組み。

- 既定では使わない。`enabled` を true にしたときだけ、`docv check` が textlint をかけ、`docv fix` が自動修正できるところを直す。かけるのは、管理対象のファイルと、`slots.dir` の中の Markdown（md の Slot と ADR）
- textlint は Node.js で動くので、使うときだけ Node.js（npx）が必要になる。docv のバイナリには textlint を入れない。`enabled` が true なのに Node.js が見つからないときは、終了コード 2 で終わる
- 結果は、ほかのルールと同じ形で出す。ルール ID は `textlint/<textlint のルール ID>`（例: textlint/ja-technical-writing/sentence-length）。強さは `level` で決め、既定は warn
- `overrides` で textlint を false にしたパスには、かけない。`--no-textlint` で、textlint だけを飛ばせる

| 使い方   | 書く項目                  | ルールのパッケージ                                                                    | 向いている場合                |
| -------- | ------------------------- | ------------------------------------------------------------------------------------- | ----------------------------- |
| 組み込み | `presets`、`rules`、`prh` | docv の版ごとに決めた版を、npx で取ってくる。プロジェクトに `package.json` はいらない | docv が用意したルールで足りる |
| 持ち込み | `config`                  | プロジェクトの node_modules に入っているものを使う                                    | 自分で選んだルールを使いたい  |

`presets`・`rules`・`prh` と `config` は、同時には書けない。

### 組み込み

| `presets` に書く名前 | パッケージ                                | 中身                                                                   |
| -------------------- | ----------------------------------------- | ---------------------------------------------------------------------- |
| ja-technical-writing | textlint-rule-preset-ja-technical-writing | 技術文書向けの規則（一文の長さ、読点の数、助詞の重複、文末の句点など） |
| ja-spacing           | textlint-rule-preset-ja-spacing           | 全角と半角の間のスペースなど                                           |
| jtf-style            | textlint-rule-preset-jtf-style            | JTF 日本語標準スタイルガイド                                           |

- `rules`: `<プリセット>/<ルール>` をキーにして、false（使わない）か、そのルールのオプションを書く
- `prh`: 表記ゆれの辞書（`prh` の YAML）のパスの配列。textlint-rule-prh で確かめる。プロジェクトの用語をそろえるために使う

```yaml
lint:
  textlint:
    enabled: true
    presets: [ja-technical-writing, ja-spacing]
    rules:
      ja-technical-writing/sentence-length: { max: 120 }
      ja-technical-writing/no-exclamation-question-mark: false
      ja-spacing/ja-space-between-half-and-full-width: { space: always }
    prh: [docs/rule/prh.yml]
```

- docv は、textlint の設定ファイルを OS のキャッシュのディレクトリに作り、npx で textlint とプリセットを指定して実行する。リポジトリにはファイルを作らない
- パッケージの版は docv の版ごとに固定する。同じ docv なら同じ結果になる
- 初めて実行するときは npx がパッケージを取ってくるので、ネットワークが必要

### 持ち込み

- `config` に、textlint の設定ファイルのパス（ルートからの相対パス）を書く
- docv はルートで `npx textlint --config <config> --format json <ファイル...>` を実行する。`docv fix` は `--format json` の代わりに `--fix` を付ける
- ルールのパッケージは、利用者が `package.json` で入れておく

---

## 7. 整形との連動（`format.enabled`）

整形の形は docv が決め、設定では変えない。設定で決めるのは、check を整形と連動させるかだけ。整形は、管理対象のファイルと、`slots.dir` の中の Markdown（md の Slot と ADR）にかける。

- 表は、列ごとに縦の線の位置を揃える。幅は、全角の文字（Unicode の East Asian Width が W か F の文字）を 2、それ以外の文字を 1 と数える
- エディタの表の揃え方は、この数え方と違うことがある。手で揃えず、`docv fix` の結果を正とする
- 整形の形は docv の版ごとに固定する。同じ docv なら、同じ入力から同じ結果になる

| コマンド     | `format.enabled` が true のときの動き                             |
| ------------ | ----------------------------------------------------------------- |
| `docv check` | 整形が必要なファイルを、ルール ID format-diff の error として出す |

- format-diff には、整形した結果と最初に違う行を出す
- false でも、`docv fix` を直接実行すれば整形できる
- `overrides` で `format` を false にしたパスでは、fix で整形せず、format-diff も出さない

---

## 8. 出力を git で管理するか（`output.commit`）

| `output.commit` | 動き                                                  | 使い方                                                 |
| --------------- | ----------------------------------------------------- | ------------------------------------------------------ |
| true            | 何もしない。出力をコミットするのは利用者              | README からリンクする、GitHub Pages でホスティングする |
| false           | `output.dir` に、`*` だけを書いた `.gitignore` を置く | 手元で見るだけにする                                   |

リポジトリのルートの `.gitignore` には触らない。

---

## 9. 推奨する配置

新しく始めるプロジェクトでは、次の配置をすすめる。`docv init` は、この配置を前提にした `docv.yaml` を作る。

```
README.md        # 管理外。Slot の include で章を埋め込める
docs/            # Free ファイル
  *.md           # 分類はディレクトリではなくタグで行う
  tmp/           # メモを置く。overrides で textlint などを外すか、ignore に入れる
docv/            # Slot ファイル
  service/       # サービス仕様。ほかに system/、guide/、operation/
    service.md
  adr/           # ADR
  build/         # docv build の出力
```

- `docs/` と `docv/` を分けるのは、プロジェクトが問いと形を決める Free ファイルと、docv が決める Slot ファイルを、置き場所で見分けられるようにするため

配置はプロジェクトが決める。既存のディレクトリに合わせて `docs` を書けばよい。

---

## 10. スキーマの参照

- `docv.yaml` の先頭に、yaml-language-server の書式でスキーマの URL を書くと、エディタで補完と検証が効く
- URL には docv の版を含める。`docv init` は、実行した CLI の版の URL を書く
- `docv init --print` で、すべての項目を既定の値でコメントにした `docv.yaml` を出す。スキーマの写しは `docs/schema/config.md` に置く
