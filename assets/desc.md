# docv

docv の解決したいこと

- AI Agent の開発速度に人間の理解がついていけるように、ドキュメントの構造化ルールを提供し人間用のビジュアライズを行う
- AI Agent が無秩序にドキュメントを肥大化させないように、薄いガイドラインを提供する

docv の提供するもの

- およそ普遍的に必要になるドキュメントのテンプレート
- Free ファイル（`docs/` の Markdown）の YAML front matter
- それらのビジュアライズ HTML 生成
- AI Agent に伝えるドキュメント管理のガイドラインの雛形

このツールそのものには LLM は含まない

## 1. コンセプト

### `docv/`

- プロジェクトの `docv/` ディレクトリに docv 指定のファイルを配置するとビジュアライズされ HTML で閲覧できる
- 環境構築や用語集などそれぞれのページを Slot といい、Slot ごとに所定の書式がある
- HTML に加え JPG 画像を出力することも可能
    - GitHub で直接ドキュメントを閲覧できるようにするため
- `docv/` は `docv.yaml` で変更可能

### `docs/`

- `docv/` 以外のドキュメントは、Free ファイルとして `docs/` に Markdown で置く
- ビジュアライズ時は、ファイルは通常の Markdown → HTML 変換のほか、リンク切れチェックや textlint / format ( 設定次第 ) も行われる
- YAML front matter でタグなどをつけ管理する
- `docs/` は `docv.yaml` で変更可能

## 2. Slot

`docv add --list` で確認でき、作成したい場合は `docv add <slot>` で作成する

## 3. YAML front matter

`docs/` の Markdown ファイルを管理するための YAML front matter

```yaml
---
mission: 注文の処理は、どの注文をいつ確定し、失敗したときにどう扱うか
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

## 4. コマンド

主要ユースケースのみここで紹介する

すべてのコマンドは `docv --help` で確認

- `docv init`: `docv.yaml` を作る
- `docv add --list`: Slot 一覧を見る
- `docv add <slot>`: Slot ファイルを作る
- `docv schema <slot>`: Slot ファイルのスキーマを確認する
- `docv {check|fix} [path...]`: ドキュメントを検査する / 自動修正する

新たに導入する場合は `docv init` し、`docv add doc-guide` でガイドラインを作り、`docv desc` に従い `docv add` と `docv check` で作成・調整を進める

## 5. スキーマ ( 設定ファイル )

設定ファイル ( `docv.yaml` ) のスキーマは `docv schema` で確認する

## 6. スキーマ ( docv ファイル )

Slot ファイルのうち、次のページは YAML である

- `install`: インストール
- `words`: 用語集
- `stack`: 技術スタック
- `api-yaml`: API 仕様書（md でも置ける）
- `db-yaml`: DB 仕様書（md でも置ける）
- `directory-tree`: ディレクトリ
- `setup`: 環境構築
- `env`: 環境変数
- `develop`: 開発コマンド
- `cost`: 費用

これらのスキーマは `docv schema <slot>` で確認する

## 7. ルール、ガイドライン、設定

docv の決まりは、次の 3 つに分かれる

- ルール: docv の仕組みが決める決まり。Slot と `docv.yaml` のスキーマ、YAML front matter の key、check が確かめることなど。利用側は変えられない。この概要と `docv schema <slot>` で確認する
- ガイドライン: ドキュメントを書くときの方針。`docv add doc-guide` で Doc ガイド（`docv/guide/doc-guide.md`）に雛形を作り、プロジェクトに合わせて自由に直す
- 設定: `docv.yaml` で切り替えること。check のルールや textlint を使うか、出力を git で管理するかなど。`docv schema` で確認する
