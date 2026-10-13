---
mission: docv の最初の版を出すまでに、何をどの順でやるか
tags: [計画]
---

# 最初の版を出す

`spec/` の設計をもとに CLI を作り、最初の版をリリースするまでのタスク。Slot と HTML は、このリポジトリのドキュメントで使いながら直していく。

---

## 1. Slot と HTML

- [ ] このリポジトリの Slot と HTML を見て、足りない Slot や見せ方を直す
- [ ] 過去の言い回しの既定の語を見直す（`fixme/open.md` の 4 章）

---

## 2. 仕上げ

- [ ] textlint のプリセットを docs にかけ、docv の既定で切るルールを決める（`fixme/open.md` の 6 章）
- [ ] `docv.yaml` のスキーマをまとめて見直す
- [ ] build の出力を README からリンクする
- [ ] GitHub Actions でのリリース
