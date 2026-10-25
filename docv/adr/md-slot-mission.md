---
group: 構造化
date: 2026-10-25
---

# md の Slot の YAML front matter に、ミッションを必ず書く

## 背景

Slot のミッションは docv が決め、`docv slot list` で出すだけだった。YAML の Slot は雛形の 1 行目にミッションのコメントがあるが、md の Slot は空のファイルで、YAML front matter も持たなかった。AI Agent がいつも `docv slot list` を見るとは限らず、md の Slot のファイルを開いても、何を書く Slot かが分からなかった。Slot は、複数のプロジェクトでドキュメントの決まりと見方を揃えるためにあり、ミッションをプロジェクトで変えることはできない。

## 決めたこと

md の Slot には、YAML front matter の `mission` に、`docv slot list` のミッションと同じ文を必ず書く。ページに分けたサービス仕様は、どのページにも `order` と一緒に書く。`mission` がないか、ミッションと違えば、check が slot-mission の error にする。`docv slot gen` は、`mission` を書いた YAML front matter だけのファイルを作る。Doc ガイドの雛形にも書いておく。

## 結果

どの md の Slot も、ファイルを開けば何を書く Slot かが分かる。誰かが書き換えても、docv の版を上げてミッションが変わっても、check の error で気づき、直すまで check が通らない。直すのは AI Agent で、error のメッセージに書くべき文を出す。`mission` が違っても、HTML には本文を出す。

docv のサービス仕様（`docv/service/service.md`）は、`assets/desc.md` へのシンボリックリンクである。そのため `assets/desc.md` にも YAML front matter を書く。`docv desc` は、YAML front matter を除いた本文を出す。

## 選ばなかった案

| 案                                                  | 選ばなかった理由                                                                    |
| --------------------------------------------------- | ----------------------------------------------------------------------------------- |
| 1 行目に HTML のコメントで書く                      | ただのメモになり、書き換えられても、版を上げてミッションが変わっても気づけない      |
| `mission` を任意にする                              | 書いていない Slot では、ファイルを開いてもミッションが分からない                    |
| ミッションが違えば、docv fix で書き換える           | fix は整形と textlint の自動修正だけをする決まり。check の error で直させれば足りる |
| `mission` が違う Slot は HTML に出さない            | 版を上げただけで、直すまでページが消える                                            |
| YAML の Slot にも、`mission` を必須の項目として書く | YAML の Slot はスキーマで形を強制している。項目にすると、HTML の出力にも乗る        |
