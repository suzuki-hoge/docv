---
group: コマンド
date: 2026-10-13
---

# Slot のディレクトリでも除外のルールを守る

## 背景

check は、`slots.dir` の下のファイルをすべて Slot の候補として読み、除外するかどうかを見ていなかった。`output.dir` の既定の `docv/build` は `slots.dir` の中にある。そのため、build のあとの check が、出力の HTML と JPG を slot-unknown の error にしていた。macOS の `.DS_Store` と、`.gitignore` に書いたファイルも、同じく error になっていた。

## 決めたこと

Slot と ADR を探すときも、原本と同じく、`.gitignore` に書かれたもの、`output.dir` の中、`ignore` に当てはまるものを除く。名前がドットで始まるファイルとディレクトリも読まない。Slot の名前はドットで始まらないため。`output.dir` の既定は `docv/build` のままにする。

## 結果

既定の設定のまま build しても、check は出力を見ない。`slots.dir` の中に置いたドットで始まるファイルは、Slot の名前に近くても slot-unknown にならない。

## 選ばなかった案

| 案                                           | 選ばなかった理由                                                                      |
| -------------------------------------------- | ------------------------------------------------------------------------------------- |
| `output.dir` の既定を `slots.dir` の外にする | 出力の問題しか解けず、`.DS_Store` と `.gitignore` に書いたファイルは error のまま残る |
