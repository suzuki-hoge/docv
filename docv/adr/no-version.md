---
group: 記録
date: 2026-10-12
---

# docv で版を管理しない

## 背景

原本の YAML front matter に、そのファイルが従う docv の版（`version`）を書いていた。check は版が古いと知らせ、migrate-guide はファイルの版から今の版まで、版ごとの手順をつないで出していた。`docv.yaml` にも、設定ファイルの形式の版を書いていた。

## 決めたこと

docv から版の管理をなくす。原本と `docv.yaml` の `version` をなくす。check の version-missing、version-newer、version-older と、migrate-guide の版の章もなくす。docv を上げたときも、後から入れるときも、`docv desc` と `docv check` を見て直す。

## 結果

原本の YAML front matter は `mission` と `tags` だけになる。Slot の書式が版で変わったら、check のエラーで気づく。

## 選ばなかった案

| 案                                                       | 選ばなかった理由                                    |
| -------------------------------------------------------- | --------------------------------------------------- |
| 原本の `version` を残す                                  | `docs/` のファイルは、docv で版を管理する必要がない |
| 版ごとの手順だけ残し、migrate-guide で元の版を指定させる | `docv desc` と `docv check` で直せる                |
| `docv.yaml` にプロジェクトの版を 1 つ書く                | `docv desc` と `docv check` で直せる                |
