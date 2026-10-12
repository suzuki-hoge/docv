# docv のルール

このリポジトリのドキュメントは docv で管理している。ドキュメントを読む・書く・直すときは、このルールに従う。枠は docv/ に、原本は docs/ にあり、原本が答える問いは frontmatter の mission に書いてある。

---

## 1. 考え方

- ドキュメントは、枠と原本の 2 つからなる
  - 枠: インストール、サービス仕様、利用マニュアルのように、どのプロジェクトにもある問いに、決まった形の YAML で答えるファイル。docv/ に置く。人間は枠ごとの HTML のページで見る
  - 原本: ファイルごとに答えること（mission）を frontmatter に宣言した Markdown。詳細の記録で、AI Agent は必要なファイルだけを読む
- 原本の中身は、問いへの答えだけにする。要点を本文に書き、細かい理由や例外は `<details>` に入れる
- 原本は、最新の状態だけを書く。状態が変わったら、前の内容を直接書き換える
- やったことはドキュメントに書かず、PR とコミットメッセージに書く
- 仕様は、実装されているかどうかにかかわらず「こうあるもの」として書く。実装状態は書かない
- 分類はタグで行う。タグの意味とルールは、プロジェクトの docv.yaml が決める
- docv.yaml の docs に当てはまるファイルだけが管理対象。それ以外は管理外で、自由に書いてよい

---

## 2. 枠

枠のファイルは、docv.yaml の slots.dir（既定は docv）に `<まとまり>/<枠>.<拡張子>` の名前で置く（まとまりは service、system、guide、operation）。拡張子は枠ごとに決まる。yaml の枠は決まった項目で書き、ファイルの中のパスはすべてルートからの相対パス。md の枠は自由に書き、HTML にそのまま出る。

| ファイル | 枠 | 主な項目 |
|---|---|---|
| service/install.yaml | インストール | requirements、steps（id、name、items（os、markdown）。items が 1 つなら配列にしなくてよい） |
| service/service.md | サービス仕様 | 自由に書く Markdown |
| service/manual.yaml | 利用マニュアル | commands（command、args、description、options（name、description、default）、details） |
| service/words.yaml | 用語集 | groups（name、items（ja、en、desc。ja と en はどちらかがあればよい）） |
| system/stack.yaml | 技術スタック | stack（group、image、items（name、version、purpose、url）） |
| system/system.md | システム仕様 | 自由に書く Markdown |
| system/api.yaml か system/api.md | API 仕様書 | yaml なら ref（外の定義のパスか URL）だけで、HTML ではカードから直接リンクする。md なら自由に書く |
| system/db.yaml か system/db.md | DB 仕様書 | api と同じ |
| guide/directory-tree.yaml | ディレクトリ | entries（path、description） |
| guide/setup.yaml | 環境構築 | install と同じ |
| guide/env.yaml | 環境変数 | envs（key、desc、secret、local_from、release_from） |
| guide/develop.yaml | 開発コマンド | install と同じ |
| operation/infrastructure.md | インフラと環境 | 自由に書く Markdown。ない環境も書く |
| operation/deploy.md | デプロイ | 自由に書く Markdown |
| operation/release.md | リリース | 自由に書く Markdown |
| operation/cost.yaml | 費用 | accepted_items、rejected_items（name、desc、monthly、ref） |

- API 仕様書と DB 仕様書を除く yaml の枠には、body（Markdown）と include（`パス` か `パス#見出し` の配列）を書ける。include は、既にある Markdown の章を埋め込む。原本と同じことを枠に書かず、include を使う
- 枠の問いに答える詳しいことも、枠の中に書く。原本へのリンクで済ませない
- 図、API の定義、DB のスキーマそのものは、枠の形式にしない。外の道具で作り、image や files から指す
- 新しく作るときは `docv add <slot>` で、決まったパスに作る（`docv add --list` で一覧を出す）。yaml の枠には項目のサンプルが入る
- ADR は docv/adr/<id>.md に、決定 1 つを 1 ファイルで書く。ファイル名（.md を除く）が id。frontmatter に group、date、supersedes（打ち消す過去の決定の id）を書き、本文の h1 を題にする。本文には背景、決めたこと、結果、選ばなかった案を書く

---

## 3. 書く前に決めること

1. 書くべき変更か。振る舞い、手順、設定、費用、規約のどれかが変わったときだけ書く。コードを変えただけなら書かない
2. 枠か、原本か。枠がある問いなら枠に書く
3. どのファイルか。原本の frontmatter の mission を見て、問いに収まるファイルを選ぶ。収まるファイルがなければ docs/ に新しく作る

---

## 4. 何をどこに書くか

| 書く内容 | 書く場所 |
|---|---|
| どのプロジェクトにもある問いへの答え | 枠 |
| 決まったこと | 原本。前の内容を直接書き換える |
| 今の状態の理由 | 原本に 1〜2 行。細かい理由は `<details>` |
| 決めた経緯、選ばなかった案 | ADR（docv/adr/ の YAML）。決定 1 つを 1 ファイルにし、決めた後は書き換えない |
| やったこと | PR とコミットメッセージ。ドキュメントには書かない |
| 実装状態、コードの改善点 | 書かない。必要なら Issue にする |
| まとまっていない意思決定、後で見るかもしれないメモ | 管理外のファイル。ほかのドキュメントからリンクも言及もしない |

- 原本に `〜に変更した`、`以前は`、`未実装` のような言い回しを書かない
- 古い内容は消す。なぜ変えたか、何と比べて選んだかを残したいときは ADR を書く
- 決定を変えるときは、新しい ADR の supersedes から過去の決定の id を指す。過去の ADR は書き換えない

---

## 5. 範囲を守る

- 問いに収まらない内容を足さない
- 内容を収めるために、問いを広げない
- 問いに対してファイルが大きくなったら、問いを分け、ファイルも分ける

---

## 6. frontmatter

```yaml
---
mission: 注文の処理は、どの注文をいつ確定し、失敗したときにどう扱うか
version: 0.1
tags: [仕様, 注文]
---
```

| key | 対象 | 中身 |
|---|---|---|
| mission | すべての原本 | このファイルが答えること。1 文で書く。疑問の形にすると範囲が決まりやすい |
| version | すべての原本 | このファイルが従う docv の版（例: 0.1）。新しく作るときは `docv --version` の版を書く |
| tags | すべての原本 | タグの配列。docv.yaml に tags があれば、その中から選ぶ |

章番号、文体、強調の使い方などの書式は、プロジェクトの規約に従う。

---

## 7. 読むもの

- 枠と原本は、必要なものを読む
- ADR は、決めた理由を聞かれたときだけ読む
- docv build-html と docv build-image の出力（HTML と JPG）は読まない、書かない

---

## 8. コマンド

```sh
docv add --list                     # 作れる枠の一覧を出す
docv add manual                     # 枠のファイルを決まったパスに作る
docv check                          # ルールに沿っているかを確かめる。ドキュメントを変えたら実行する
docv check docs/order.md            # 指定したファイルだけ確かめる
docv fix docs/order.md              # 整形と textlint の自動修正をする
```

- `docv check` の error は直す。warn も、言い回しの警告なら書き換える。version-older が出たら、`docv migrate-guide` の手順で直す
