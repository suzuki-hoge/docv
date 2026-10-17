---
mission: Free ファイルの YAML front matter とタグの形と、check が Free ファイルで確かめること
tags: [仕様, ルール]
---

# Free ファイル

docv が管理する Free ファイル（`docv.yaml` の `dir.free` のディレクトリの下の Markdown）のルール。何をどう書くかはガイドライン（Slot の Doc ガイド、`guide/doc-guide.md`）で、プロジェクトが決める。Slot のルールは [`spec/slot.md`](slot.md)、ADR のルールは [`spec/adr.md`](adr.md)、`docv.yaml` は [`spec/config.md`](config.md) に書く。

---

## 1. YAML front matter

| key       | 必須 | 中身                                                 |
| --------- | ---- | ---------------------------------------------------- |
| `mission` | 必須 | ミッション。このファイルを読むと何が分かるか（1 文） |
| `tags`    | 任意 | タグの配列（2 章）                                   |

```yaml
---
mission: 注文を確定するタイミングと、失敗したときの扱い
tags: [仕様, 注文]
---
```

- `mission` は 1 行で書く
- ファイルごとの設定は、すべて YAML front matter に書き、`docv.yaml` には書かない。ファイルを動かしても設定がずれず、AI Agent はファイルを読むだけで、そのファイルの扱いが分かる
- docv が知らない key は、そのまま残し、check もしない。作成者の記録（`created` / `updated` など）のように、プロジェクトが決めた key を書いてよい
- HTML は、`mission` をファイルの説明として表示する

---

## 2. タグ

- Free ファイルの分類は、YAML front matter の `tags` に書くタグで行う。1 つのファイルにいくつでも付けられる
- docv はタグの意味を決めない。どんなタグを使うかは、プロジェクトが決める
- 使うタグは、すべて `docv.yaml` の `tags` に書く。`tags` にないタグを付けると、check が error にする
- `tags` には、タグごとに名前（`name`）と色（`color`）を書く。色は HTML でタグを見分けるためのもので、blue、green、yellow、red、purple、orange、teal、pink から選ぶ

```yaml
tags:
  - name: 仕様
    color: blue
  - name: 計画
    color: green
```

---

## 3. check のルール

`docv check` が Free ファイルで確かめるルール。`docv.yaml` で使うと決めたときだけ確かめるもの（太字などの書式の規則、整形、textlint）は [`spec/config.md`](config.md) に書く。

### YAML front matter

| ID                  | 内容                                                       |
| ------------------- | ---------------------------------------------------------- |
| frontmatter-invalid | YAML front matter が読めない。`mission`、`tags` の型が違う |
| mission-missing     | `mission` がない                                           |
| mission-multiline   | `mission` が複数行                                         |
| tag-unknown         | `docv.yaml` の `tags` にないタグがある                     |

### 構造とリンク

| ID             | 内容                                                                     |
| -------------- | ------------------------------------------------------------------------ |
| title-count    | h1 が 1 つでない                                                         |
| heading-number | 章番号で始まる h2 があるのに、h2 が `1. ` から順に章番号で始まっていない |
| link-broken    | ルートの中を指す相対リンクの先にファイルがない                           |
| link-text      | 管理対象のファイルへのリンクの表示文字が、そのファイルのパスでない       |

- 章番号は、h2 のどれかが `1. ` のような章番号で始まっているファイルでだけ確かめる。章番号を使わないファイルでは確かめない
- リンクのアンカー（`#` 以降）は確かめない

<details>
<summary>link-text のパス</summary>

- 表示文字は、`docv.yaml` の `dir.free` のディレクトリからの、リンク先のパスにする
- 表示文字は、パスと完全に一致していなければならない。ファイル名だけでは、どのディレクトリのファイルか分からないため

| `dir.free` | リンク元              | リンク先                | 表示文字           |
| ---------- | --------------------- | ----------------------- | ------------------ |
| `docs`     | `docs/spec/config.md` | `docs/spec/slot.md`     | `spec/slot.md`     |
| `docs`     | `docs/spec/config.md` | `docs/schema/config.md` | `schema/config.md` |

</details>
