---
group: 構造化
date: 2026-10-17
---

# docv が読み書きするディレクトリを dir にまとめる

## 背景

Free ファイルの置き場所は `docs`（glob の配列）、Slot は `slots.dir`、出力は `output.dir` と、置き場所の項目が散らばっていた。`docs` が複数書けるため、リンクの表示文字の起点を推し量る必要があった。出力を git で管理するかを `output.commit` で決め、docv が出力先に `.gitignore` を置いていた。

## 決めたこと

- 置き場所は `dir` にまとめ、`free`（Free ファイル）、`slots`（Slot ファイル）、`html`（HTML）、`image`（JPG）を書く
- `dir.free` はディレクトリ 1 つだけにする。リンクの表示文字は、このディレクトリからのパスにする
- `dir.image` を書いたときだけ、Slot のページを JPG にする
- 出力を git で管理するかは、プロジェクトの `.gitignore` に任せ、docv は `.gitignore` を書かない

## 結果

置き場所はすべて `dir` を見れば分かる。`dir.free` の外の Markdown（`README.md` など）は、管理対象にならない。

## 選ばなかった案

| 案                                     | 選ばなかった理由                         |
| -------------------------------------- | ---------------------------------------- |
| `docs` を glob の配列のまま残す        | 複数書く場面がなく、起点の推し量りが要る |
| `output.image` を true か false で残す | JPG の置き場所を HTML と分けられない     |
