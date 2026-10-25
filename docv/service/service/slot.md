---
order: 2
mission: 外から見た機能と、それぞれの振る舞いと決まり
---

# Slot

どのプロジェクトにも要るドキュメントを、docv が決めたミッションと形で書くファイルの規格。Slot を置く理由は [概要](desc.md) のページに、HTML の全体は [ビジュアライズ](html.md) のページに書く。

---

## 1. 置き場所と書き方

- Slot ファイルは、`docv.yaml` の `dir.slot`（既定は `docv`）に、`<まとまり>/<Slot>.<拡張子>` の名前で置く。まとまりのディレクトリは `service`（サービス）、`system`（システム）、`guide`（開発者向けガイド）、`operation`（運用）。`docs/` には Free ファイルを、`docv/` には Slot ファイルを置く
- 拡張子は Slot ごとに決まる。YAML の Slot は決まった項目で書き、md の Slot は中身の形を決めず、そのまま HTML に出す
- サービス仕様とシステム仕様だけは、ページに分けて `service/service/` と `system/system/` に置ける（「md の Slot」の章）
- `docv/adr/` は ADR の置き場所で、Slot としては読まない（[ADR](adr.md) のページ）
- `dir.slot` の中でも、除外に当たるファイル（[設定ファイル](config.md) のページの「管理対象と管理外」の章）と、名前がドットで始まるファイルとディレクトリ（`.DS_Store` など）は読まない。それ以外で Slot の名前でないファイルは、check が slot-unknown の error にする
- Slot を置くかどうかは任意。プロジェクトに当てはまらない Slot は作らない（例: 利用者の手元に入れるものがなければ、インストールの Slot はいらない）
- Slot のミッションに当たる詳しいことも、Slot の中に書く。Free ファイルへのリンクで済ませない
- ファイルの中のパスは、すべてルートからの相対パスで書く。md の Slot の中のリンクは、ほかの Markdown と同じく、ファイルの場所からの相対パスで書く
- YAML の文字列の項目には Markdown を書ける
- `docv slot gen <slot>` で、Slot ファイルを決まったパスに作れる。YAML の Slot には項目のサンプルを書き、md の Slot には YAML front matter の `mission` だけを書く（`docv slot list` で一覧とミッションを出す）

---

## 2. 一覧

HTML には、この表の順に、まとまりごとに並べる。

| ファイル                      | Slot           | まとまり         | ミッション                                                       |
| ----------------------------- | -------------- | ---------------- | ---------------------------------------------------------------- |
| `service/overview.md`         | サービス概要   | サービス         | 利用者に向けた、これが何で、誰の何を解決し、何ができるかの説明   |
| `service/service.md`          | サービス仕様   | サービス         | 外から見た機能と、それぞれの振る舞いと決まり                     |
| `service/manual.md`           | 利用マニュアル | サービス         | 利用者の使い方（CLI のコマンド、GUI の操作など）                 |
| `service/install.yaml`        | インストール   | サービス         | 利用者がこれを入れる手順                                         |
| `service/words.yaml`          | 用語集         | サービス         | このプロジェクトの言葉の意味                                     |
| `system/system-overview.md`   | システム概要   | システム         | 開発者に向けた、システムを作る部品と、そのつながりの全体像       |
| `system/system.md`            | システム仕様   | システム         | 部品ごとの振る舞いと、部品の間の流れと、部品をまたいで守る決まり |
| `system/stack.yaml`           | 技術スタック   | システム         | 作るのに使っている言語、ライブラリ、サービス                     |
| `system/api.md`               | API 仕様書     | システム         | API の一覧と、それぞれが受け取るものと返すもの                   |
| `system/db.md`                | DB 仕様書      | システム         | 保存しているデータと、その保存先と形                             |
| `guide/directory-tree.yaml`   | ディレクトリ   | 開発者向けガイド | ディレクトリごとの役割                                           |
| `guide/setup.yaml`            | 環境構築       | 開発者向けガイド | 開発を始めるのに用意するものと、実行するコマンド                 |
| `guide/env.yaml`              | 環境変数       | 開発者向けガイド | システムを動かすのに要る環境変数と、それぞれを設定している場所   |
| `guide/develop.yaml`          | 開発コマンド   | 開発者向けガイド | テストやビルドなど、開発者が実行するコマンド                     |
| `guide/doc-guide.md`          | Doc ガイド     | 開発者向けガイド | このプロジェクトのドキュメントの置き場所と書き方                 |
| `operation/infrastructure.md` | インフラと環境 | 運用             | 環境（本番、ステージング、開発など）の一覧と、それぞれの用途     |
| `operation/deploy.md`         | デプロイ       | 運用             | 作ったものを環境に反映する手順                                   |
| `operation/release.md`        | リリース       | 運用             | 版の決め方と、利用者への届け方                                   |
| `operation/cost.yaml`         | 費用           | 運用             | かかっている費用と、費用を理由にやっていないこと                 |

- Slot の名前に近いパスで置くと、check が slot-unknown で正しいパスを示す。たとえば、まとまりのディレクトリが違う `words.yaml`、番号を付けた `service/03-words.yaml`、拡張子が違う `service/service.yaml`
- 利用マニュアル（manual）と開発コマンド（develop）を分けるのは、使う人と目的が違うため。利用マニュアルは、リポジトリが CLI や GUI のときの、利用者向けの使い方
- サービス概要には、これは何で、何に使えるかを、利用者に向けて書く。サービス仕様には、外から見た機能と、それぞれがどう振る舞うかを詳しく書く。実装をそっくり差し替えても変わらないことが、サービス仕様になる
- システム概要には、どんな部品からなり、どうつながっているかを、開発者に向けて図で書く。システム仕様には、部品ごとの振る舞いと、部品をまたいで守る決まりを詳しく書く。部品を作る言語やライブラリは、技術スタックに書く
- インフラと環境には、ある環境だけでなく、ない環境も書く（例: ステージングと本番があり、開発サーバーはない）
- デプロイは環境に反映すること、リリースは利用者に届けることで、どちらかしかないプロジェクトもある（例: サーバーを持たない CLI はリリースだけ）。どちらも、環境ごとの違い、承認、切り戻しなど、手順の形に収まらないことを書くので md にする

---

## 3. YAML の Slot の項目

- 項目の形と意味は、Slot ごとのスキーマ（JSON Schema）で決める。`docv slot schema <slot>` で出す。元のファイルは `assets/schema/<ID>.schema.json` で、`docs/schema/` に写しを置く
- `docv slot gen` は、YAML の Slot の 1 行目に、yaml-language-server の書式でスキーマの URL を書く。URL は、実行した CLI の版のタグの `assets/schema/` を指す。エディタで項目の補完と検証が効く
- 名前が `_markdown` で終わる項目は Markdown。値は、1 行でも必ず `|` のブロックで書く。`|-`、`|+`、字下げの数を付けた `|2` などは使わない。それ以外の文字列の項目は、Markdown として扱わず、1 行で書く
- Markdown の項目の中のリンクは、ルートからの相対パスで書く
- どの YAML の Slot にも、`appendix_markdown` を書ける。項目に収まらない補足を書く

```yaml
groups:
  - name: ファイルの種類
    items:
      - en: Slot
        desc_markdown: |
          どのプロジェクトにも要るドキュメントを書くファイル。`docv/` に置く
```

---

## 4. md の Slot

- 中身の形は決めない
- YAML front matter には `mission` だけを書き、`docv slot list` のミッションと同じ文にする。ミッションは docv が決めるもので、プロジェクトでは変えない。ないか違えば、check が slot-mission の error にする。docv の版を上げてミッションが変わったときも、check で気づいて直す。ファイルを開けば、何を書く Slot かが分かるようにするため
- `mission` がないか違っても、HTML には本文を出す
- `docv slot gen` は、YAML front matter の `mission` だけを書いたファイルを作り、中身の形は自由だと表示する。Doc ガイド（`guide/doc-guide.md`）だけは、docv が勧めるガイドラインを書いた雛形を作る。雛形にも `mission` を書いてある。作った雛形はプロジェクトのもので、自由に直してよい。docv の版を上げても書き換えない

サービス仕様とシステム仕様は、ページに分けられる。1 つのページが長くならないようにするため。読む人ごとに仕様が分かれるサービス（利用者向けの機能と管理者向けの機能など）や、部品の多いシステム（画面、サーバ、バッチなど）で分ける。

- 分けるときは、1 つのファイルの代わりに、ページのディレクトリの直下に 1 ページ 1 つの Markdown で置く。サービス仕様は `service/service/`、システム仕様は `system/system/` に置く。ファイル名は自由。両方に置くと check の error になる
- どのページにも、YAML front matter に `order`（整数）と `mission` を書く。ページは `order` の順に並べる。`order` は重ねない。`mission` は、分けないときと同じく、その Slot のミッションと同じ文にする
- 本文の h1 が、ページの名前になる。h1 は重ねない
- 分けないときは、今までどおり `service/service.md` か `system/system.md` に書く

```markdown
---
order: 2
mission: 外から見た機能と、それぞれの振る舞いと決まり
---

# 管理者向け機能
```

---

## 5. check

| ID                  | 内容                                                                                                                              |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| slot-unknown        | Slot のディレクトリに、Slot の名前でないファイルがある                                                                            |
| slot-invalid        | YAML の Slot の形に合わない。知らない項目、手順の `id` や `os` の重なり、`os` のない `item`、`ja` も `en` もない用語も含む        |
|                     | 分けたサービス仕様とシステム仕様のページの形に合わない。YAML front matter の `order` がない、知らない項目、`order` や h1 の重なり |
|                     | md の Slot の YAML front matter に、知らない項目がある。閉じる `---` がない                                                       |
|                     | 同じ Slot が、1 つのファイル（`service/service.md` など）と、ページのディレクトリ（`service/service/` など）の両方にある          |
| slot-mission        | md の Slot の YAML front matter に `mission` がないか、`docv slot list` のミッションと違う                                        |
| slot-path-missing   | パスの先がない                                                                                                                    |
| slot-markdown-style | `_markdown` で終わる項目を、`\|` のブロックで書いていない                                                                         |
| slot-text-multiline | `_markdown` で終わらない文字列の項目が、複数行になっている                                                                        |

- md の Slot には、YAML front matter のほかは、Free ファイルと同じ検査（[Free](free.md) のページの「check のルール」の章）をかける
- YAML の Slot の Markdown の項目には、リンク、書式の規則、整形、textlint の検査をかける。見出しの検査はかけない。項目の中に h1 はなく、HTML では見出しの段を付け直すため。指摘の行番号は、YAML の行番号で出す

- directory-tree の `path` の先がなくなったら、ディレクトリの Slot が実装からずれたということ。実装とずれたことに気づけるのは、ここだけ
