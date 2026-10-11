---
question: docv check は何を確かめ、どう報告するか
topics: [CLI]
---

# check

`docv check` が確かめるルールと、報告の形。ルールの元になっている決まりは [spec/kind.md](kind.md)、[spec/frontmatter.md](frontmatter.md)、[spec/writing.md](writing.md)。docv.yaml で使うと決めたときだけ確かめるものは、[spec/style.md](style.md)、[spec/textlint.md](textlint.md)、[spec/fmt.md](fmt.md) に書く。

---

## 1. 使い方

```sh
docv check                     # 管理対象のファイルをすべて確かめる
docv check docs/spec/order.md # 指定したファイルだけ確かめる
docv check --no-textlint       # textlint だけを飛ばす
```

- パスを渡す使い方は、hook から編集したファイルだけを確かめるためにある。管理対象でないパスは無視する
- 1 件ずつ次の形で出し、最後に error と warn の件数を出す

```
docs/spec/order.md:12: warn past-wording 「以前は」は過去の言い回し。経緯は docv retire で archive に移す
docs/plan/009-impl.md:1: error status-invalid status は todo か doing
```

- error が 1 件以上あれば、終了コード 1 で終わる
- tmp のファイルは確かめない（[spec/kind.md](kind.md) の 5 章）

---

## 2. 強さ

- ルールごとに、既定の強さ（error / warn）がある
- docv.yaml の docs の項目で check: warn にすると、その項目のファイルの error を warn に下げる。check: off にすると、そのファイルは確かめない
- docv.yaml がスキーマに合わないときは、ルールを確かめる前に終了コード 2 で終わる

---

## 3. ルール

### frontmatter

| ID | 強さ | 内容 |
|---|---|---|
| frontmatter-invalid | error | frontmatter が YAML として読めない |
| question-missing | error | question がない。docv.yaml でも与えられていない |
| question-multiline | error | question が複数行 |
| question-duplicated | error | question が frontmatter と docv.yaml の両方にある |
| topic-unknown | error | topics に、docv.yaml の topics にない話題がある |
| status-invalid | error | plan の status が todo / doing でない |
| source-missing | error | archive に source がない |

### 構造

| ID | 強さ | 内容 |
|---|---|---|
| title-count | error | h1 が 1 つでない |
| section-required | error | 種類の見出しの制約（[spec/kind.md](kind.md) の 1 章）の語を含む章がない |
| archive-entry-format | error | archive の章の見出しが「YYYY-MM-DD 見出し」になっていない、または章の中に「理由」の行がない |
| plan-done-task | warn | plan に終わったタスク（`- [x]`）が残っている |

### 言い回し

現在の種類のファイルだけを確かめる。コードブロックとインラインコードの中は見ない。

| ID | 強さ | 内容 |
|---|---|---|
| past-wording | warn | 本文に、過去や実装状態の言い回しがある |
| past-heading | warn | 見出しに、過去を扱う語がある |

既定の語は次のとおり。docv.yaml の lint.words.add は past-wording に語を足し、lint.words.remove は両方から語を外す。語は大文字と小文字を区別し、部分一致で探す（種類の fixme や、plan の status の todo は警告しない）。

| ルール | 既定の語 |
|---|---|
| past-wording | `以前は`、`従来は`、`旧仕様`、`旧方式`、`に変更した`、`に変更しました`、`を廃止した`、`を廃止しました`、`未実装`、`実装予定`、`実装済み`、`今後は`、`将来的に`、`TODO`、`FIXME` |
| past-heading | `変更履歴`、`更新履歴`、`経緯`、`廃案`、`不採用`、`未実装`、`今後の予定` |

### リンク

| ID | 強さ | 内容 |
|---|---|---|
| link-broken | error | ルートの中を指す相対リンクの先にファイルがない。archive のファイルは確かめない |
| link-to-tmp | error | tmp のファイルを指すリンクがある。tmp はいつ消えるか分からないため。tmp を指すリンクには link-broken を出さない |

アンカー（`#` 以降）は確かめない。

### 使うと決めたときだけ確かめるもの

| ID | 強さ | 使う条件 | 内容 |
|---|---|---|---|
| style-* | error | lint.style に規則を書いたとき | 書式の規則（[spec/style.md](style.md)） |
| format-diff | error | format.enabled が true のとき | 整形が必要（[spec/fmt.md](fmt.md)） |
| textlint/* | lint.textlint.level（既定は warn） | lint.textlint.enabled が true のとき | textlint の結果（[spec/textlint.md](textlint.md)） |
