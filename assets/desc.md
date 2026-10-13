# docv

docv の解決したいこと

- AI Agent の開発速度に人間の理解がついていけるように、ドキュメントの構造化ルールを提供し人間用のビジュアライズを行う
- AI Agent が無秩序にドキュメントを肥大化させないように、薄いガイドラインを提供する

docv の提供するもの

- およそ普遍的に必要になるドキュメントのテンプレート
- 自由に記載する Markdown ファイルの YAML front matter
- それらのビジュアライズ HTML 生成
- AI Agent に伝えるドキュメント管理のガイドライン

このツールそのものには LLM は含まない

## 1. コンセプト

### `docv/`

- プロジェクトの `docv/` ディレクトリに docv 指定のファイルを配置するとビジュアライズされ HTML で閲覧できる
- 環境構築や用語集などそれぞれのページを Slot といい、Slot ごとに所定の書式がある
- HTML に加え JPG 画像を出力することも可能
    - GitHub で直接ドキュメントを閲覧できるようにするため
- `docv/` は `docv.yaml` で変更可能

### `docs/`

- `docv/` 以外のドキュメントは `docs/` で自由に Markdown で管理する
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

新たに導入する場合は `docv init` し、`docv desc` に従い `docv add` と `docv check` で作成・調整を進める

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
