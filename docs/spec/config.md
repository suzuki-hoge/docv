---
question: docv.yaml には何を書き、各項目は何を意味するか
topics: [設定]
---

# 設定ファイル（docv.yaml）

リポジトリのルートに置く設定ファイル。型は [schema/docv.schema.json](../../schema/docv.schema.json) が正本で、ここではそれぞれの項目の意味を書く。

---

## 1. 例

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/suzuki-hoge/docv/v1.0.0/schema/docv.schema.json
version: 1
title: shop-app
repository:
  url: https://github.com/<owner>/shop-app
  branch: main

topics: [注文, 在庫, 決済, 認証]

docs:
  - path: README.md
    kind: overview
    question: shop-app は何で、どこから見ればいいか
  - path: docs/design/*.md
    kind: spec
  - path: docs/manual/*.md
    kind: operation
  - path: docs/pricing/*.md
    kind: cost
  - path: docs/plan/*.md
    kind: plan
    check: warn
  - path: docs/archive/**/*.md
    kind: archive
  - path: docs/tmp/**/*.md
    kind: tmp

ignore:
  - CLAUDE.md

archive:
  dir: docs/archive

output:
  dir: docv
  commit: true
  jpg: false

lint:
  style:
    bold: deny
    dollar: deny
  textlint:
    enabled: true
    presets: [ja-technical-writing]

format:
  enabled: true
```

---

## 2. 項目

| 項目                  | 必須 | 既定値                 | 意味                                                                                                         |
| --------------------- | ---- | ---------------------- | ------------------------------------------------------------------------------------------------------------ |
| version               | 必須 | —                      | 設定ファイルの形式の版。今は 1                                                                               |
| title                 | 任意 | ルートのディレクトリ名 | HTML とインデックスに出すプロジェクト名                                                                      |
| repository.url        | 任意 | —                      | GitHub のリポジトリの URL。HTML で、管理対象でないファイルへのリンクを GitHub の URL にするために使う        |
| repository.branch     | 任意 | main                   | 上のリンクで使うブランチ                                                                                     |
| topics                | 任意 | なし                   | 話題の一覧。frontmatter の topics にはここにある話題だけを書ける                                             |
| docs                  | 必須 | —                      | どのファイルをどの種類として管理するか（3 章）                                                               |
| ignore                | 任意 | なし                   | 管理対象にも管理外にもしないファイルの glob                                                                  |
| kinds                 | 任意 | なし                   | 独自の種類（4 章）                                                                                           |
| templates             | 任意 | なし                   | 種類ごとのテンプレの差し替え。種類の ID をキーに、テンプレのファイルのパスを書く                             |
| archive.dir           | 任意 | docs/archive           | `docv retire` が archive を書くディレクトリ                                                                  |
| output.dir            | 任意 | docv                   | `docv build` の出力先                                                                                        |
| output.commit         | 任意 | true                   | 出力を git で管理するか（[spec/build.md](build.md)）                                                         |
| output.jpg            | 任意 | false                  | HTML に加えて JPG を作るか（[spec/build.md](build.md)）                                                      |
| lint.words.add        | 任意 | なし                   | 過去の言い回しとして警告する語を足す（[spec/check.md](check.md)）                                            |
| lint.words.remove     | 任意 | なし                   | 既定の語から外す                                                                                             |
| lint.style            | 任意 | なし                   | 使う書式の規則と、その値（[spec/style.md](style.md)）                                                        |
| lint.textlint.enabled | 任意 | false                  | textlint を使うか（[spec/textlint.md](textlint.md)）                                                         |
| lint.textlint.level   | 任意 | warn                   | textlint の結果の強さ。warn / error                                                                          |
| lint.textlint.presets | 任意 | なし                   | 組み込みの使い方で使うプリセット                                                                             |
| lint.textlint.rules   | 任意 | なし                   | 組み込みの使い方で、ルールごとに使わない・オプションを変える。presets に書いたプリセットのルールだけを書ける |
| lint.textlint.prh     | 任意 | なし                   | 表記ゆれの辞書のパス                                                                                         |
| lint.textlint.config  | 任意 | なし                   | 持ち込みの使い方で使う、textlint の設定ファイルのパス。presets・rules・prh とは同時に書けない                |
| format.enabled        | 任意 | false                  | check、new、retire を整形と連動させるか（[spec/fix.md](fix.md)）                                             |

- パスと glob はすべてルートからの相対パスで書く
- ignore に当てはまるファイルは、check、index、HTML、migrate-guide のどれにも出さない
- .gitignore に書かれたファイルと、output.dir の中は、ignore に書かなくても常に除く

---

## 3. docs

| 項目     | 必須 | 既定値 | 意味                                                                                                                          |
| -------- | ---- | ------ | ----------------------------------------------------------------------------------------------------------------------------- |
| path     | 必須 | —      | 対象のファイルの glob。.md のファイルだけが対象になる                                                                         |
| kind     | 必須 | —      | 種類の ID。組み込みの種類か、kinds で足した種類                                                                               |
| check    | 任意 | error  | error / warn / off。warn にすると、このファイルの error を warn に下げる。off にすると check しない（index と HTML には出る） |
| question | 任意 | —      | frontmatter の question の代わり。path が 1 ファイルだけを指すときに書ける                                                    |
| topics   | 任意 | —      | frontmatter の topics の代わり。question と同じ条件                                                                           |
| style    | 任意 | —      | この項目のファイルにだけ効く lint.style の上書き。false を書くと、その規則を使わない（[spec/style.md](style.md) の 4 章）     |

- 上から順に見て、最初に当てはまった項目を使う
- どの項目にも当てはまらない .md のファイルは管理外になる

---

## 4. kinds

```yaml
kinds:
  - id: research
    label: 調査
    time: future
```

| 項目  | 必須 | 意味                                                                    |
| ----- | ---- | ----------------------------------------------------------------------- |
| id    | 必須 | 種類の ID。小文字の英数字とハイフン。組み込みの種類と同じ ID は使えない |
| label | 任意 | HTML に出す表示名。省略すると id                                        |
| time  | 必須 | future / present / past                                                 |

---

## 5. 推奨する配置

新しく始めるプロジェクトでは、次の配置をすすめる。`docv init` は、この配置を前提にした docv.yaml を作る。

```
README.md              # overview
docs/
  spec/
  operation/
  cost/
  rule/
  plan/
  proposal/
  fixme/
  archive/
  tmp/
```

配置は自由に決めてよい。既存のディレクトリに合わせて docs を書けばよい。

---

## 6. スキーマの参照

- docv.yaml の先頭に、yaml-language-server の書式でスキーマの URL を書くと、エディタで補完と検証が効く
- URL には docv の版を含める。`docv init` は、実行した CLI の版の URL を書く
