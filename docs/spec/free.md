---
mission: Free ファイルの YAML front matter とタグはどんな形で、check は Free ファイルの何を確かめるか
tags: [仕様, ルール]
---

# Free ファイル

docv が管理する Free ファイル（`docv.yaml` の `docs` に当てはまる Markdown）のルール。何をどう書くかはガイドライン（Slot の Doc ガイド、`guide/doc-guide.md`）で、プロジェクトが決める。Slot のルールは [`spec/slot.md`](slot.md)、ADR のルールは [`spec/adr.md`](adr.md)、`docv.yaml` は [`spec/config.md`](config.md) に書く。

---

## 1. YAML front matter

| key       | 必須 | 中身                             |
| --------- | ---- | -------------------------------- |
| `mission` | 必須 | このファイルが答えること（1 文） |
| `tags`    | 任意 | タグの配列（2 章）               |

```yaml
---
mission: 注文の処理は、どの注文をいつ確定し、失敗したときにどう扱うか
tags: [仕様, 注文]
---
```

- `mission` は 1 行で書く
- ファイルごとの設定は、すべて YAML front matter に書き、`docv.yaml` には書かない。ファイルを動かしても設定がずれず、AI Agent はファイルを読むだけで、そのファイルの扱いが分かる
- `docv.yaml` の `overrides` で `mission: optional` にしたパスでは、`mission` を書かなくてよい（[`spec/config.md`](config.md) の 3 章）
- docv が知らない key は、そのまま残し、check もしない。作成者の記録（`created` / `updated` など）のように、プロジェクトが決めた key を書いてよい
- HTML は、`mission` をファイルの説明として表示する

---

## 2. タグ

- Free ファイルの分類は、YAML front matter の `tags` に書くタグで行う。1 つのファイルにいくつでも付けられる
- docv はタグの意味を決めない。どんなタグを使うかは、プロジェクトが決める
- `docv.yaml` に `tags`（タグの一覧）を書いたときだけ、YAML front matter の `tags` をその中に限る。書かなければ、どんなタグでも付けられる

### タグに付けるルール

`docv.yaml` の `lint.tags` に、タグごとにルールを書く。そのタグを持つファイルにだけ、ルールをかける。1 つのファイルが複数のタグを持てば、それぞれのタグのルールをすべてかける。

```yaml
lint:
  tags:
    仕様: [no-past-wording]
    計画: [no-done-task]
```

| ルール          | かけること                                                                  |
| --------------- | --------------------------------------------------------------------------- |
| no-past-wording | 過去や実装状態の言い回しを warn にする（3 章の past-wording、past-heading） |
| no-done-task    | 終わったタスク（`- [x]`）を warn にする                                     |

---

## 3. check のルール

`docv check` が Free ファイルで確かめるルール。`docv.yaml` で使うと決めたときだけ確かめるもの（書式の規則、整形、textlint）は [`spec/config.md`](config.md) に書く。

### YAML front matter

| ID                  | 強さ  | 内容                                                       |
| ------------------- | ----- | ---------------------------------------------------------- |
| frontmatter-invalid | error | YAML front matter が読めない。`mission`、`tags` の型が違う |
| mission-missing     | error | `mission` がない                                           |
| mission-multiline   | error | `mission` が複数行                                         |
| tag-unknown         | error | `docv.yaml` に `tags` を書いたとき、そこにないタグがある   |

- `docv.yaml` の `overrides` で `mission: optional` にしたパスでは、mission-missing を出さない

### 構造とリンク

| ID          | 強さ  | 内容                                           |
| ----------- | ----- | ---------------------------------------------- |
| title-count | error | h1 が 1 つでない                               |
| link-broken | error | ルートの中を指す相対リンクの先にファイルがない |

- リンクのアンカー（`#` 以降）は確かめない

### タグに付けたルール

`lint.tags` でタグに付けたときだけ、そのタグを持つファイルで確かめる。コードブロックとインラインコードの中は見ない。

| ID           | 強さ | タグのルール    | 内容                                   |
| ------------ | ---- | --------------- | -------------------------------------- |
| past-wording | warn | no-past-wording | 本文に、過去や実装状態の言い回しがある |
| past-heading | warn | no-past-wording | 見出しに、過去を扱う語がある           |
| done-task    | warn | no-done-task    | 終わったタスク（`- [x]`）が残っている  |

既定の語は次のとおり。`docv.yaml` の `lint.words.add` は past-wording に語を足し、`lint.words.remove` は両方から語を外す。語は大文字と小文字を区別し、部分一致で探す。

| ルール       | 既定の語                                                                                                                                                                        |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| past-wording | `以前は`、`従来は`、`旧仕様`、`旧方式`、`に変更した`、`に変更しました`、`を廃止した`、`を廃止しました`、`未実装`、`実装予定`、`実装済み`、`今後は`、`将来的に`、`TODO`、`FIXME` |
| past-heading | `変更履歴`、`更新履歴`、`経緯`、`廃案`、`不採用`、`未実装`、`今後の予定`                                                                                                        |
