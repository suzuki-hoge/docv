# docv

ドキュメント管理のフレームワーク docv（仮の名前）を開発するリポジトリ。全体像とドキュメントの一覧は [README.md](README.md)。

- 応答、ドキュメント、コードのコメントは日本語で書く
- このリポジトリのドキュメントは docv 自身で管理する（ドッグフーディング）。設定は [docv.yaml](docv.yaml)

---

## 1. ドキュメントを書くとき

- [docs/rule/docs/guideline.md](docs/rule/docs/guideline.md)（docv のルール）と [docs/rule/writing-style.md](docs/rule/writing-style.md)（このリポジトリの書式）に従う
- docv の設計は枠（docv/）に書く。サービス仕様、システム仕様、コマンドの挙動（利用マニュアル）などの枠の問いに答える内容は枠に、どの枠にも収まらない規格の詳細だけを docs/ に書く（枠、ADR、docv.yaml は docs/spec/、原本の書き方は docs/rule/docs/guideline.md）。同じことを 2 か所に書かない。docv/ には、docv に当てはまる枠だけを置く。docs/ はプロジェクトが自由に書くファイル、docv/ は docv の規格のファイル。実装の前に枠と spec を読み、違う実装が必要になったら、実装より先に人間に確認して枠か spec を直す
- 決めたことが変わったら枠か spec を書き換え、決めた経緯と選ばなかった案は ADR（docv/adr/。[docs/spec/adr.md](docs/spec/adr.md) の形）に残す。やったことは書かない
- 決まっていないことは [docs/fixme/open.md](docs/fixme/open.md) に足し、決まったら枠か spec に書いて消す
- タスクは docs/plan/ に書き、終わったものは消す。docs/plan/ は git で管理するが、docv.yaml の ignore に入れてあり、docv は扱わない（HTML に出さない）
- ドキュメントを変えるたびに docv fix と docv check を通す
- README.md のドキュメントの一覧は、docs/ にファイルを足したり消したりしたら直す

---

## 2. コードを書くとき

- Rust で書く。単一のバイナリにし、テンプレ・ルールファイル・CSS は埋め込む（[docv/system/system.md](docv/system/system.md) の「バイナリに埋め込むもの」）
- コミットの前に次が通ること

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

- build の出力は、入力が同じなら毎回同じにする（生成日時・実行したマシンのパス・乱数を入れない、並び順を固定する）。[docv/system/system.md](docv/system/system.md) の「毎回同じ出力にする決まり」
- ファイルを書くコマンドは、[docv/system/system.md](docv/system/system.md) の「ファイルを書くコマンド」に書いた範囲の外に書かない
- templates/、schema/、assets/ は製品の一部（バイナリへの埋め込みと公開の対象）。変えるときは枠と spec と揃える

---

## 3. 試すとき

- ほかのリポジトリには書き込まない
- ルートの tmp/ は作業用。コミットしない
- まとまっていない意思決定やメモは docs/tmp/ に置く（docv.yaml の ignore に入れてあり、docv は扱わない）

---

## 4. git

- 常に main で作業し、main にコミットしてよい。ブランチは切らない
- 1 つのコミットは、1 つの要件を実現する単位にする。複数の要件の変更を 1 つのコミットに混ぜない
- すでにコミットした要件への修正は、新しいコミットにせず、その要件のコミットに amend する
- push は人間が行う

---

## 5. 人間が行うこと

- リリースのタグの push
- リポジトリの公開設定の変更
