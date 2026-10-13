# リリース

版は `Cargo.toml` の `version` で決め、タグの push を合図に、GitHub Actions が各プラットフォーム向けのバイナリを GitHub Release に置く。

## 手順

1. `Cargo.toml` の `version` を上げてコミットする
2. v で始まるタグを push する（人間が行う）

   ```sh
   git tag v1.0.0
   git push origin v1.0.0
   ```

3. GitHub Actions が各プラットフォーム向けにビルドし、GitHub Release に置く

## 置くもの

- アーカイブの名前は `docv-<版>-<target>.tar.gz`。Windows は .zip。チェックサムも置く
- 対象は macOS（arm64、x86_64）、Linux（x86_64、arm64）、Windows（x86_64）
