# docv

ドキュメント管理のフレームワーク docv を開発するリポジトリ。全体像とドキュメントの一覧は [`docv/README.md`](docv/README.md)。

- 応答、ドキュメント、コードのコメントは日本語で書く
- このリポジトリのドキュメントは docv 自身で管理する（ドッグフーディング）。設定は [`docv.yaml`](docv.yaml)

---

## 1. ドキュメントを書くとき

- [`docv/slot/guide/doc-guide.md`](docv/slot/guide/doc-guide.md)（このリポジトリの Doc ガイド）に従う。どこに何を書くか、書き方、書式は、ここに書く

---

## 2. コードを書くとき

- 実装の前に Slot と spec を読む。違う実装が必要になったら、実装より先に人間に確認して Slot か spec を直す
- Rust で書く。単一のバイナリにし、テンプレ・スキーマ・docv の概要・CSS は埋め込む（[`docv/slot/system/system.md`](docv/slot/system/system.md) の「バイナリに埋め込むもの」）
- コミットの前に `make fix` が通ること。手順は [`docv/slot/guide/develop.md`](docv/slot/guide/develop.md)（開発コマンド）

- build の出力は、入力が同じなら毎回同じにする（生成日時・実行したマシンのパス・乱数を入れない、並び順を固定する）。[`docv/slot/service/service/html.md`](docv/slot/service/service/html.md) の「毎回同じ出力にする決まり」
- ファイルを書くコマンドは、[`docv/slot/service/service/cli.md`](docv/slot/service/service/cli.md) の「ファイルを書くコマンド」に書いた範囲の外に書かない
- `assets/` は製品の一部（バイナリへの埋め込みと公開の対象）。実装から参照して出力するものは、すべて `assets/` に置く。変えるときは Slot と spec と揃える
- `assets/schema/` のスキーマを変えたら、`docv/free/schema/` の写しも直す。揃っていないと cargo test が落ちる
- 版は変えない。`Cargo.toml` の `version`、`docv.yaml` の `docv_version`、設定ファイルのページの例の版、タグには触らない。版上げは、人間が `make new-version` で行う（[`docv/slot/operation/release.md`](docv/slot/operation/release.md)）
  - 頼まれたら、`assets/update-guide.yaml` の先頭にその版の変更点と対応を書くところまでを行う。コミットはしない
  - `make new-version` は実行しない

---

## 3. 試すとき

- ほかのリポジトリには書き込まない
- ルートの `tmp/` は作業用。コミットしない
- 予定、まとまっていない意思決定、メモは `docv/free/plan/` に置く（`docv.yaml` の `docv_exclude` に入れてあり、docv は扱わない）

---

## 4. git

- 常に main で作業し、main にコミットしてよい。ブランチは切らない
- 1 つのコミットは、1 つの要件を実現する単位にする。複数の要件の変更を 1 つのコミットに混ぜない
- すでにコミットした要件への修正は、新しいコミットにせず、その要件のコミットに amend する
- push は人間が行う

---

## 5. 人間が行うこと

- 版上げ（`make new-version V=x.y.z`。コミット、タグ、push までを行う）
- リポジトリの公開設定の変更
