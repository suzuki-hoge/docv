---
mission: 部品ごとの振る舞いと、部品の間の流れと、部品をまたいで守る決まり
---

# システム仕様

部品の全体像は、[システム概要](system-overview.md)。コマンドから見た振る舞いは、サービス仕様に書く。読み書きするファイルと終了コードは [コマンド](../service/service/cli.md)、HTML と JPG は [ビジュアライズ](../service/service/html.md) のページ。

## 処理の流れ

1. `docv.yaml` を読み、スキーマで確かめる。init、desc、slot list、slot schema と、`--print` を付けた slot gen は、`docv.yaml` を読まない
2. ルート以下のファイルを分類し、Slot、ADR、Free ファイルを読む
3. コマンドに応じて、次の部品を使う

| コマンド | 使う部品                                                                                                                                |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| check    | 埋め込んだスキーマと決まりで確かめる。`lint.textlint` が enable なら、npx で textlint を呼ぶ                                            |
| fix      | `lint.textlint` が enable なら、textlint に自動修正させる。続けて、埋め込んだ整形の道具で整形し、書き戻す                               |
| build    | HTML を `dir.html` に書き、`dir.index` を書いていればインデックスも書く。そのあと、`dir.image` を書いていれば、Chrome に JPG を撮らせる |

## バイナリに埋め込むもの

Rust で書いた単一のバイナリにし、次のものを埋め込む。利用者に Rust の環境はいらない。

- Slot のテンプレ（`assets/template/`。YAML の Slot の項目のサンプルと、ドキュメントのガイドラインの雛形）
- YAML の Slot と `docv.yaml` のスキーマ（`assets/schema/`）
- docv の概要（`docv desc` が出す `assets/desc.md`）。サービス仕様の 1 ページ目（`docv/service/service/desc.md`）は、このファイルへのシンボリックリンク
- HTML の CSS（`assets/style.css`）
- 検索のページの JS（`assets/search.js`）
- Markdown の整形に使う dprint-plugin-markdown
- コードブロックの色付けに使う syntect と、その構文の定義とテーマ

textlint を使うときだけ Node.js が、JPG を作るときだけ Chrome が要る。どちらもバイナリには入れない。

## スキーマの公開

`docv.yaml` と YAML の Slot の JSON Schema（`assets/schema/`）は、埋め込むほかにリポジトリでも公開する。`docv init` と `docv slot gen` は、ファイルの 1 行目に、版のタグを含む URL を書いて参照させる。新しい版の docv が書く URL は、その版のタグを push するまで開けない。

## textlint とのやりとり

check と fix は、`lint.textlint` が enable のときだけ textlint を動かす。textlint と 2 つのプリセット（ja-technical-writing、ja-spacing）は、npx で取ってくる。それぞれの版は、docv の版ごとに決める。

- 設定ファイルは docv が作り、OS のキャッシュのディレクトリに書く。プリセットの既定に頼らず、使わないルールも含めて、すべての値を書く
- YAML の Slot の Markdown の項目は、OS のキャッシュのディレクトリに一時的なファイルとして書いて渡し、使い終わったら消す
- リポジトリには、ファイルを作らない
- 終了コード 1 は指摘として読み、それより大きい終了コードは失敗として扱う

## Chrome とのやりとり

build は、ローカルの Chrome をヘッドレスで起動し、DevTools Protocol で出力先の HTML を開いて撮る。印刷の表示にし、ページの高さに画面を合わせて、Slot のページを 1 枚ずつ撮る。

Chrome が見つからないときや、DevTools に接続できないときは、error で終わる。HTML は、それまでに書き終えている。
