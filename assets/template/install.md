---
mission: 利用者がこのサービスをインストールする方法
---

# インストール

## 前提とされるもの

| 名前 | 用途         |
| ---- | ------------ |
| Git  | ソースを取る |

## 手順

### バイナリを取る

macOS では、次を実行する。

```sh
curl -L https://example.com/example-macos.tar.gz | tar xz
```

Windows では、リリースのページから zip を取り、展開する。

### 動くことを確かめる

```sh
example --version
```
