---
question: docv は何で作られ、どう配られ、利用者はどう入れるか
topics: [配布]
---

# 配布

docv の作り、リリースの仕組み、利用者の入れ方。コマンドは [spec/cli.md](cli.md)。

---

## 1. 作り

- Rust で書いた単一のバイナリ。利用者に Rust の環境はいらない
- テンプレ、ルールファイル（`docv desc` が出すもの）、HTML の CSS と JS は、バイナリに埋め込む
- JSON Schema（[schema/docv.schema.json](../../schema/docv.schema.json)）は、リポジトリに置いて公開する

---

## 2. 対象のプラットフォーム

| OS | アーキテクチャ | target |
|---|---|---|
| macOS | arm64 | aarch64-apple-darwin |
| macOS | x86_64 | x86_64-apple-darwin |
| Linux | x86_64 | x86_64-unknown-linux-musl |
| Linux | arm64 | aarch64-unknown-linux-musl |
| Windows | x86_64 | x86_64-pc-windows-msvc |

---

## 3. リリース

- リポジトリは public にする。GitHub が用意する標準のランナーは、public リポジトリなら macOS と Windows も無料で使える
- v で始まるタグを push すると、GitHub Actions が各プラットフォーム向けにビルドし、GitHub Release にアーカイブとチェックサムを置く
- アーカイブの名前は `docv-<版>-<target>.tar.gz`。Windows は .zip
- スキーマはタグごとに固定され、docv.yaml からは版を含む URL で参照する（[spec/config.md](config.md) の 6 章）

---

## 4. 入れ方

```sh
# macOS（arm64）の例
curl -L https://github.com/<owner>/docv/releases/download/v1.0.0/docv-1.0.0-aarch64-apple-darwin.tar.gz | tar xz
mv docv /usr/local/bin/
```

- ブラウザでダウンロードしたバイナリは、macOS の Gatekeeper に止められることがある。curl で取れば止められないので、手順は curl で書く
- Rust の環境がある人は、`cargo install --git https://github.com/<owner>/docv` でも入れられる
