# docv

プロジェクトのドキュメントを、AI Agent が読み書きする Markdown（Free ファイル）と、人間が全体像を掴む HTML の 2 つの形で扱うためのフレームワーク。ドキュメントのルールと、そのルールを前提にした CLI からなる。

```
Slot ファイル（YAML、docv/）     ── Slot ごとのページ ───▶ HTML（と、その JPG）
Free ファイル（Markdown、docs/） ── ミッションとタグのツリー ──▶ プロジェクト詳細.html
                                 └─ mission ─────────────▶ AI Agent
```

- インストール、サービス仕様、利用マニュアルのように、どのプロジェクトにも要るドキュメントは「Slot」に、docv が決めたミッションと形で書く。HTML には、規格の順にすべての Slot が並び、置いていない Slot はグレーになる
- Free ファイルは、ファイルごとにミッション（読むと何が分かるか）を宣言し、その範囲を超えて書き足さない。分類はタグで行い、タグにルールを付けられる
- 図や API の定義は外の道具で作り、Slot からリンクするか埋め込む
- HTML は、CLI が LLM を使わずに作る。同じ入力からは毎回同じものができる。すべてローカルで完結する

## 使い方

```sh
docv init                          # docv.yaml を作る
docv desc                          # docv の概要を出す
docv check                         # ルールに沿っているかを確かめる
docv fix                           # 空行や表の余白を整形し、textlint で直せるところを直す
docv build                         # HTML を作る（dir.image を書いていれば JPG も）
docv open                          # 作った HTML をブラウザで開く
docv slot list                     # Slot の一覧と、ミッションを出す
docv slot gen service              # Slot を決まったパスに作る
docv slot gen doc-guide            # ドキュメントのガイドラインの雛形を作る
docv slot schema install           # YAML の Slot のスキーマを出す
```

AI Agent には、`CLAUDE.md` などに次のように書いて伝える。

```
- セッションの最初に `docv desc` を実行し、docv の使い方を把握すること
- ドキュメントを書く前に、`docv/guide/doc-guide.md`（ガイドライン）を読むこと
```

このリポジトリのサービス仕様、利用マニュアル、開発者向けガイドなどは、Slot（[`docv/`](docv)）に書いてある。`docv build` で HTML（`output/docv/html/`）にして見られる。

## ドキュメント

### docv の規格

サービス仕様（[`docv/service/service/`](docv/service/service)）のページに書く。1 ページ目は `docv desc` と同じ `assets/desc.md`。

- [`slot.md`](docv/service/service/slot.md) — Slot の一覧と、それぞれのファイルに書くことと形
- [`free.md`](docv/service/service/free.md) — Free ファイルの YAML front matter とタグの形と、check が Free ファイルで確かめること
- [`adr.md`](docv/service/service/adr.md) — ADR に書くことと形
- [`config.md`](docv/service/service/config.md) — `docv.yaml` に書くことと、各項目の解釈、check と fix で使う道具の設定
- [`cli.md`](docv/service/service/cli.md) — コマンドが読み書きするファイルと、出力先、終了コード、メッセージの言語
- [`html.md`](docv/service/service/html.md) — docv build が作る HTML と JPG の作り（README、Slot、ADR、現在の設定のページを含む）と、毎回同じ出力にする決まり
- [`docs/schema/`](docs/schema) — `docv.yaml` と YAML の Slot の JSON Schema の写し（`assets/schema/` のファイルと同じもの）

### これからと経緯

- `docs/plan/` — やると決めたタスクと、まだ決まっていないこと（終わったら、決まったら消すので、ないこともある）
- [`docv/adr/`](docv/adr) — 決めた経緯（ADR）
