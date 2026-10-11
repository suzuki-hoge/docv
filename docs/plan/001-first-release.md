---
question: docv の最初の版を出すまでに、何をどの順でやるか
status: todo
---

# 最初の版を出す

spec/ の設計をもとに CLI を作り、試作の HTML を見て中身を決め、最初の版をリリースするまでのタスク。

---

## 1. 土台

- [ ] Cargo のプロジェクトを作る
- [ ] docv.yaml を読み込み、スキーマで検証する
- [ ] ルートの探索、`--config` と `--root`、ファイルの分類（管理対象、管理外、除外）

---

## 2. 読むだけのコマンド

- [ ] check
- [ ] index
- [ ] desc（ルールファイルを書いて埋め込む）
- [ ] migrate-guide

---

## 3. HTML の試作

- [ ] build（各ファイルのページと、時間と種類ごとの一覧だけ）
- [ ] migrate-guide と build を試す
- [ ] 生成したものを見て画面とセクションを決め、spec/build.md に書き、fixme/open.md の 1 章を消す
- [ ] 過去の言い回しの既定の語を見直す（fixme/open.md の 8 章）

---

## 4. 書くコマンド

- [ ] init、new（テンプレを埋め込む）
- [ ] retire

---

## 5. 仕上げ

- [ ] JPG
- [ ] docv 自身のドキュメントに check をかけて通す。build の出力を README からリンクする
- [ ] GitHub Actions でのリリース
