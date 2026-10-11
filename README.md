# docv

プロジェクトのドキュメントを、AI Agent が読み書きする Markdown と、人間が見る HTML の 2 つの形で扱うためのフレームワーク。ドキュメントのルールと、そのルールを前提にした CLI からなる。

```
詳細ドキュメント（Markdown + frontmatter）  ← 唯一のデータソース
   ├─ AI Agent: docv index でファイルの一覧と問いを知り、必要なファイルだけ読む
   └─ 人間: docv build が生成する HTML（任意で JPG）を見る
```

- ドキュメントを時間で 未来 / 現在 / 過去 に分ける。現在のドキュメントには最新の状態だけを書き、経緯は過去のドキュメントへ移す
- ファイルごとに答える問いを宣言し、その範囲を超えて書き足さない
- インデックスと HTML は、CLI が LLM を使わずに作る。同じ入力からは毎回同じものができる
- docv.yaml で指定したファイルだけを管理する。ルールに沿わないファイルも置ける
- すべてローカルで完結する

## 使い方

```sh
docv init            # docv.yaml を作る
docv migrate-guide   # 既存のドキュメントのうち、管理外のものとルールに沿っていないものを見る
docv fmt             # 空行や表の余白を整形する
docv check           # ルールに沿っているかを確かめる
docv build           # HTML を作る
```

AI Agent には、CLAUDE.md などに次のように書いて伝える。

```
- セッションの最初に `docv desc` と `docv index` を実行し、ドキュメントのルールと一覧を把握すること
```

## ドキュメント

### 考え方とルール

- [docs/spec/concept.md](docs/spec/concept.md) — docv は何を解決し、どんな原則でルールと CLI を決めているか
- [docs/spec/kind.md](docs/spec/kind.md) — 組み込みの種類にはどんなものがあり、それぞれ何をどんな大きさで書くか
- [docs/spec/frontmatter.md](docs/spec/frontmatter.md) — 管理対象のファイルの frontmatter には何を書くか
- [docs/spec/writing.md](docs/spec/writing.md) — docv で管理するドキュメントを、人間と AI Agent はどう読み書きするか
- [docs/spec/config.md](docs/spec/config.md) — docv.yaml には何を書き、各項目は何を意味するか

### CLI

- [docs/spec/cli.md](docs/spec/cli.md) — docv の CLI にはどんなコマンドがあり、共通してどう動くか
- [docs/spec/check.md](docs/spec/check.md) — docv check は何を確かめ、どう報告するか
- [docs/spec/retire.md](docs/spec/retire.md) — docv retire は何を、どこへ、どんな形で移すか
- [docs/spec/style.md](docs/spec/style.md) — docv check は、Markdown の書き方のどんな規則を静的に確かめられるか
- [docs/spec/textlint.md](docs/spec/textlint.md) — docv check は textlint をどう使い、利用者はルールをどこまで決められるか
- [docs/spec/fmt.md](docs/spec/fmt.md) — docv fmt は Markdown をどう整形し、エディタの自動整形とどう付き合うか
- [docs/spec/build.md](docs/spec/build.md) — docv build は何を生成し、その出力をどう扱うか
- [docs/spec/migrate-guide.md](docs/spec/migrate-guide.md) — 既存のプロジェクトに後から docv を入れるとき、docv migrate-guide は何を示すか
- [docs/spec/distribution.md](docs/spec/distribution.md) — docv は何で作られ、どう配られ、利用者はどう入れるか

### このリポジトリの規約

- [docs/rule/writing-style.md](docs/rule/writing-style.md) — このリポジトリのドキュメントは、どんな書式で書くか

### 未来と過去

- [docs/fixme/open.md](docs/fixme/open.md) — docv の設計で、まだ決まっていないことは何か
- [docs/plan/](docs/plan) — やると決めたタスク
- [docs/archive/](docs/archive) — 外した内容と、外した理由
