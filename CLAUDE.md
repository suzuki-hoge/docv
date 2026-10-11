# docv

ドキュメント管理のフレームワーク docv（仮の名前）を開発するリポジトリ。全体像とドキュメントの一覧は [README.md](README.md)。

- 応答、ドキュメント、コードのコメントは日本語で書く
- このリポジトリのドキュメントは docv 自身で管理する（ドッグフーディング）。設定は [docv.yaml](docv.yaml)

---

## 1. ドキュメントを書くとき

- [docs/spec/writing.md](docs/spec/writing.md)（docv のルール）と [docs/rule/writing-style.md](docs/rule/writing-style.md)（このリポジトリの書式）に従う
- 設計の正本は docs/spec/。実装の前に spec を読み、spec と違う実装が必要になったら、実装より先に人間に確認して spec を直す
- 決めたことが変わったら spec を書き換え、捨てた案は理由を付けて docs/archive/ に移す（[docs/spec/retire.md](docs/spec/retire.md) の形）。やったことは書かない
- 決まっていないことは [docs/fixme/open.md](docs/fixme/open.md) に足し、決まったら spec に書いて消す
- タスクは docs/plan/ に書き、終わったものは消す
- docv check を実行できるようになるまでは、[docs/spec/check.md](docs/spec/check.md) のルールを目で確かめる。実行できるようになったら、ドキュメントを変えるたびに check を通す
- README.md のドキュメントの一覧は、docs/ にファイルを足したり消したりしたら直す

---

## 2. コードを書くとき

- Rust で書く。配布の前提は [docs/spec/distribution.md](docs/spec/distribution.md)（単一のバイナリ、テンプレ・ルールファイル・CSS と JS は埋め込む）
- コミットの前に次が通ること

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

- build の出力は、入力が同じなら毎回同じにする（生成日時・実行したマシンのパス・乱数を入れない、並び順を固定する）。[docs/spec/build.md](docs/spec/build.md) の 4 章
- ファイルを書くコマンドは、[docs/spec/cli.md](docs/spec/cli.md) の 3 章に書いた範囲の外に書かない
- templates/、schema/、assets/ は製品の一部（バイナリへの埋め込みと公開の対象）。変えるときは spec と揃える

---

## 3. 試すとき

- ほかのリポジトリには書き込まない
- docv.yaml は tmp/ に置き、`--config`、`--root`、`--out` を使って、ほかのリポジトリのファイルを読むだけにする
- ルートの tmp/ は作業用。コミットしない
- まとまっていない意思決定やメモは docs/tmp/ に置く（docv の tmp。[docs/spec/kind.md](docs/spec/kind.md) の 5 章）

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
