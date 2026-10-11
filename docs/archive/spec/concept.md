---
question: docs/spec/concept.md から外した内容と、外した理由は何か
source: docs/spec/concept.md
---

# spec/concept.md の経緯

## 2026-10-11 AI Agent 向けの Skill を提供し、init で CLAUDE.md と hook を設定する案

- 移した元: [spec/concept.md](../../spec/concept.md)
- 理由: docv はルールファイルを公開するところまでにする。ルールに従って直させる作業や CLAUDE.md への追記は、フレームワークの外で利用者が行う

ドキュメントを書く手順（変更の分類、書く先の選択、書き換え、retire）を Skill として提供し、`docv init` で CLAUDE.md への追記と、編集のたびに check を実行する hook の設定まで行う案だった。

## 2026-10-10 GitHub Actions で check と HTML の公開を行う案

- 移した元: [spec/concept.md](../../spec/concept.md)
- 理由: ローカルで完結するほうが使いやすい

PR のたびに check を実行し、main にマージしたら HTML を公開する案だった。

## 2026-10-10 実装とのずれを検出する案

- 移した元: [spec/concept.md](../../spec/concept.md)
- 理由: 実装状態をドキュメントに書かない方針にしたので、ずれを管理する必要がない。検討中の仕様は proposal に、コードの改善は Issue にする

frontmatter に根拠のコードのパスと確認した日を書いておき、その後にコードが変わったら check が「要確認」として出す案だった。
