---
mission: テストやビルドなど、開発者が実行するコマンド
---

# 開発コマンド

| すること                 | コマンド                                                               |
| ------------------------ | ---------------------------------------------------------------------- |
| 整えて確かめる           | `make fix`                                                             |
| 雛形を整えて確かめる     | `make check-templates`                                                 |
| 手元の docv を入れ直す   | `make install`                                                         |
| 版を上げる               | `make new-version V=x.y.z`                                             |
| 任意の版の docv を入れる | `cargo install --git https://github.com/suzuki-hoge/docv --tag v1.7.0` |

## 整えて確かめる

```sh
make fix
```

コードかドキュメントを変えたら、コミットの前に実行する。指摘が 1 件でもあれば止まる。textlint を使うので Node.js が要る。

<details>
<summary>行われること</summary>

- コードを `cargo fmt` で整え、`cargo clippy` と `cargo test` を通す
- `make check-templates` で docv を install し、雛形を整えて確かめる。以降は、install した docv で実行する
- このリポジトリのドキュメントを `docv fix` で整え、`docv check` を通す
- `docv build` で、HTML を `output/docv/html/` に、インデックスを `docv/README.md` に作り直す。どちらもコミットする。GitHub で `docv/README.md` から Slot を辿れるようにし、clone してすぐ `docv open` で HTML を見られるようにするため

</details>

## 雛形を整えて確かめる

```sh
make check-templates
```

`make fix` も、途中でこれを実行する。

<details>
<summary>行われること</summary>

- docv を install する
- `docv gen` が書く雛形（`assets/template/`）を一時ディレクトリに書き出して `docv fix` する。雛形は管理対象の外にあるため
- 変わった雛形を書き戻して、install し直す
- 書き出した雛形に `docv check` を通す

</details>

## 手元の docv を入れ直す

```sh
make install
```

## 版を上げる

```sh
make new-version V=x.y.z
```

人間が実行する。前もって、`assets/update-guide.yaml` の先頭にその版の変更点と対応を書いておく。コミットはしない（版上げのコミットに入れる）。

| オプション | 意味         |
| ---------- | ------------ |
| `V=x.y.z`  | 上げる先の版 |

<details>
<summary>行われること</summary>

- 版を書き換えて `make fix` を通す
- 「版を x.y.z にする」のコミット、タグ、push までを行う

</details>

## 任意の版の docv を入れる

```sh
cargo install --git https://github.com/suzuki-hoge/docv --tag v1.7.0
```

版のタグを指定して、GitHub のソースから入れる。前の版の動きを確かめるときに使う。

| オプション   | 意味           |
| ------------ | -------------- |
| `--tag <版>` | 入れる版のタグ |

<details>
<summary>行われること</summary>

- `make install` で入れた docv を置き換える
- 手元のソースに戻すときは、もう一度 `make install` を実行する

</details>
