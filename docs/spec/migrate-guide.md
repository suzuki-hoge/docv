---
question: 既存のプロジェクトに後から docv を入れるとき、docv migrate-guide は何を示すか
topics: [CLI]
---

# migrate-guide

既存のプロジェクトに後から docv を入れるときに、何が管理外で、何がルールに沿っていなくて、次に何をすればいいかを示すコマンド。少しずつ入れていく考え方は [spec/concept.md](concept.md) の 6 章。

---

## 1. 使い方

```sh
docv migrate-guide
docv migrate-guide --config ../trial/docv.yaml --root .   # リポジトリに docv.yaml を置かずに試す
```

- docv.yaml がなくても動く。そのときは、すべてのファイルを管理外として扱う
- ファイルは書かない。直すのは利用者で、出力と `docv desc` を AI Agent に渡して直させてもよい

---

## 2. 出すもの

| 順 | 見出し | 中身 |
|---|---|---|
| 1 | 設定 | docv.yaml の有無。なければ `docv init` を案内する |
| 2 | 管理外のファイル | ディレクトリごとの件数と、ファイルの一覧 |
| 3 | docs の案 | 管理外のディレクトリの名前から種類を推測し、docv.yaml の docs に足す項目を YAML で出す |
| 4 | ルールに沿っていないファイル | 管理対象のファイルごとに、満たしていないルールを並べる。ルールごとの件数も出す。docv.yaml で使うと決めた書式の規則、整形、textlint の結果も含む |
| 5 | 次にやること | 1〜4 の結果から、やる順に並べる。整形が必要なファイルがあれば、ほかを直す前に `docv fmt` をすすめる（整形の差分と内容の差分を混ぜないため） |

---

## 3. 種類の推測

ディレクトリ名（大文字と小文字は区別しない）に次の語が含まれていれば、その種類を案として出す。

| 語 | 種類 |
|---|---|
| spec、specification、architecture、design | spec |
| operation、command、manual、setup、runbook | operation |
| cost、price、pricing | cost |
| rule、guideline、coding、convention | rule |
| plan、task、todo | plan |
| proposal、rfc、draft | proposal |
| fixme、question | fixme |
| archive、history、deprecated | archive |

- generated を含むディレクトリは、種類を推測せず、ignore に足す案を出す
- ルートの README.md は、overview の案を出す
- どれにも当てはまらないディレクトリは、案を出さずに管理外のまま残す

---

## 4. 出力の例

```
## 1. 設定
docv.yaml: なし。docv init で作る

## 2. 管理外のファイル（12 件）
docs/design/  5 件
docs/manual/  4 件
docs/plan/    3 件

## 3. docs の案
docs:
  - path: README.md
    kind: overview
    question: （書く）
  - path: docs/design/*.md
    kind: spec
  - path: docs/manual/*.md
    kind: operation
  - path: docs/plan/*.md
    kind: plan

## 4. ルールに沿っていないファイル
（docv.yaml がないため、なし）

## 5. 次にやること
1. docv init で docv.yaml を作り、3 の案を docs に足す
2. もう一度 docv migrate-guide を実行し、4 に出たファイルを直す
```
