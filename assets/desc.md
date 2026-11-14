---
order: 1
mission: 外から見た機能と、それぞれの振る舞いと決まり
---

# 概要

docv の解決したいこと

- AI Agent の開発速度に人間の理解がついていけるように、ドキュメントの構造化ルールを提供し人間用のビジュアライズを行う
- AI Agent が無秩序にドキュメントを肥大化させないように、薄いガイドラインを提供する

docv の提供するもの

- およそ普遍的に必要になるドキュメントのテンプレート
- Free ファイル（`docv/free/` の Markdown）の YAML front matter
- それらのビジュアライズ HTML 生成
- AI Agent に伝えるドキュメント管理のガイドラインの雛形

このツールそのものには LLM は含まない

## 1. コンセプト

docv が扱うドキュメントは、すべて `docv/` の下に置く

```
docv/
  README.md   # docv build が作るインデックス。手で書き換えない
  free/       # Free ファイル
  slot/       # Slot ファイル
  adr/        # ADR
  pdr/        # PDR
```

- 配置は docv が決める。`docv.yaml` で変えられるのは、`docv/` の名前 ( `dir.document_root` ) だけ
- `docv/` の直下には、これ以外のものを置けない ( `docv check` が error にする )
- docv build の出力 ( HTML と JPG ) は `output/docv/` ( `dir.output` ) に出す。`docv/` の中には置けない。AI Agent が `docv/` を読むときに、同じ内容が倍入らないようにするため
- メモや下書きなど、docv に読ませないファイルは、`docv.yaml` の `docv_exclude` に glob で書く ( 例: `docv/free/tmp/**` )

### `docv/slot/`

`docv/slot/` 配下のファイルを Slot ファイルと呼ぶ

- プロジェクトの `docv/slot/` ディレクトリに docv 指定のファイルを配置するとビジュアライズされ HTML で閲覧できる
- 環境構築や用語集などそれぞれのページを Slot といい、Slot ごとに所定の書式がある
- 設定すれば、HTML に加え JPG 画像も出力する
    - GitHub で直接ドキュメントを閲覧できるようにするため
- Slot ファイルは複数プロジェクトのドキュメントルールや閲覧方法を統一する目的のために存在するため、強制力が強い
    - ミッションの変更やファイル名の変更は不可能

### `docv/free/`

`docv/free/` 配下の Markdown を Free ファイルと呼ぶ

- Slot ファイル以外のドキュメントは、Free ファイルとして `docv/free/` に Markdown で置く
- ビジュアライズ時は、ファイルは通常の Markdown → HTML 変換のほか、リンク切れチェックや textlint / format ( 設定次第 ) も行われる
- YAML front matter でタグなどをつけ管理する
- Free ファイルは docv でサポートしないプロジェクト固有の情報を任意の構造で管理するためのもので、強制力は低い
    - ミッションの決定は利用者側に委ねられている
    - docv はタグ付けとビジュアライズ HTML への組み込み程度を提供する

### 画像

Free ファイル、Slot ファイル、ADR、PDR で使う画像の置き方

- 画像は、使うファイルと同じディレクトリに、`{使うファイルの名前}_{画像の名前}.{拡張子}` の名前で置く
    - 例: `docv/free/order.md` で使う流れの図は `docv/free/order_flow.svg`
    - 例: `docv/slot/system/system-overview.md` で使う構成図は `docv/slot/system/system-overview_parts.svg`
- 共通の画像のディレクトリは作らない。複数のページで使う画像は、主になるページの隣に置き、ほかのページからはそのパスを参照する
- ADR と PDR は、それぞれのディレクトリの中の画像だけを使う。ほかの画像を使いたいときは、写しを置く
    - ADR と PDR は、それ単体で完結することに価値があるため。ほかのドキュメントの画像を参照すると、最新のドキュメントの内容に過去の決定が左右されてしまう
- 持ち主 ( 名前の `_` より前と同じ名前の Markdown か YAML のファイル ) のない画像は、`docv check` が error にする
- 画像として扱うのは、拡張子が png、jpg、jpeg、gif、svg、webp のファイル

## 2. Slot

`docv slot list` で一覧と、それぞれの Slot のミッションを確認でき、作成したい場合は `docv slot gen <slot>` で作成する

md の Slot には、YAML front matter の `mission` に、その Slot のミッションを `docv slot list` のとおりに書く

- ミッションは docv が決めるもので、プロジェクトでは変えられない
- `mission` がないか、ミッションと違えば、`docv check` が error にする。docv の版を上げてミッションが変わったときも、check で気づいて直す

```markdown
---
mission: 利用者の使い方（CLI のコマンド、GUI の操作など）
---

# 利用マニュアル
```

### ページ分け

サービス仕様、システム仕様、開発コマンド、アーキテクチャ、設計・実装、テスト方針、インフラと環境は、ページに分けられる

- 分けるときは、1 つのファイルの代わりに、次のディレクトリの直下に 1 ページ 1 ファイルで置く。拡張子は、分けないときと同じ
    - サービス仕様: `docv/slot/service/service.md` の代わりに `docv/slot/service/service/`
    - システム仕様: `docv/slot/system/system.md` の代わりに `docv/slot/system/system/`
    - 開発コマンド: `docv/slot/guide/develop.yaml` の代わりに `docv/slot/guide/develop/`
    - アーキテクチャ: `docv/slot/guide/architecture.md` の代わりに `docv/slot/guide/architecture/`
    - 設計・実装: `docv/slot/guide/design.md` の代わりに `docv/slot/guide/design/`
    - テスト方針: `docv/slot/guide/testing.md` の代わりに `docv/slot/guide/testing/`
    - インフラと環境: `docv/slot/operation/infrastructure.md` の代わりに `docv/slot/operation/infrastructure/`
- ページは `order`（整数）の順に並ぶ。ページの名前は、HTML でページを切り替えるタブの名前になる。`order` もページの名前も、同じ Slot のページどうしで重ねない
- 分けないときは 1 つのファイルに書く

md のページは、YAML front matter に `order` と `mission` を書き、本文の h1 をページの名前にする。`mission` は、分けないときと同じ、その Slot のミッション

```markdown
---
order: 1
mission: 外から見た機能と、それぞれの振る舞いと決まり
---

# 利用者向け機能
```

YAML のページ（開発コマンド）は、分けないときの項目に、`order` と、ページの名前の `title` を足す

```yaml
order: 1
title: Web 画面
steps:
  - id: test
    name: テストを実行する
    items:
      body_markdown: |
        npm test
```

## 3. YAML front matter

Free ファイル ( `docv/free/` の Markdown ) を管理するための YAML front matter

```yaml
---
mission: 注文を確定するタイミングと、失敗したときの扱い
tags: [ 仕様, 注文 ]
---
```

### `mission`

必須

HTML にインデックスするときに添える
また AI Agent が適切にメンテできるようにするための目印

### `tags`

任意 / リスト形式

HTML にインデックスするときに組み込まれる

## 4. ADR

ADR（Architecture Decision Record）は、技術の決定 1 つを、背景や選ばなかった案と一緒に残す記録

プロダクトやサービスの仕様の変更は、ADR ではなく PDR に残す

- `docv/adr/<id>.md` に、決定 1 つを 1 ファイルで置く ( ファイル名から `.md` を除いたものが id )
- 画像は `docv/adr/<id>_<画像の名前>.<拡張子>` に置き、`docv/adr/` の中の画像だけを使う ( 外の画像や URL の画像は `docv check` が error にする )
- ファイルは手で作る
- 本文は、h1 に題を書き、次の章をこの順に h2 で書く。任意の章は、書くことがなければ置かない
    - `## 背景`: 決める必要が生じた課題と、そのときの制約や前提
    - `## 決めたこと`: 決めたことと、その案を選んだ理由。決めたことが当てはまる範囲
    - `## 決めないこと`（任意）: この ADR では決めないことと、それをどこで決めるか
    - `## 結果`: 決めたことで変わること。引き受けること（できなくなること、後で考えること）も書く
    - `## 選ばなかった案`: 比べた案と、それぞれを選ばなかった理由
    - `## 参考`（任意）: 根拠にしたファイルや資料へのリンク
- 決めた後は書き換えない
    - 決定を変えるときは、新しい ADR を書き、その `supersedes` で過去の決定の id を指す
- HTML では、`group` ごとに決定のつながりを図にする

```markdown
---
group: 言語
date: 2026-03-01
supersedes: [ go ]
---

# Rust で書く

## 背景

配布するバイナリを 1 つにしたい。利用者の手元に、言語の実行環境を求めたくない

## 決めたこと

CLI は Rust で書き、単一のバイナリにして配る。CLI の外で動く道具（textlint など）は、この決定の範囲に含めない

## 決めないこと

バイナリをどこで配るかは決めない。リリースの Slot で決める

## 結果

利用者は、言語の実行環境なしで使える。開発者は、Rust の環境を用意する

## 選ばなかった案

| 案             | 選ばなかった理由             |
| -------------- | ---------------------------- |
| Node.js で書く | 利用者の手元に実行環境が要る |
| Zig で書く     | 情報が少ない                 |

## 参考

- [技術スタック](../system/stack.yaml)
```

### `group`

任意

決定のまとまりの名前
省略すると「その他」

### `date`

必須 / YYYY-MM-DD

決めた日
決定を並べる順に使う

### `supersedes`

任意 / リスト形式

打ち消す過去の決定の id

## 5. PDR

PDR（Product Decision Record）は、プロダクトやサービスの仕様の変更 1 つを、その理由や見直す条件と一緒に残す記録。料金、プラン、提供する機能の範囲、対象とする利用者などを足す・変える・やめるときに書く。なぜ今の仕様になったのかを将来も分かるようにするための履歴なので、案を比べて選んだ決定に限らない

- `docv/pdr/<id>.md` に、変更 1 つを 1 ファイルで置く
- 画像は `docv/pdr/<id>_<画像の名前>.<拡張子>` に置き、`docv/pdr/` の中の画像だけを使う
- ファイルの作り方、YAML front matter の項目、書き換えないこと、HTML の図は ADR と同じ
- `supersedes` は PDR だけを指す。ADR と PDR は、同じ id でも別の決定
- 本文の章は、ADR の章の `## 結果` の後に `## 見直す条件` を足し、`## 選ばなかった案` を任意にしたもの
    - `## 背景`: 決める必要が生じた課題と、そのときの利用者、市場、事業の状況
    - `## 決めたこと`: 決めたことと、その案を選んだ理由。決めたことが当てはまる範囲（対象の利用者、プラン、機能など）
    - `## 決めないこと`（任意）: この PDR では決めないことと、それをどこで決めるか
    - `## 結果`: 利用者と事業に起きると見込むこと。引き受けること（できなくなること、後で考えること）も書く
    - `## 見直す条件`: どんな状況になったら、この決定を見直すか。数や時期など、起きたかどうかを確かめられる形で書く
    - `## 選ばなかった案`（任意）: 比べた案があれば、それぞれを選ばなかった理由
    - `## 参考`（任意）: 根拠にしたファイルや資料へのリンク
- 見直す条件に当たっても書き換えない。決定を変えるなら、新しい PDR を書き、その `supersedes` で過去の決定の id を指す

```markdown
---
group: 料金
date: 2026-04-01
supersedes: [ free-only ]
---

# 注文の多い店舗に有料のプランを求める

## 背景

無料のプランだけで提供してきた。月の注文が 1 万件を超える店舗が増え、サーバの費用が売上を上回った

## 決めたこと

月の注文が 1,000 件を超える店舗には、有料のプランを求める。1,000 件までは、今までどおり無料にする

## 結果

注文の多い店舗の一部は、解約すると見込む

## 見直す条件

- 有料のプランに移った店舗が、対象の店舗の半分を下回ったとき
- 2027 年 3 月の時点で、サーバの費用が売上を上回ったままのとき

## 選ばなかった案

| 案                       | 選ばなかった理由                 |
| ------------------------ | -------------------------------- |
| すべての店舗を有料にする | 小さな店舗が使い始められなくなる |
```

## 6. コマンド

すべてのコマンドは `docv --help` と `docv <sub-command> --help` で確認する

新たに導入する場合は `docv init` し、`docv slot gen doc-guide` でガイドラインを作り、`docv slot gen` と `docv check` で作成・調整を進める

## 7. スキーマ ( 設定ファイル )

設定ファイル ( `docv.yaml` ) の項目は `docv init --print` で確認する

すべての項目が、既定の値でコメントとして載っている

## 8. スキーマ ( docv ファイル )

YAML の Slot は `docv slot list` で確認し、そのスキーマは `docv slot schema <slot>` で確認する

## 9. ルール、ガイドライン、設定

docv の決まりは、次の 3 つに分かれる

- ルール: docv の仕組みが決める決まり。Slot と `docv.yaml` のスキーマ、YAML front matter の key、check が確かめることなど。利用側は変えられない。この概要と `docv slot schema <slot>` で確認する
- ガイドライン: ドキュメントを書くときの方針。`docv slot gen doc-guide` で Doc ガイド（`docv/slot/guide/doc-guide.md`）に雛形を作り、プロジェクトに合わせて自由に直す
- 設定: `docv.yaml` で切り替えること。check のルールや textlint を使うか、出力を git で管理するかなど。`docv init --print` で確認する

## 10. docv の更新

`docv.yaml` の `docv_version` に、ドキュメントを合わせた docv の版を書く

- docv の版と違えば、パッチの版の違いでも `docv check` が error にする
- docv を更新したら、`docv update-guide --from <docv_version>` で版ごとの変更点と対応を確かめ、対応する
- 対応が終わったら、`docv_version` と、`docv.yaml` と YAML の Slot の 1 行目にあるスキーマの URL の版を、docv の版にする
- `docv_version` が docv の版より新しければ、docv を更新する
