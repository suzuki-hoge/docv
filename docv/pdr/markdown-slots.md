---
group: 出力
date: 2026-11-19
supersedes: [no-image]
---

# Slot をすべて Markdown で書き、JPG をやめる

## 背景

インストール、用語集、技術スタック、環境構築、開発コマンド、環境変数、費用の 7 つの Slot は、決まった項目の YAML で書いていた。YAML の Slot は GitHub でそのまま読めないので、build が Chrome でページを JPG に撮り、インデックスから JPG へリンクしていた。

GitHub で JPG を開いて読むことは、ほとんどなかった。JPG のために、次のものを抱えていた。

- Chrome への依存と、ページを撮る処理
- `generate.image`、`--no-image`、出力先の `image/`、JPG を git で管理すること
- 「JPG は同じマシンでしか同じにならない」という、毎回同じ出力にする決まりの例外
- 版を上げるたびに直す、YAML の Slot の 1 行目のスキーマの URL

YAML の値は、HTML を作るのに使っているだけだった。Markdown にしても、失う機能はない。

## 決めたこと

Slot は、すべて Markdown で書く。JPG は作らない。

- 7 つの Slot の形は、`docv gen` が作る雛形で示す
  - check で確かめるのは、ほかの Slot と同じく `mission` だけにする
  - 列や文を足すのは、利用者の自由にする
  - 雛形の形から外れていないかは、`docv review-guide` のレビューで見る
- 残った YAML の Slot は、check が error にし、雛形を見て Markdown に書き直すよう案内する
- `generate.image`、`--no-image`、出力先の `image/`、Chrome への依存、YAML の Slot のスキーマと `docv slot schema` をなくす
- インデックスは、すべての Slot を Markdown のファイルへリンクする

前の決定のうち、次のところはこの決定で変わる。

| 決定                                                                        | 変わるところ                                                                          |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| [ドキュメントを 1 つのディレクトリに集める](document-root.md)               | 出力先の下は `html/` だけになり、`generate.image` はなくなる                          |
| [docv update-guide で出す](update-guide.md)                                 | check がスキーマの URL の版を比べるのは、`docv.yaml` の 1 行目だけになる              |
| [開発コマンドとインフラと環境も分けられる](develop-infrastructure-pages.md) | 開発コマンドのページも Markdown で書き、ページの名前は h1 にする。ページの JPG はない |

## 結果

- 利用者は、7 つの Slot を `docv gen <slot> --print` の雛形を見て、Markdown に書き直す
- GitHub の PR で、どの Slot も描画した差分を見られる。`mission` を front matter に書けて、check も効く
- build に Chrome が要らなくなり、出力は、どのマシンでも入力が同じなら同じになる
- 雛形の形は check で守られないので、崩れてもレビューまで気づかない

## 見直す条件

- 雛形の形から外れた Slot を、`docv review-guide` のレビューで指摘することが、1 つのプロジェクトで 3 回続いたとき
  - 形を check で確かめる理由になる
- GitHub で Slot を読む人から、表や手順を Markdown より見やすい形で見たいという要望が出たとき

## 選ばなかった案

| 案                                                   | 選ばなかった理由                                                                |
| ---------------------------------------------------- | ------------------------------------------------------------------------------- |
| YAML の Slot を残し、JPG だけをやめる                | YAML の Slot は GitHub で読めず、インデックスから辿れなくなる                   |
| 雛形の形（見出しや表の列）を check で確かめる        | 列や文を足す利用者の自由がなくなる。形は雛形と LLM のレビューで保てる           |
| OS ごとの手順のタブを、Markdown の書き方を決めて残す | docv だけの書き方を作り込むことになる。並べて書くか、`<details>` で畳めば足りる |
