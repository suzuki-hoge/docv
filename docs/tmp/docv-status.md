# docv の作業状況（2026-10-11 時点）

## 終わったこと

- 設計
  - spec 14 ファイル（考え方、種類、frontmatter、書き方、docv.yaml、CLI 全体、check、retire、build、migrate-guide、書式の規則、textlint、fmt、配布）
  - このリポジトリの rule、archive（検討で捨てた案）、JSON Schema、種類ごとのテンプレ、CLAUDE.md
- CLI の土台
  - docv.yaml の読み込みとスキーマ・意味の検証
  - --config / --root、ルートの探索、ファイルの分類（管理対象、管理外、除外）
- docv check の基本部分
  - frontmatter、構造、言い回し、リンクのルール
  - docs の項目ごとの check: warn / off、パス指定
  - docv 自身の docs は error 0 件、warn 0 件
- docv build の試作
  - ファイルごとのページと、時間 → 種類の一覧
  - リンクの書き換え、出力が毎回同じになること、出力先を作り直すときの安全策（.docv の目印）
- tmp という種類（このメモを書いたときに追加）
- Homebrew の tap は用意しないと決めた ( どこにも何も書かないので決定で完 )

## やっていないこと

決まっていて spec に書いていないものは、spec に書いたら fixme/open.md から消す。

- 配布
  - GitHub Actions でのリリース
- HTML およびドキュメントの構造全般
  - 画面とセクションを決めて spec/build.md に書く（決まっていないことの HTML の項目が決まってから）
  - JPG。違うマシンでも同じ JPG にする（フォントを埋め込んで固定する）、と spec に書く。チーム開発で、実行するマシンごとに差分が出るのを避けるため
  - build の出力を README からリンクする
  - proposal と fixme は分けたままにする、と spec に書く。proposal は確度の高い内容、fixme は雑多なメモに近い
- YAML スキーマ
  - docv.yaml のスキーマをまとめて見直す（もう少し進んだら人間がまとめて確認する）
- CLI およびツールの挙動
  - check の書式の規則、check の textlint、fmt
  - index、desc（ルールファイルの文面から）、migrate-guide
  - init、new、retire
  - CLI のメッセージとルールファイルは日本語だけにする、と spec に書く
  - textlint の自動修正を docv から呼ぶ、と spec に書く
- ほか
  - 名前を docv に決めた、と書く（document viewer の略、読み方はドックブイ）。「仮の名前」の書き方を外す

## 決まっていないこと

- HTML およびドキュメントの構造全般
  - 一覧の分け方。「時間 → 種類」のままか、「動かし方 / 費用 / 仕様 / 未決」のような問いの入口にするか
  - 種類の中の並べ方。topics で分けるか、ディレクトリで分けるか
  - 追加する要素。サイドバー、見出しの id とページ内の目次、種類 × 話題のマップ、リンクの関係図、最近の変更
  - HTML で費用を集計するか、git の履歴から最近の変更を出すか、Mermaid を描画するか
- YAML スキーマ
  - docs の項目の `check: off` の値を変えるか（YAML 1.1 のパーサが真偽値として読む）
- CLI およびツールの挙動
  - 過去の言い回しの既定の語
  - rule を言い回しの警告の対象から外すか（rule が「未実装」をルールの説明として使うと引っかかる）
  - リンクのアンカーを check するか
  - retire のあとに章番号を振り直すか
  - desc のルールファイルと spec をどう揃えるか
  - dprint-plugin-markdown で決めた整形の形にできるか
  - textlint のプリセットで既定で切るルール
