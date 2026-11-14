---
mission: テストの範囲と方法（自動テストの範囲、E2E テスト、手動テストについてなど）
---

# テスト方針

自動テストは `cargo test` で、`make fix` の中で実行する（[開発コマンド](develop.yaml)）。

## 自動テストで確かめること

- テストは、確かめるコードと同じファイルの `tests` モジュールに書く
- ファイルを読むものは、一時ディレクトリに小さなプロジェクトを作り、check の指摘や build が書いた HTML の文字列を確かめる
- 埋め込むものどうしが揃っていることも確かめる。スキーマと `docv/free/schema/` の写し、YAML の Slot の雛形とスキーマ、`assets/update-guide.yaml` にある今の版と 1 行の長さ

## 自動テストで確かめないこと

- textlint と Chrome は、`cargo test` では呼ばない。Node.js と Chrome がなくても通るようにするため。確かめるのは、渡す設定と、受け取った結果の読み方だけ
- 代わりに、`make fix` で、このリポジトリのドキュメントに textlint を含む docv check と、JPG を含む docv build をかける。docv を使うプロジェクトとして、外から通して確かめる E2E テストに当たる
- HTML と JPG の見た目は、人間が `docv open` で開いて確かめる
