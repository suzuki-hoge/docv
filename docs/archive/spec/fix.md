---
question: docs/spec/fix.md から外した内容と、外した理由は何か
source: docs/spec/fix.md
---

# spec/fix.md の経緯

## 2026-10-11 整形だけをする docv fmt と、整形が必要なファイルを出す docv fmt --check

- 移した元: [spec/fix.md](../../spec/fix.md)
- 理由: textlint の自動修正も docv から呼ぶことにし、整形とまとめて docv fix にした。check に --fix を付けないのは、check が通らないのに直さない選択肢はないため。check は検査だけにし、整形が必要なファイルは check の format-diff で分かるので --check もなくした

```sh
docv fmt               # 管理対象のファイルをすべて整形する
docv fmt <パス...>     # 指定したファイルだけ整形する
docv fmt --check       # 書き換えず、整形が必要なファイルを出す。あれば終了コード 1
```

- 対象は管理対象のファイルだけ。docs の項目で check: off にしたファイルと、tmp のファイルは整形しない
- Markdown として読めないファイルは、書き換えずにエラーとして出す
