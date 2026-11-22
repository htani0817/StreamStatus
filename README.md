---
title: StreamStatus for Twitch
date: 2026-11-22
updated: 2026-11-23
tags:
  - コーディング/rust
  - コーディング/windows
  - プロジェクト/開発中
---

# StreamStatus for Twitch

Twitch の配信タイトル・カテゴリ・タグをプリセットとして登録しておき、配信前に1回押すだけで切り替える Windows アプリです。タイトルに `{n}` を入れておけば、配信回数も自動で入ります。

[English summary](#english)

## ダウンロードと起動

1. [Releases](https://github.com/htani0817/StreamStatus/releases) から `StreamStatus-v<バージョン>.exe` をダウンロードする
2. 好きな場所に置いて起動する。インストールは要りません
3. 「Twitch でログイン」を押す。ブラウザで Twitch の認証ページが開くので、アプリに出ている8文字のコードを入れて「許可」を押す

署名していないため、初回起動で「Windows によって PC が保護されました」と出ることがあります。「詳細情報」→「実行」で起動できます。

## 使い方

- **プリセットを作る**: 「プリセットを追加」か、左の「いまの配信」の「プリセットに保存」から作ります
- **反映する**: カードの「適用」を押すと、タイトル・カテゴリ・タグがまとめて Twitch に反映されます
- **配信回数**: タイトルに `{n}` を入れると、その場所に配信回数が入ります。「加算して適用」は回数を1つ進めてから反映します。回数は編集画面で直せます。全角の `｛ｎ｝` も同じに扱います
- **反映中の印**: いま Twitch に出ている内容と同じプリセットには、アンバーの点と「反映中」が付きます
- **告知文**: カードのコピーボタンか「告知文をコピー」で、ひな形から作った告知文をクリップボードに入れます。「X で投稿」は X の投稿画面を開きます（投稿は自分で押します）
- **並び替え**: カード左のつまみをドラッグします。つまみを選んで上下の矢印キーでも動かせます
- **ほかの操作**: カードの「…」から、複製・書き出し・削除などができます

### ショートカット

| キー | 動き |
| --- | --- |
| F5 / Ctrl+R | いまの配信を Twitch から読み直す |
| Ctrl+N | プリセットを追加 |

## 機能

- プリセットの件数に上限はありません。タイトル・カテゴリ・タグ（最大10個・1個25文字、空白と半角記号は不可）・メモ（500文字、Twitch には送らない）・配信回数を持ちます
- カテゴリは名前を入れると候補を出します
- バックアップ: 設定画面からすべてを `.tsb` に、カードの「…」から1件を `.tsp` に書き出せます。読み込みは「追加」と「置き換え」を選べます
- 告知文のひな形には `{title}` `{category}` `{tags}` `{url}` `{name}` `{n}` を差し込めます
- 表示モードは OS に合わせる・ライト・ダークから選べます
- × を押すとタスクトレイにしまいます（設定で切り替えられます）。トレイのアイコンから開くか終了します。二重に起動すると、起動済みのウィンドウが前に出ます

## プライバシー

StreamStatus が通信する相手は Twitch だけです（ログインと API の `id.twitch.tv` / `api.twitch.tv`、カテゴリの画像の `static-cdn.jtvnw.net`）。作者のサーバーはありません。

- ログインは Twitch の Device Code Flow です。パスワードはこのアプリに入力しません
- 求める権限は `channel:manage:broadcast` だけです。使うのは配信タイトル・カテゴリ・タグの変更だけです
- Twitch の決まりに従い、起動時と1時間ごとにログインが有効かを確かめます

| 場所 | 中身 |
| --- | --- |
| `%APPDATA%\StreamStatus\data.json` | プリセットと設定。壊れていたら `data.broken-<時刻>.json` に名前を変えて残し、空の状態で起動する |
| Windows の資格情報マネージャー（「StreamStatus for Twitch」を含む項目） | ログイン情報（トークン）。ログアウトで消える |
| `%LOCALAPPDATA%\app.streamstatus.desktop` | 画面（WebView2）の一時データ |

## アンインストール

1. アプリの設定画面で「ログアウト」を押す（ログイン情報を消し、使っていたトークンを無効にする）
2. タスクトレイのアイコンから「終了」を選び、exe を削除する
3. 残したくなければ `%APPDATA%\StreamStatus` と `%LOCALAPPDATA%\app.streamstatus.desktop` を削除する
4. Twitch の「設定」→「接続」で「StreamStatus Desktop」の接続を解除する

## 動作環境と制限

- Windows 10 / 11 の x64。画面の表示には Microsoft Edge WebView2 ランタイムを使います（Windows 11 には最初から入っています）
- 画面の表示は日本語だけです
- 使うには、配信者本人の Twitch アカウントでログインする必要があります

## 不具合の報告

[Issues](https://github.com/htani0817/StreamStatus/issues) にお願いします。どの操作で、どんな文言のエラーが出たかを書いてもらえると調べやすくなります。

## 開発者向け

### ビルド

必要なもの: Rust（stable）、Visual Studio Build Tools の C++、Tauri CLI 2（`cargo install tauri-cli --version "^2" --locked`）。

```powershell
pwsh scripts/publish.ps1
```

`artifacts/StreamStatus-v<バージョン>.exe` ができます。ビルドの中間ファイルは `%LOCALAPPDATA%\cargo-target\stream-status` に置きます。

### Client ID

リポジトリには作者が登録した Client ID（Twitch では秘密情報ではない）が入っています。改造して配る場合は、自分の Client ID を使ってください。

1. [Twitch Developer Console](https://dev.twitch.tv/console/apps) でアプリを登録する
   - 名前: 「Twitch」を含む名前は「for Twitch」の形でも登録できません（Invalid client name）
   - OAuth のリダイレクト URL: `http://localhost` を入れて「追加」を押す（このアプリでは使いませんが、入力は必須です）
   - カテゴリー: `Broadcaster Suite`
   - クライアントのタイプ: 公開（Public）
2. `pwsh scripts/publish.ps1 -ClientId <Client ID>` でビルドする（`src-tauri/src/twitch.rs` の `BUILTIN_CLIENT_ID` を書き換えてもよい）

### テスト

```powershell
cd src-tauri
cargo test
```

`{n}` の置き換え、タグの検証、バックアップの読み書き、告知文、保存を確かめます。

画面だけをブラウザで確かめるときは、`ui` フォルダを http で配って開きます。Rust 側の代わりに `ui/js/mock.js` が動きます。URL に `?signed-out`（ログイン前）、`?empty`（プリセットなし）、`?open=editor` / `?open=settings` / `?open=login` / `?open=menu` を付けると、その状態で開きます。

### 構成

| 場所 | 中身 |
| --- | --- |
| `src-tauri/src/main.rs` | 起動・タスクトレイ・二重起動の防止・× でしまう処理・1時間ごとのトークン検証 |
| `src-tauri/src/twitch.rs` | ログイン（Device Code Flow）・トークンの更新・Helix API |
| `src-tauri/src/commands.rs` | 画面から呼ぶコマンド |
| `src-tauri/src/store.rs` | `data.json` の読み書き |
| `src-tauri/src/secrets.rs` | 資格情報マネージャーへの保存 |
| `src-tauri/src/text.rs` / `announce.rs` / `backup.rs` | `{n}`・タグの検証、告知文、書き出し形式 |
| `ui/` | 画面（素の HTML・CSS・JS。ビルド工程なし） |
| `assets/icon-source.png` | アイコンの原画（`cargo tauri icon` で `src-tauri/icons` を作る） |

## ライセンス

[MIT License](LICENSE)。使っている部品のライセンスは [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt) にあります。

## English

StreamStatus for Twitch is a small Windows app for streamers. Save your stream title, category and tags as presets and apply them to your Twitch channel with one click before you go live. Put `{n}` in a title to insert a stream counter, and use "加算して適用" (apply with +1) to advance it. It also copies a go-live announcement from a template and can open the X post screen. The UI is Japanese only for now.

Download `StreamStatus-v<version>.exe` from [Releases](https://github.com/htani0817/StreamStatus/releases) and run it; no installer is needed. Sign in with the Twitch device code flow (the app never sees your password; it only asks for `channel:manage:broadcast`). The executable is not code-signed, so SmartScreen may warn you on first launch ("More info" → "Run anyway"). Licensed under the [MIT License](LICENSE).
