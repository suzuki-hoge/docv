---
question: docv の CLI にはどんなコマンドがあり、共通してどう動くか
topics: [CLI]
---

# CLI

docv の CLI のコマンドの一覧と、すべてのコマンドに共通する動き。大きいコマンドは別のファイルに書く。配布は [spec/distribution.md](distribution.md)。

---

## 1. コマンド一覧

| コマンド | やること | 詳細 |
|---|---|---|
| `docv init` | docv.yaml を作る | 4 章 |
| `docv new <パス>` | テンプレからファイルを作る | 4 章 |
| `docv check [パス...]` | 管理対象のファイルがルールに沿っているかを確かめる | [spec/check.md](check.md) |
| `docv retire <ファイル>` | 章やファイルを archive に移す | [spec/retire.md](retire.md) |
| `docv index` | AI 用のインデックスを出す | 4 章 |
| `docv desc` | フレームワークのルールファイルを出す | 4 章 |
| `docv build` | HTML と JPG を作る | [spec/build.md](build.md) |
| `docv migrate-guide` | 後から導入するときのガイドを出す | [spec/migrate-guide.md](migrate-guide.md) |

---

## 2. 共通の動き

### ルートと設定ファイル

- カレントディレクトリから親へ向かって docv.yaml を探し、見つかったディレクトリをルートにする
- `--config <パス>` で docv.yaml を直接指定できる。このときのルートは `--root <パス>` で指定し、省略するとカレントディレクトリになる
- `--config` は、リポジトリに docv.yaml を置かずに試すためにある

### ファイルの分類

ルート以下の .md のファイルを集め、次のように分ける。

| 分類 | 条件 |
|---|---|
| 除外 | .gitignore に書かれている、output.dir の中にある、ignore に当てはまる |
| 管理対象 | docs のどれかの項目に当てはまる。種類が決まる |
| 管理外 | それ以外 |

### 出力と終了コード

- 結果は標準出力に、エラーは標準エラーに出す
- 終了コード

| コード | 意味 |
|---|---|
| 0 | 成功。check で warn だけが出たときも含む |
| 1 | check で error が 1 件以上ある |
| 2 | docv.yaml がスキーマに合わない、引数が正しくない、など実行できない |

---

## 3. ファイルを書き換えるコマンド

| コマンド | 書くもの |
|---|---|
| init | docv.yaml だけ |
| new | 指定したファイルだけ |
| retire | 移す元のファイルと、archive のファイルだけ |
| build | output.dir の中だけ |

- それ以外のコマンドはファイルを書かない
- CLAUDE.md、リポジトリのルートの .gitignore、hook の設定には、どのコマンドも触らない

---

## 4. 小さいコマンド

### init

- 推奨する配置（[spec/config.md](config.md) の 5 章）の docv.yaml を作る
- スキーマの URL は、実行した CLI の版に合わせる
- docv.yaml がすでにあればエラーにする

### new

- `docv new <パス>` のパスから docs の項目を引いて種類を決め、その種類のテンプレでファイルを作る
- どの項目にも当てはまらないパスはエラーにする。管理外のファイルは docv を使わずに作ればよい
- ファイルがすでにあればエラーにする
- archive は new では作らない（retire が作る）

### index

- 管理対象の現在と未来のファイルを、時間と種類ごとに、パス・問い・話題で並べる
- 過去のファイルは、archive.dir の場所とファイル数だけを出す。`--all` を付けると全部並べる
- 管理外のファイルはパスだけを並べる
- `--format md | json`。既定は md

```
# docv index: shop-app

## 現在
### spec
- docs/design/order.md — 注文の処理は、どの注文をいつ確定し、失敗したときにどう扱うか [注文]
### cost
- docs/pricing/payment.md — 決済サービスに何にいくらかかり、費用を理由に何を決めているか

## 未来
### plan
- docs/plan/003-release.md (doing) — リリースまでに人間が何をやるか

## 過去
- docs/archive/ に 3 ファイル。経緯を聞かれたときだけ読む

## 管理外
- docs/memo.md
```

### desc

- フレームワークのルールファイルを出す。ルールファイルはバイナリに埋め込まれていて、CLI の版と内容が一致する
- 中身は [spec/concept.md](concept.md)、[spec/kind.md](kind.md)、[spec/writing.md](writing.md) を AI Agent 向けにまとめたものと、AI Agent が使うコマンド（index、check、new、retire）の使い方
- 利用者が CLAUDE.md などに「セッションの最初に `docv desc` と `docv index` を実行する」と書いておけば、AI Agent にルールが伝わる
