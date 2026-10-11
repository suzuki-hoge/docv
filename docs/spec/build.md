---
question: docv build は何を生成し、その出力をどう扱うか
topics: [CLI, 出力]
---

# build

人間向けの HTML と JPG を生成するコマンド。HTML にどんな画面とセクションを置くかは [fixme/open.md](../fixme/open.md) の 1 章で決める。設定は [spec/config.md](config.md) の output。

---

## 1. 使い方

```sh
docv build              # output.dir に出す
docv build --out <dir>  # 出力先を一時的に変える
```

---

## 2. 出力

```
docv/                 # output.dir
  index.html
  ...                 # 各ページ
  assets/             # CSS と JS
  jpg/                # output.jpg が true のとき
  .gitignore          # output.commit が false のとき
```

- build は、毎回 output.dir の中を作り直す。output.dir の外には書かない

---

## 3. HTML

- 静的なファイルだけで動く。file:// で開いても、そのままホスティングしても見られる
- CSS と JS はバイナリに埋め込み、assets/ に書き出す。外部の CDN は使わない
- 管理対象のファイルを 1 ファイルずつ 1 ページにし、タイトルと question を並べて表示する
- リンクの書き換え
  - 管理対象のファイルへのリンクは、そのファイルのページへのリンクにする
  - 管理対象でないファイルへのリンクは、repository.url があれば GitHub の URL にし、なければ元の相対パスのまま出力先から辿れるように直す
- 画面の構成で決まっているのは、次のことだけ
  - 時間（未来、現在、過去）と種類で分けて見られる
  - 過去は、現在と混ざらないように分けて置く
  - 管理外のファイルは「その他」として並べる

---

## 4. 毎回同じものを作る

- 生成日時、実行したマシンのパス、乱数を出力に入れない
- 並び順は、パスの辞書順に固定する
- 入力（管理対象のファイル、docv.yaml、docv の版）が同じなら、HTML と assets はバイト単位で同じになる
- JPG は、同じマシン・同じ Chrome・同じフォントなら同じになる。マシンが違うと変わりうる

---

## 5. JPG

- output.jpg が true のとき、HTML の各ページを 1 枚ずつ JPG にし、jpg/ に置く
- ローカルの Chrome をヘッドレスで起動し、DevTools Protocol で操作して撮る。幅と画質は固定
- Chrome は次の順で探す
  1. 環境変数 DOCV_CHROME
  2. OS ごとの標準のインストール先
  3. PATH にある chrome、chromium
- 見つからなければ error で終わる。output.jpg が false なら Chrome はいらない
- README などに `![docv](docv/jpg/index.jpg)` のように埋め込めば、ホスティングしなくても GitHub で見られる

---

## 6. git で管理するか

| output.commit | 動き | 使い方 |
|---|---|---|
| true | 何もしない。出力をコミットするのは利用者 | README からリンクする、GitHub Pages でホスティングする |
| false | output.dir に、`*` だけを書いた .gitignore を置く | 手元で見るだけにする |

リポジトリのルートの .gitignore には触らない。
