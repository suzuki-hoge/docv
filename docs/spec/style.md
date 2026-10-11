---
question: docv check は、Markdown の書き方のどんな規則を静的に確かめられるか
topics: [ルール, CLI]
---

# 書式の規則

太字や HTML タグを使ってよいかなど、プロジェクトごとに決める Markdown の書式を、`docv check` で機械的に確かめるための規則。どの規則を使うかは docv.yaml の lint.style で選ぶ（[spec/config.md](config.md)）。文章の校正は [spec/textlint.md](textlint.md)、空行や表の余白の整形は [spec/fmt.md](fmt.md)。

---

## 1. 考え方

- 書式はプロジェクトが決める。docv は規則を用意するだけで、既定ではどの規則も使わない
- docv.yaml に書いた規則だけを、管理対象のファイルで確かめる
- 規則に合わない書き方は error にする。docs の項目で check: warn にすると warn に下がる
- コードブロックとインラインコードの中は見ない
- docv のバイナリだけで動く。Node.js などはいらない

---

## 2. 規則

| docv.yaml のキー | 値 | ルール ID | 確かめること |
|---|---|---|---|
| bold | deny | style-bold | 太字（`**`、`__`）を使っていない |
| italic | deny | style-italic | 斜体（`*`、`_`）を使っていない |
| html | 使ってよいタグの配列 | style-html | 配列にない HTML タグを使っていない。`[]` ならすべての HTML タグを禁止する |
| dollar | deny | style-dollar | コードの外の 1 行に `$` が 2 つ以上ない。GitHub は `$` で挟んだ部分を数式として表示するため |
| heading_number | require | style-heading-number | h2 の見出しが `1. ` のような章番号で始まり、1 から順に並んでいる |
| section_separator | require | style-section-separator | 2 つ目以降の h2 の直前に `---` がある |
| link_text | `{ base: <ディレクトリ> }` | style-link-text | 管理対象のファイルへのリンクの表示文字が、そのファイルのパスになっている（3 章） |
| image_alt | require | style-image-alt | 画像に代替テキストがある |

- どの規則も、値に false を書くと使わない。docs の項目で一部のファイルだけ外すときに使う（4 章）
- archive には heading_number と section_separator を適用しない。archive の章の形は `docv retire` が決める（[spec/retire.md](retire.md)）

---

## 3. link_text のパス

- リンク元が base の中にあるときは、表示文字を base からのパスにする
- リンク元が base の外にあるとき（README.md など）は、表示文字をルートからのパスにする
- 表示文字は、パスと完全に一致していなければならない

| base | リンク元 | リンク先 | 表示文字 |
|---|---|---|---|
| docs | docs/spec/cli.md | docs/spec/check.md | spec/check.md |
| docs | docs/plan/001-first-release.md | docs/spec/build.md | spec/build.md |
| docs | README.md | docs/spec/cli.md | docs/spec/cli.md |

ファイル名だけでは、どのディレクトリのファイルか分からないので、この規則がある。

---

## 4. 設定の例

```yaml
lint:
  style:
    bold: deny
    html: [details, summary]
    dollar: deny
    heading_number: require
    section_separator: require
    link_text: { base: docs }

docs:
  - path: README.md
    kind: overview
    style:                  # README.md だけ章番号と区切り線を求めない
      heading_number: false
      section_separator: false
```

docs の項目の style は、lint.style に上書きする形で効く。
