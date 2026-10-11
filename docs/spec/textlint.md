---
question: docv check は textlint をどう使い、利用者はルールをどこまで決められるか
topics: [ルール, CLI]
---

# textlint

文章の校正に textlint を使う仕組み。使うかどうかと、使うルールは docv.yaml の lint.textlint で決める（[spec/config.md](config.md)）。Markdown の書式の規則は [spec/style.md](style.md)。

---

## 1. 考え方

- 既定では使わない。lint.textlint.enabled を true にしたときだけ、`docv check` が管理対象のファイルに textlint をかける
- textlint は Node.js で動くので、使うときだけ Node.js（npx）が必要になる。docv のバイナリには textlint を入れない
- 結果は、docv check のほかの結果と同じ形で出す

---

## 2. 2 つの使い方

| 使い方 | 書く項目 | ルールのパッケージ | 向いている場合 |
|---|---|---|---|
| 組み込み | presets、rules、prh | docv の版ごとに決めた版を、npx で取ってくる。プロジェクトに package.json はいらない | docv が用意したルールで足りる |
| 持ち込み | config | プロジェクトの node_modules に入っているものを使う | 自分で選んだルールを使いたい |

presets・rules・prh と config は、同時には書けない。

---

## 3. 組み込みの使い方

### presets

| presets に書く名前 | パッケージ | 中身 |
|---|---|---|
| ja-technical-writing | textlint-rule-preset-ja-technical-writing | 技術文書向けの規則（一文の長さ、読点の数、助詞の重複、文末の句点など） |
| ja-spacing | textlint-rule-preset-ja-spacing | 全角と半角の間のスペースなど |
| jtf-style | textlint-rule-preset-jtf-style | JTF 日本語標準スタイルガイド |

### rules

`<プリセット>/<ルール>` をキーにして、false（使わない）か、そのルールのオプションを書く。

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

### prh

- 表記ゆれの辞書（prh の YAML）のパスの配列。textlint-rule-prh で確かめる
- プロジェクトの用語をそろえるために使う

### 実行のしかた

- docv は、textlint の設定ファイルを OS のキャッシュのディレクトリに作り、npx で textlint とプリセットを指定して実行する。リポジトリにはファイルを作らない
- パッケージの版は docv の版ごとに固定する。同じ docv なら同じ結果になる
- 初めて実行するときは npx がパッケージを取ってくるので、ネットワークが必要

---

## 4. 持ち込みの使い方

- config に、textlint の設定ファイルのパス（ルートからの相対パス）を書く
- docv はルートで `npx textlint --config <config> --format json <ファイル...>` を実行する
- ルールのパッケージは、利用者が package.json で入れておく

---

## 5. 結果

- ルール ID は `textlint/<textlint のルール ID>` にする（例: textlint/ja-technical-writing/sentence-length）
- 強さは lint.textlint.level で決める。既定は warn。docs の項目の check: warn / off も効く
- archive には textlint をかけない。archive は移した本文をそのまま残すため
- `docv check --no-textlint` で、textlint だけを飛ばせる。hook で速く回したいときに使う
- enabled が true なのに Node.js が見つからないときは、終了コード 2 で終わる
