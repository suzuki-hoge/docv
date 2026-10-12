# docv

プロジェクトのドキュメントを、AI Agent が読み書きする Markdown（原本）と、人間が全体像を掴む HTML の 2 つの形で扱うためのフレームワーク。ドキュメントのルールと、そのルールを前提にした CLI からなる。

```
枠（YAML、docv/）      ── 枠ごとのページ ──────▶ HTML（と、その JPG）
原本（Markdown、docs/） ── 問いとタグのツリー ──▶ プロジェクト詳細.html
                        └─ mission ─────────────▶ AI Agent
```

- インストール、サービス仕様、利用マニュアルのように、どのプロジェクトにもある問いは「枠」に、決まった形で書く。HTML には、規格の順にすべての枠が並び、置いていない枠はグレーになる
- 原本は、ファイルごとに答える問いを宣言し、その範囲を超えて書き足さない。分類はタグで行い、タグにルールを付けられる
- 図や API の定義は外の道具で作り、枠からリンクするか埋め込む
- HTML は、CLI が LLM を使わずに作る。同じ入力からは毎回同じものができる。すべてローカルで完結する

## 使い方

```sh
docv init                          # docv.yaml を作る
docv migrate-guide                 # 既存のドキュメントに、タグと枠の案と、直すところを出す
docv add --list                    # 作れる枠の一覧を出す
docv add service                   # 枠を決まったパスに作る
docv fix                           # 空行や表の余白を整形し、textlint で直せるところを直す
docv check                         # ルールに沿っているかを確かめる
docv build-html                    # HTML を作る
docv build-image                   # 枠のページを JPG にする
```

AI Agent には、CLAUDE.md などに次のように書いて伝える。

```
- セッションの最初に `docv desc` を実行し、ドキュメントのルールを把握すること
```

このリポジトリのサービス仕様、利用マニュアル、開発者向けガイドなどは、枠（[docv/](docv)）に書いてある。`docv build-html` で HTML（tmp/html/）にして見られる。

## ドキュメント

### docv の規格

- [docs/rule/docs/guideline.md](docs/rule/docs/guideline.md) — 原本には何をどう書き、frontmatter とタグをどう付け、check は何を確かめるか
- [docs/spec/slot.md](docs/spec/slot.md) — 枠にはどんなものがあり、それぞれのファイルに何をどんな形で書き、HTML でどう見せるか
- [docs/spec/adr.md](docs/spec/adr.md) — ADR には何をどんな形で書き、HTML でどう見せるか
- [docs/spec/config.md](docs/spec/config.md) — docv.yaml には何を書き、各項目はどう解釈され、check と fix の道具をどう使うか

### このリポジトリの規約

- [docs/rule/writing-style.md](docs/rule/writing-style.md) — このリポジトリのドキュメントは、どんな書式で書くか

### これからと経緯

- [docs/fixme/open.md](docs/fixme/open.md) — docv の設計で、まだ決まっていないことは何か
- [docs/plan/](docs/plan) — やると決めたタスク
- [docv/adr/](docv/adr) — 決めた経緯（ADR）
