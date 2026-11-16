---
mission: 部品ごとの振る舞いと、部品の間の流れ
---

# システム仕様

## 処理の流れ

1. `docv.yaml` を読み、スキーマで確かめる。init、desc、update-guide、slot list、slot schema と、`--print` を付けた slot gen は、`docv.yaml` を読まない
2. ルート以下のファイルを分類し、Slot、ADR、PDR、Free ファイルを読む
3. コマンドに応じて、次の部品を使う

| コマンド  | 使う部品                                                                                                                                                            |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| check     | 埋め込んだスキーマと決まりで確かめる。`lint.textlint` が enable なら、npx で textlint を呼ぶ                                                                        |
| fix       | `lint.textlint` が enable なら、textlint に自動修正させる。続けて、埋め込んだ整形の道具で整形し、書き戻す                                                           |
| build     | HTML を `dir.output` の下の `html/` に書き、インデックスを `document_root` の `README.md` に書く。そのあと、`generate.image` が true なら、Chrome に JPG を撮らせる |
| diff-open | git で `--from` のリビジョンと作業ツリーの差分を取り、印を付けた HTML を OS のキャッシュのディレクトリに書いて、OS のコマンドで開く                                 |

## バイナリに埋め込むもの

Rust で書いたバイナリに、次のものを埋め込む。

- Slot のテンプレ（`assets/template/`。YAML の Slot の項目のサンプルと、ドキュメントのガイドラインの雛形）
- YAML の Slot と `docv.yaml` のスキーマ（`assets/schema/`）
- docv の概要（`docv desc` が出す `assets/desc.md`）。サービス仕様の 1 ページ目（`docv/slot/service/service/desc.md`）は、このファイルへのシンボリックリンク
- 版ごとの変更点と対応（`docv update-guide` が出す `assets/update-guide.yaml`）。手元の docv の版と、出す内容を一致させるため
- ドキュメントをレビューする手順（`docv review-guide` が出す `assets/review-guide.md`）
- HTML の CSS（`assets/docv.css`）と JS（`assets/docv.js`）。build が、出力先の直下にそのまま書く
- Markdown の整形に使う dprint-plugin-markdown
- コードブロックの色付けに使う syntect と、その構文の定義とテーマ

textlint を使うときだけ Node.js が、JPG を作るときだけ Chrome が、diff-open を使うときだけ git が要る。どれもバイナリには入れない。

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

## git とのやりとり

diff-open は、docv ルートで git のコマンドを呼び、`--from` のリビジョンと作業ツリーを比べる。

- `git diff` で、変わったファイルごとに、新規、変更、削除のどれかと、足した行数と消した行数を取る
  - 名前を変えたファイルは、消したファイルと新しいファイルとして扱う
  - 行数がどちらも 0 のファイル（モードだけの変更）は、変わっていないとみなす
  - 画像などのバイナリは、行数を取れないので、行数のない変更として扱う
- まだ git に追加していないファイルは、`git ls-files` で取り、新規として扱う。行数は、ファイルの行を数える
- 消したファイルは、`git show` で `--from` での中身を読み、題や ADR と PDR の `group` を出す
- 書き換えた YAML のファイルは、`git show` で `--from` での中身を読み、docv の版を伏せて今の中身と比べる
  - 伏せるのは、docv check が版を確かめる行（1 行目のスキーマの URL の版と、`docv_version` の値）だけ
  - 伏せて同じなら、変わっていないとみなす。違えば、版だけが違う行を、足した行数と消した行数から引く
- 比べるのは `--from` のコミットそのもので、ブランチの分かれ目（merge-base）ではない
- git が見つからないとき、ルートが git の管理下にないとき、`--from` がコミットを指さないときは、error で終わる
