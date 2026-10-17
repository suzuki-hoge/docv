---
group: 配布
date: 2026-10-17
---

# スキーマを JSON に揃え、YAML の Slot の 1 行目から参照する

## 背景

`docv.yaml` のスキーマは、エディタが読めるよう JSON で書き、YAML の Slot のスキーマは、`docv slot schema` で読みやすいように YAML で書いていた。形を分ける理由は、ほかになかった。YAML の Slot はスキーマを参照しておらず、エディタで補完が効かなかった。

## 決めたこと

スキーマは、すべて JSON Schema を JSON で書き、`assets/schema/<名前>.schema.json` に置く。`docv slot gen` は、YAML の Slot の 1 行目に、yaml-language-server の書式でスキーマの URL を書く。URL は `docv init` と同じく、実行した CLI の版のタグの `assets/schema/` を指す。

## 結果

`docv.yaml` と YAML の Slot のどちらも、エディタで項目の補完と検証が効く。版のタグを push するまでは、URL の先がない。check は今までどおり Slot の型で確かめる。

## 選ばなかった案

| 案                         | 選ばなかった理由                                    |
| -------------------------- | --------------------------------------------------- |
| YAML に揃える              | YAML で書いたスキーマを、エディタが読める保証がない |
| main ブランチの URL を使う | 使っている CLI の版と、スキーマがずれる             |
