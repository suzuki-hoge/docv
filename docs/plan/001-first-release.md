---
question: docv の最初の版を出すまでに、何をどの順でやるか
status: doing
---

# 最初の版を出す

spec/ の設計をもとに CLI を作り、試作の HTML を見て中身を決め、最初の版をリリースするまでのタスク。いちばん分かっていない HTML の中身を早く決めるため、check の基本部分の次に HTML を試作する。

---

## 1. 土台

- [ ] Cargo のプロジェクトを作る
- [ ] docv.yaml を読み込み、スキーマで検証する
- [ ] ルートの探索、`--config` と `--root`、ファイルの分類（管理対象、管理外、除外）

---

## 2. check の基本部分

- [ ] check（frontmatter、構造、言い回し、リンク）
- [ ] docv 自身の docs に check をかけて通す

---

## 3. HTML の試作

- [ ] build（各ファイルのページと、時間と種類ごとの一覧だけ）
- [ ] 試作の HTML を build する
- [ ] 生成したものを見て画面とセクションを決め、spec/build.md に書き、fixme/open.md の 1 章を消す
- [ ] 過去の言い回しの既定の語を見直す（fixme/open.md の 8 章）

---

## 4. 残りのコマンド

- [ ] check の書式の規則（lint.style）
- [ ] check の textlint（組み込みと持ち込み）。プリセットを両リポジトリにかけ、既定で切るルールを決める（fixme/open.md の 12 章）
- [ ] fmt（dprint-plugin-markdown を埋め込み、fixme/open.md の 11 章を確かめる）
- [ ] index
- [ ] desc（ルールファイルを書いて埋め込む）
- [ ] migrate-guide
- [ ] init、new（テンプレを埋め込む）
- [ ] retire

---

## 5. 仕上げ

- [ ] JPG
- [ ] docv.yaml のスキーマをまとめて見直す
- [ ] build の出力を README からリンクする
- [ ] GitHub Actions でのリリース
