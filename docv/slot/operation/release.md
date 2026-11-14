---
mission: 版の決め方と、利用者への届け方
---

# リリース

版は `Cargo.toml` の `version` で決め、版のタグを push する。バイナリは配らず、利用者は GitHub のソースから cargo で入れる（[インストール](../service/install.yaml)）。

## 手順

1. `Cargo.toml` の `version` を上げ、次の 2 つと一緒にコミットする
   - `assets/update-guide.yaml` の先頭に、その版の変更点と対応を足す。対応のない版も書く。docv update-guide が出す 1 行は 100 文字以内にし、収まらない変更は分ける。足していないときと、長すぎるときは cargo test が落ちる
   - このリポジトリの `docv.yaml` の `docv_version` を、その版にする
2. v で始まるタグを push する（人間が行う）

   ```sh
   git tag v1.0.0
   git push origin v1.0.0
   ```

タグは、`docv init` と `docv slot gen` がファイルの 1 行目に書くスキーマの URL の参照先になる。タグを push するまで、その版の URL は開けない。
