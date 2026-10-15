---
mission: Slot にはどんなものがあり、それぞれのファイルに何をどんな形で書き、HTML でどう見せるか
tags: [仕様, ルール, 出力]
---

# Slot

どのプロジェクトにもある問いに、決まった形で答えるファイルの規格。Slot を置く理由と HTML の全体は、Slot の「サービス仕様」（[`docv/service/service.md`](../../docv/service/service.md)）。

---

## 1. 置き場所と書き方

- Slot ファイルは、`docv.yaml` の `slots.dir`（既定は `docv`）に、`<まとまり>/<Slot>.<拡張子>` の名前で置く。まとまりのディレクトリは `service`（サービス仕様）、`system`（システム仕様）、`guide`（開発者向けガイド）、`operation`（運用）。`docs/` には Free ファイルを、`docv/` には Slot ファイルを置く
- 拡張子は Slot ごとに決まる。YAML の Slot は決まった項目で書き、md の Slot は中身の形を決めず、そのまま HTML に出す
- `docv/adr/` は ADR の置き場所で、Slot としては読まない（[`spec/adr.md`](adr.md)）
- Slot を置くかどうかは任意。プロジェクトに当てはまらない Slot は作らない（例: 利用者の手元に入れるものがなければ、インストールの Slot はいらない）。`index.html` には置いていない Slot のカードも並べ、グレーにしてリンクにしない
- Slot の問いに答える詳しいことも、Slot の中に書く。Free ファイルへのリンクで済ませない
- ファイルの中のパスは、すべてルートからの相対パスで書く。md の Slot の中のリンクは、ほかの Markdown と同じく、ファイルの場所からの相対パスで書く
- YAML の文字列の項目には Markdown を書ける
- `docv add <slot>` で、Slot ファイルを決まったパスに作れる。YAML の Slot には項目のサンプルを書き、md の Slot は空のファイルを作る（`docv add --list` で一覧を出す）

---

## 2. 一覧

HTML には、この表の順に、まとまりごとに並べる。

| ファイル                             | Slot           | まとまり         | 答える問い                                                               |
| ------------------------------------ | -------------- | ---------------- | ------------------------------------------------------------------------ |
| `service/install.yaml`               | インストール   | サービス仕様     | 利用者はどう入れるか                                                     |
| `service/service.md`                 | サービス仕様   | サービス仕様     | これは何で、誰のために、何を解決し、どんな部品からなるか                 |
| `service/manual.md`                  | 利用マニュアル | サービス仕様     | 利用者はどう使うか（CLI のコマンド、GUI の操作など）                     |
| `service/words.yaml`                 | 用語集         | サービス仕様     | このプロジェクトの言葉は何を意味するか                                   |
| `system/stack.yaml`                  | 技術スタック   | システム仕様     | 何で作られているか                                                       |
| `system/system.md`                   | システム仕様   | システム仕様     | システムは、内部でどう動き、どんな決まりに従うか                         |
| `system/api.yaml` か `system/api.md` | API 仕様書     | システム仕様     | どんな API があり、何を受け取り、何を返すか                              |
| `system/db.yaml` か `system/db.md`   | DB 仕様書      | システム仕様     | どんなデータを、どこにどう保存しているか                                 |
| `guide/directory-tree.yaml`          | ディレクトリ   | 開発者向けガイド | どこに何があるか                                                         |
| `guide/setup.yaml`                   | 環境構築       | 開発者向けガイド | 開発を始めるには、何を用意して何を実行するか                             |
| `guide/env.yaml`                     | 環境変数       | 開発者向けガイド | システムを動かすのにどんな環境変数が要り、それぞれどこで設定されているか |
| `guide/develop.yaml`                 | 開発コマンド   | 開発者向けガイド | 開発者はテストやビルドなどをどう実行するか                               |
| `operation/infrastructure.md`        | インフラと環境 | 運用             | どんな環境（本番、ステージング、開発など）があり、それぞれ何に使うか     |
| `operation/deploy.md`                | デプロイ       | 運用             | 作ったものを、どの環境に、どう反映するか                                 |
| `operation/release.md`               | リリース       | 運用             | 版をどう決めて、利用者にどう届けるか                                     |
| `operation/cost.yaml`                | 費用           | 運用             | 何にいくらかかり、費用を理由に何をしていないか                           |

- Slot の名前に近いパスで置くと、check が slot-unknown で正しいパスを示す。たとえば、まとまりのディレクトリが違う `words.yaml`、番号を付けた `service/03-words.yaml`、拡張子が違う `service/service.yaml`
- 利用マニュアル（manual）と開発コマンド（develop）を分けるのは、使う人と目的が違うため。利用マニュアルは、リポジトリが CLI や GUI のときの、利用者向けの使い方
- サービス仕様には、これが何で、どんな部品からなるか（外から見た姿）を書く。システム仕様には、内部でどう動くかを書く
- インフラと環境には、ある環境だけでなく、ない環境も書く（例: ステージングと本番があり、開発サーバーはない）
- デプロイは環境に反映すること、リリースは利用者に届けることで、どちらかしかないプロジェクトもある（例: サーバーを持たない CLI はリリースだけ）。どちらも、環境ごとの違い、承認、切り戻しなど、手順の形に収まらないことを書くので md にする

---

## 3. YAML の Slot の項目

<details>
<summary>手順の Slot（インストール、環境構築、開発コマンド）</summary>

| 項目           | 中身                                                                    |
| -------------- | ----------------------------------------------------------------------- |
| `requirements` | 前提とされるもの。`name`、`version`、`purpose`、`url`                   |
| `steps`        | 手順の配列。`id`、`name`、`items`。`items` は `os` と `markdown` を持つ |

- `id` は手順ごとに重ねない
- `markdown` は、その手順の中身（コマンド、説明など）を Markdown で書く
- `items` が 1 つなら、配列にしなくてよい。HTML ではタブにしない
- `items` が複数あるときは、HTML で `os` ごとのタブにする。どの `item` にも `os` を書き、`os` は重ねない。タブは書いた順に並べる。`os` は windows、macos、linux など

````yaml
steps:
  - id: binary
    name: バイナリを取る
    items:
      - os: windows
        markdown: リリースのページから zip を取り、展開する
      - os: macos
        markdown: |
          ```sh
          curl -L https://example.com/example-macos.tar.gz | tar xz
          ```
  - id: cargo
    name: ソースから入れる
    items:
      markdown: "`cargo install example`"
````

</details>

<details>
<summary>用語集、技術スタック、ディレクトリ</summary>

| Slot           | 項目      | 中身                                                                                                        |
| -------------- | --------- | ----------------------------------------------------------------------------------------------------------- |
| words          | `groups`  | 用語のまとまりの配列。`name` と `items`。`items` は `ja`、`en`、`desc`。`ja` と `en` はどちらかがあればよい |
| stack          | `stack`   | 層の配列。`group`、`image`（任意。図があれば出す）、`items`                                                 |
|                |           | `items` は `name`、`version`、`purpose`、`url`                                                              |
| directory-tree | `entries` | `path` と `description` の配列。書いた順に、パスの深さで字下げして出す                                      |

</details>

<details>
<summary>API 仕様書、DB 仕様書、環境変数、費用</summary>

| Slot    | 項目             | 中身                                                                                                      |
| ------- | ---------------- | --------------------------------------------------------------------------------------------------------- |
| api、db | `ref`            | 外の道具（OpenAPI など）で作った定義のパスか URL。ページを作らず、`index.html` のカードから直接リンクする |
| env     | `envs`           | `key`、`desc`、`secret`、`local_from`、`release_from` の配列                                              |
| cost    | `accepted_items` | 費用をかけて使っているもの。`name`、`desc`、`monthly`、`ref` の配列                                       |
|         | `rejected_items` | 費用を理由にやっていないこと。形は `accepted_items` と同じ                                                |

- API 仕様書と DB 仕様書は、YAML なら `ref` の先へ `index.html` のカードから直接リンクし、md ならページにしてそのまま出す。説明を書きたいときは md にする。両方は置かない
- 環境変数の `secret` は、秘密の値か。`local_from` と `release_from` には、どこで設定しているかを書く
  - `local_from`: ローカル環境（Docker Compose、`.env` など）
  - `release_from`: ステージングや本番（ECS のタスク定義、SSM など）
- 費用の `monthly` は 1 か月あたりの金額で、数でも文字列（「3 ドル」など）でもよい。予算が変われば、`accepted_items` と `rejected_items` の間で項目を移す

</details>

- API 仕様書と DB 仕様書を除く YAML の Slot には、`body` と `include` も書ける。`body` は Slot の最後に出す Markdown、`include` は埋め込む Markdown の配列（`パス` か `パス#見出し`）
- `include` の `パス` はファイル全体を、YAML front matter と h1 を除いて埋め込む。`パス#見出し` は、その見出しの下から、次の同じ深さ以上の見出しの手前までを埋め込む
- YAML の Slot には、項目の型と意味を書いたスキーマ（YAML で書いた JSON Schema）がある。`docv schema <slot>` で出す。元のファイルは `assets/schema/<ID>.yaml` で、`docs/schema/` に写しを置く

---

## 4. md の Slot

- 中身の形は決めない。YAML front matter は要らない
- HTML では、最初の h1 を除き、いちばん浅い見出しが h2 になるように段を付け直して出す
- 見出しには、Free ファイルのページと同じく、GitHub と同じ形のアンカーを付ける。`[docv init](#docv-init)` のようなページ内リンクが HTML でも GitHub でも飛べる
- md の Slot のページは JPG にしない。md は GitHub でそのままプレビューできるため

---

## 5. check

| ID                   | 強さ         | 内容                                                                                                                 |
| -------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------- |
| slot-unknown         | error        | Slot のディレクトリに、Slot の名前でないファイルがある                                                               |
| slot-duplicate       | error        | 同じ Slot ファイルが 2 つある（`api.yaml` と `api.md` など）                                                         |
| slot-invalid         | error        | YAML の Slot の形に合わない。知らない項目、手順の `id` の重なり、`os` のない `item`、`ja` も `en` もない用語も error |
| slot-path-missing    | error / warn | パスの先がない。directory-tree の `path` は warn、それ以外は error                                                   |
| slot-include-missing | error        | `include` のファイルか見出しがない                                                                                   |

- directory-tree の `path` を warn にするのは、ディレクトリの Slot が実装に追いついていないことを知らせるため。実装とずれたことに気づけるのは、ここだけ

---

## 6. HTML での見せ方

- Slot は 1 つずつ独立したページ（`<まとまり>/<Slot>.html`）にする。`index.html`（プロジェクト概要）には、まとまりの見出しの下に、2 章の順で、すべての Slot のカードを横に並べる。紙面を縦に使いすぎないため。置いていない Slot のカードは、グレーにしてリンクにしない。どの問いに答えていないかを見せるため。カードの幅は固定し、画面の幅に合わせない。幅は、いちばん長い Slot の名前（7 文字）が折り返さない大きさ。収まらないカードは横にスクロールして見る
- Slot のカードと、Slot のページの見出しには、Slot ごとのアイコンを付ける。アイコンは、docv が Slot ごとに持つ線画の SVG で、外部の素材を使わない
- Slot のページには、Slot の名前の見出しを出す。2 章の問いは HTML には出さない。問いは、AI Agent がどの Slot に書くかを決める手がかりにする
- 折りたたみは使わない。`details` も、Markdown の中の `<details>` も、開いて出す。画像にしても、すべての中身が読めるようにするため
- 開閉するのは、OS ごとの手順のタブだけ。JPG では、すべての OS の手順を並べて写す
- 用語集は、まとまりごとに `name` を h2 にし、`items` を「日本語 | 英語 | 意味」の表にする
- 見た目に凝らない。見出し、段落、箇条書き、表だけで出す
- 見出しは、ページの先頭の h1（Slot の名前）が 1 つだけで、その下に h2 を並べる。段は飛ばさない（h1 の次に h3 や h4 を置かない）
  - Markdown（md の Slot、`body`、`include`、`details`、手順の中身）の見出しは、置く場所に合わせて段を付け直す。md の Slot、`body`、`include` は h2 から、`details` と手順の中身は h3 から
- 手順（インストール、環境構築）は、「手順」の h2 の下に番号付きで並べる
- 開発コマンドは、順番のない項目なので番号を付けない。先頭に目次（項目へのページ内リンク）を置き、各項目を h2 で始める
