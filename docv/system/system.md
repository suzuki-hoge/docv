# システム仕様

## 処理の流れ

1. カレントディレクトリをルートにし、そこにある `docv.yaml` を読む。親のディレクトリは探さない
2. `docv.yaml` を読んでスキーマで確かめ、ルート以下のファイルを分類する
3. コマンドに応じて、検査、出力、書き換えのどれかに進む

結果は標準出力に、エラーは標準エラーに出す。

## ファイルを書くコマンド

| コマンド    | 書くもの                                                                                                                 |
| ----------- | ------------------------------------------------------------------------------------------------------------------------ |
| init        | `docv.yaml` だけ                                                                                                         |
| add         | 指定した Slot ファイルだけ                                                                                               |
| fix         | 管理対象のファイルと、`slots.dir` の中の Markdown（md の Slot と ADR）だけ。整形と textlint の自動修正以外の変更はしない |
| build-html  | `output.dir` の中だけ。`jpg/` には触らない                                                                               |
| build-image | `output.dir` の `jpg/` と目印だけ。撮る HTML は OS の一時ディレクトリに作り、撮り終えたら消す                            |

- それ以外のコマンドはファイルを書かない
- `CLAUDE.md`、リポジトリのルートの `.gitignore`、hook の設定には、どのコマンドも触らない
- build-html と build-image は、出力先に `.docv`（docv が作った目印）がなく、空でもないときは、何も書かずにエラーで終わる。`output.dir` を間違えて既存のディレクトリを指しても消さないため

## ファイルの分類

| 分類     | 条件                                                                        |
| -------- | --------------------------------------------------------------------------- |
| 除外     | `.gitignore` に書かれている、`output.dir` の中にある、`ignore` に当てはまる |
| 管理対象 | `docs` に当てはまる Markdown                                                |
| 管理外   | それ以外の Markdown。中身は見ない                                           |

Slot は、Markdown とは別に、`slots.dir`（既定は `docv/`）から `<まとまり>/<ID>.<拡張子>` の名前のファイルだけを読む。拡張子は Slot ごとに決まる（YAML か md）。知らない項目があれば、読めない Slot として check が error にする。

`slots.dir` の中でも、除外に当たるファイルと、名前がドットで始まるファイルとディレクトリ（`.DS_Store` など）は読まない。`output.dir` の既定の `docv/build` は `slots.dir` の中にあるが、出力は Slot として読まない。それ以外で Slot の名前でないファイルは、check が slot-unknown の error にする。

`slots.dir` の中の Markdown（md の Slot と ADR）には、Free ファイルのルール（YAML front matter、書式の規則など）はかけない。整形と textlint だけを、管理対象のファイルと同じくかける。

## ソースの構成

| 部品     | ファイル                                                                                           |
| -------- | -------------------------------------------------------------------------------------------------- |
| 入口     | `src/main.rs`、`src/cli.rs`                                                                        |
| 読み込み | `src/workspace.rs`、`src/config.rs`、`src/markdown.rs`、`src/meta.rs`、`src/slot.rs`、`src/adr.rs` |
| 検査     | `src/check.rs`、`src/style.rs`、`src/textlint.rs`、`src/diagnostic.rs`                             |
| 出力     | `src/build.rs`、`src/jpg.rs`、`src/desc.rs`、`src/schema.rs`                                       |
| 書き換え | `src/fix.rs`、`src/format.rs`、`src/add.rs`、`src/init.rs`                                         |

## バイナリに埋め込むもの

Rust で書いた単一のバイナリにし、次のものを埋め込む。利用者に Rust の環境はいらない。

- YAML の Slot のテンプレ（`templates/`）
- YAML の Slot と `docv.yaml` のスキーマ（`assets/schema/`、`schema/docv.schema.json`）
- docv の概要（`docv desc` が出す `assets/desc.md`。`docv/service/service.md` はこのファイルへのシンボリックリンク）
- HTML の CSS（`assets/style.css`）
- Markdown の整形に使う dprint-plugin-markdown
- コードブロックの色付けに使う syntect と、その構文の定義とテーマ

textlint を使うときだけ Node.js が、JPG を作るときだけ Chrome が要る。`docv.yaml` の JSON Schema（`schema/docv.schema.json`）は、埋め込むほかにリポジトリでも公開し、`docv.yaml` からは版を含む URL で参照する。

## HTML の作り

- CSS は各ページの中に書く。外部の CDN は使わない。file:// で開いても、そのままホスティングしても見られる
- JS は、決定のページのカーソルキーでの移動と、プロジェクト詳細のタグのトグルにだけ使い、ページの中に書く。どちらも JPG にしないページで、JS が動かなくても、すべての中身は読め、リンクから辿れる。OS ごとの手順のタブは、JS を使わず CSS で切り替える
- フェンスのコードブロックは、フェンスに書いた言語（`yaml` など）に合わせて、build のときに色を付ける。色は HTML の中に埋め込み、JS は使わない。言語がないか、syntect が知らない言語なら、色を付けずに出す
- リンクは、Free ファイル、Slot ファイル、ADR のページにだけ、出力先の中の相対パスで張る（ツリー、Slot ファイルや Free ファイルの本文の中のリンク）。file:// で開けば file:// のまま、ホスティングすればその中で辿れる。HTML は clone したリポジトリで見るので、ほかのファイル（コード、管理外のファイルなど）はエディタで開けばよく、リンクにせず文字だけを出す
- 例外は 2 つ。画像は、元のファイルを出力先から辿れる相対パスで読む。API 仕様書と DB 仕様書の `ref` の先（ページを作らない Slot）は、`index.html` のカードから相対パスでリンクする

## 毎回同じ出力にする決まり

- 生成日時、実行したマシンのパス、乱数を出力に入れない
- 並び順を固定する。Slot は規格の順、Free ファイルはディレクトリのツリーの順、Slot の中の項目は書いた順、ADR は日付と id の順
- 入力（管理対象のファイル、Slot、ADR、`docv.yaml`、docv の版）が同じなら、HTML はバイト単位で同じになる
- JPG は、同じマシン・同じ Chrome・同じフォントなら同じになる

## JPG の撮り方

ローカルの Chrome をヘッドレスで起動し、DevTools Protocol で HTML を開いて撮る。幅は 1280 CSS ピクセル、画質は 80。印刷の表示にし、ページの高さに画面を合わせて、Slot のページを 1 枚ずつ撮る。Chrome は、環境変数 DOCV_CHROME、OS ごとの標準のインストール先、PATH にある chrome と chromium の順に探し、見つからなければ error で終わる。

## 終了コード

| コード | 意味                                                                 |
| ------ | -------------------------------------------------------------------- |
| 0      | 成功。check で warn だけが出たときも含む                             |
| 1      | check で error が 1 件以上ある。fix で直せないファイルがある         |
| 2      | `docv.yaml` がスキーマに合わない、引数が正しくない、など実行できない |

## 言語

CLI のメッセージ、ヘルプ、docv の概要、テンプレは、日本語だけで書く。切り替えはできない。英語に訳した版を持つと、同じことを 2 か所で揃え続けることになるため。
