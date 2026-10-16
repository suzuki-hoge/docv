---
group: コマンド
date: 2026-10-16
---

# コマンドを init、desc、check、fix、build、slot に組み直す

## 背景

HTML と JPG を作るコマンドが build-html と build-image に分かれていた。Slot を作る add と、スキーマを出す schema は、Slot と `docv.yaml` の両方を扱い、名前と中身が合っていなかった。`docv.yaml` の項目は、JSON Schema を読まないと分からなかった。

## 決めたこと

- `docv build` で HTML を作る。`output.image` が true のときだけ、続けて Slot のページを JPG にする
- Slot を扱うコマンドは `docv slot` の下に置く
  - `slot list` は、Slot の一覧と問いを出す
  - `slot gen <slot> {--print | --force}` は、Slot ファイルを作る
  - `slot schema <slot>` は、YAML の Slot のスキーマを出す
- md の Slot に `slot gen` を実行したら、空のファイルを作り、中身の形は自由で、問いに答える Markdown を書けばよいと表示する
- `docv init {--print | --force}` は、すべての項目を既定の値でコメントにした `docv.yaml` を書く。`docv schema` はなくす
- コマンドは init、desc、check、fix、build、slot の順に並べる

## 結果

`docv --help` の上から順に、導入から日々の作業まで辿れる。設定の項目は `docv init --print` で、Slot の問いは `docv slot list` で分かる。JPG を作らないプロジェクトは、Chrome がなくても build できる。

## 選ばなかった案

| 案                                                   | 選ばなかった理由                                               |
| ---------------------------------------------------- | -------------------------------------------------------------- |
| build で常に JPG を作る                              | JPG を使わないプロジェクトにも Chrome が要る                   |
| `slot-list` のように、ハイフンでつないだ名前にする   | `docv slot --help` で、Slot のコマンドだけをまとめて見られない |
| `docv init --print` で、init が書く 3 項目だけを出す | `lint`、`format`、`output` などの項目が見えない                |
