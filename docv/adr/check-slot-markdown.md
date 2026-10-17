---
group: コマンド
date: 2026-10-17
---

# md の Slot と ADR にも、Free ファイルと同じ検査をかける

## 背景

`dir.slot` の中の Markdown（md の Slot と ADR）には、Free ファイルのルールをかけず、整形と textlint だけをかけていた。リンクが切れていても、HTML ではただの文字になるだけで、気づけなかった。書式の規則も、Free ファイルにだけかかっていた。

## 決めたこと

md の Slot と ADR にも、YAML front matter のほかは、Free ファイルと同じ検査をかける。リンク切れ、見出しの数と章番号、書式の規則、整形、textlint。ADR の YAML front matter は、今までどおり ADR の決まりで確かめる。

## 結果

どの Markdown でも、同じ基準でリンク切れと書き方を確かめる。md の Slot と ADR には、YAML front matter の `mission` を求めない。
