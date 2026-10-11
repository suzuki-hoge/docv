---
question: docv retire は何を、どこへ、どんな形で移すか
topics: [CLI]
---

# retire

現在と未来のドキュメントから、章やファイルを archive に移すコマンド。いつ移すかは [spec/writing.md](writing.md) の 3 章、archive という種類は [spec/kind.md](kind.md) の 4 章。

---

## 1. 使い方

```sh
docv retire docs/spec/order.md --section "注文履歴" --reason "履歴は監査ログにまとめた"
docv retire docs/proposal/coupon.md --reason "却下。決済サービスの手数料に見合わない"
```

| 指定 | 動き |
|---|---|
| `--section` あり | 章（h2 と、その下の次の h2 までの内容）を archive に移し、元のファイルから消す |
| `--section` なし | ファイル全体を archive に移し、元のファイルを消す。採用・却下した proposal や、役目を終えた spec に使う |

- `--section` には、章の見出しの文字列（章番号を除いた部分と完全に一致するもの）か、章番号を渡す
- `--reason` は必須。archive の価値は理由にあるため
- 移せるのは、管理対象の現在と未来のファイルだけ。archive と管理外のファイルは移せない

---

## 2. 移す先

- archive.dir の下に、移した元と同じ相対パスで置く。相対パスの起点は archive.dir の親ディレクトリで、その外にあるファイルはルートを起点にする

| archive.dir | 移した元 | 移す先 |
|---|---|---|
| docs/archive | docs/spec/order.md | docs/archive/spec/order.md |
| docs/archive | README.md | docs/archive/README.md |

- 移す先のファイルがなければ作る。frontmatter には次の 2 つを書く
  - question: 「<移した元のパス> から外した内容と、外した理由は何か」
  - source: 移した元のパス

---

## 3. 書く形

新しい章を、archive のファイルの h1 のすぐ下（いちばん上）に足す。

```markdown
## 2026-10-11 注文履歴

- 移した元: [spec/order.md](../../spec/order.md)
- 理由: 履歴は監査ログにまとめた

（移した本文）
```

- 日付は、実行したマシンの今日の日付
- 章の見出しは、移した章の見出し（章番号を除く）。ファイル全体を移したときは、元のファイルの h1
- ファイル全体を移したときは、元の question を「- 元の問い:」の行として足す。元の h2 以下の見出しは 1 段ずつ下げる
- 移した本文の中の相対リンクは、archive のファイルの位置から辿れるように書き換える

---

## 4. 移したあと

- 移した元のファイルや、移した章を指すリンクを持つファイルの一覧を出す。リンクを直すのは利用者
- 移した元のファイルの章番号は振り直さない
