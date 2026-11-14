---
group: コマンド
date: 2026-11-07
---

# docv を更新したときの変更点と対応を、docv update-guide で出す

## 背景

docv の版を上げたとき、利用者が変わったことを知る手段は、`docv check` の error と `docv desc` だけだった。check で気づけるのは、ルールに合わなくなったことだけである。足された機能、Doc ガイドの雛形の変更、出力のパスの変更は、check を通っても伝わらない。出力のパスが変わると、README から JPG へのリンクが切れる。

以前は、ファイルと `docv.yaml` に版を書き、版ごとの手順を出すコマンドを持っていた。`docv desc` と `docv check` で直せるとして、どちらもやめていた。そのため、docv を更新した後では、直前まで使っていた版が分からない。`docv init` と `docv slot gen` が 1 行目に書くスキーマの URL には版があるが、書いたときの版で、docv を更新しても変わらない。

## 決めたこと

- `docv update-guide --from <版>` で、指定した版より後の版の変更点と対応を、古い版から順に出す。対応のない版も、「対応は要らない」と書いて出す
- 変更点と対応はバイナリに埋め込み、版を上げるときに足す。手元の docv の版と、出す内容を一致させるため
- `docv.yaml` の `docv_version` に、ドキュメントを合わせた docv の版を書く。`docv init` が書く
- `docv_version` が docv の版と違えば、パッチの版の違いでも check が error にし、update-guide に渡す版を案内する。docv を更新した AI Agent が、check の error を手がかりに update-guide を見つけられるようにするため
- `docv.yaml` と YAML の Slot の 1 行目にあるスキーマの URL の版も、`docv_version` と違えば check が error にする。エディタが、古い版のスキーマで補完と検証をしないようにするため
- update-guide の最後に、`docv_version` とスキーマの URL の版を上げるよう案内する
- update-guide の使い方は、`docv desc` と利用マニュアルに書き、Doc ガイドの雛形には書かない。docv を更新するときにしか要らないため

## 結果

docv を更新すると check が error になり、AI Agent は案内に従って update-guide を実行する。check では気づけない変更にも対応できる。

版を上げるたびに、対応のない版でも、`docv_version` とスキーマの URL を書き換えるコミットが要る。チームの中では、docv の版を揃えることになる。CI で docv の最新を入れていると、新しい版が出たときに check が落ちるので、`docv_version` の版を入れる。

1.8.0 より前から使っているプロジェクトには `docv_version` がないので、check が error にする。使っていた版は、`docv.yaml` の 1 行目のスキーマの URL から推して案内する。

## 見直す条件

- 「対応は要らない」だけの版が、3 つ続けて出たとき。パッチの版の違いでは check を落とさない形を考える
- update-guide の対応をせずに、`docv_version` だけを書き換えたプロジェクトを見つけたとき

## 選ばなかった案

| 案                                                                     | 選ばなかった理由                                                                           |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| 1 行目のスキーマの URL の版を、使っている版として扱う                  | 決まりをコメントの行に持たせることになる。ローカルのパスでスキーマを指すと、版が分からない |
| 更新の前に `docv --version` で版を控えてもらう                         | 人が cargo install で更新すると、控える機会がない                                          |
| 変更点を GitHub の CHANGELOG に置く                                    | 手元の docv の版と、読む内容がずれる。AI Agent がネットワークで取りに行くことになる        |
| Doc ガイドの雛形で update-guide を案内する                             | 更新のときにしか要らない情報を、いつも読ませることになる                                   |
| `docv_version` の違いを check で知らせず、`docv desc` の案内だけにする | AI Agent は、docv が更新されたことを知らない                                               |
| パッチの版の違いは、check で知らせない                                 | パッチの版にも、対応の要る変更が入ることがある                                             |

## 参考

- [設定ファイル](../slot/service/service/config.md)
- [利用マニュアル](../slot/service/manual.md)
- [リリース](../slot/operation/release.md)
