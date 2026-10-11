---
question: docv の最初の版を出すまでに、何をどの順でやるか
status: doing
---

# 最初の版を出す

spec/ の設計をもとに CLI を作り、試作の HTML を見て中身を決め、最初の版をリリースするまでのタスク。いちばん分かっていない HTML の中身を早く決めるため、ほかのコマンドより先に HTML を試作する。

---

## 1. HTML の試作

- [ ] 生成したものを見て画面とセクションを決め、spec/build.md に書き、fixme/open.md の 1 章を消す
- [ ] 過去の言い回しの既定の語を見直す（fixme/open.md の 7 章）

---

## 2. 仕上げ

- [ ] textlint のプリセットを docs にかけ、docv の既定で切るルールを決める（fixme/open.md の 9 章）
- [ ] JPG
- [ ] docv.yaml のスキーマをまとめて見直す
- [ ] build の出力を README からリンクする
- [ ] GitHub Actions でのリリース
