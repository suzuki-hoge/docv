---
group: コマンド
date: 2026-10-12
---

# YAML の Slot のスキーマを docv schema で出す

## 背景

YAML の Slot の項目の意味と、必須かどうかを知る手段が、テンプレのサンプルと check のエラーしかなかった。docv desc と docv check だけで docv を入れたり直したりするには、項目の意味が要る。

## 決めたこと

YAML の Slot ごとに、YAML で書いた JSON Schema を `assets/schema/` に置き、バイナリに埋め込む。`docv schema <slot>` で Slot のスキーマを、`docv schema` で `docv.yaml` のスキーマを出す。同じものを `docs/schema/` の Markdown に写し、HTML で公開する。

## 結果

AI Agent は `docv desc` から `docv schema <slot>` にたどり着き、項目の意味を知ってから Slot を書ける。スキーマと `docs/schema/` の写し、テンプレとこのリポジトリの Slot がスキーマに合っていることは、cargo test で確かめる。check は今までどおり Slot の型で確かめる。
