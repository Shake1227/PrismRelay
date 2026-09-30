# Prism Relay

Lunar Client と Minecraft の設定を、選んだ項目だけ共有する Windows / macOS アプリです。設定をローカルで解析し、サーバー不要の共有コードにまとめます。

[Download](https://github.com/Shake1227/prism-relay/releases) · [Share format](docs/share-format.md) · [Verified Lunar schema](docs/lunar-schema.md)

![Prism Relay — sample data](docs/screenshots/home.jpg)

## Features

- 独自のガラス調 UI、日本語、ダーク / ライト / OS 連動テーマ、Reduce Motion。
- 設定フォルダの自動検出、プロファイル選択、検索・親子選択に対応したツリー。
- HUD、PvP、パフォーマンス、操作設定などのプリセットとカスタムプリセット保存。
- プレビュー、圧縮コード、コピー、ファイル保存、短いコードの QR 表示。
- Import 時の再選択、適用先選択、差分、自動バックアップ。
- Atomic Write、Rollback、中断した処理の復旧、Restore。
- 手動での更新確認。自動ダウンロード・自動適用はありません。

## Supported platforms

| Platform | Release artifact | Requirement |
| --- | --- | --- |
| Windows x64 | Portable ZIP / `PrismRelay.exe` | Windows 10 / 11、Microsoft WebView2 |
| Windows ARM64 | Portable ZIP / `PrismRelay.exe` | Windows 11 ARM64、Microsoft WebView2 |
| macOS Apple Silicon | ZIP / `.app` | macOS 11 以降 |
| macOS Intel | ZIP / `.app` | macOS 11 以降 |

ZIP を展開して起動します。アプリのインストーラーは使用しません。Windows では PC のアーキテクチャに合う x64 または ARM64 の ZIP を選んでください。Windows 11 には通常 WebView2 が含まれます。未導入の場合は [Microsoft WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) が必要です。

初期版は開発者証明書による署名・macOS の公証を行っていません。OS が確認を求める場合があります。macOS では起動を試した後、「システム設定 → プライバシーとセキュリティ」で対象アプリを確認できます。[署名の構成](docs/signing.md)を用意しています。

## How to use

ゲームを終了してから操作することをおすすめします。起動中の場合は警告し、勝手にプロセスを終了しません。

### Export

1. 自動検出された設定を確認します。未検出の場合は「設定」からフォルダを指定します。
2. 「エクスポート」で Minecraft の設定プロファイルと Lunar のプロファイルを選びます。
3. ツリーやプリセットで共有項目を選び、プレビュー後にコードを作成します。
4. コピーまたは `.prism` ファイルで持ち運びます。

`PRS1:` コードは MessagePack + Zstandard で圧縮し、SHA-256 の破損チェックを含みます。長いコードは QR ではなくファイルで共有してください。Resource Packs はパック名の設定であり、パック本体を含みません。

### Import

1. コードを貼り付けるか `.prism` を開きます。貼り付けただけでは適用されません。
2. 項目と適用先を選び、現在の値との差分を確認します。
3. 適用すると変更前の設定が自動保存されます。

適用先にない項目や型の異なる項目は警告して除外します。プレビュー後に設定が変わった場合は再確認が必要です。共有中のプロファイル名は匿名化され、ファイルパスとして使用されません。

### Backup and restore

Import と Restore の前に対象ファイルをアプリ専用フォルダへ保存します。「バックアップ」から手動保存、詳細、保存先表示、復元、削除を操作できます。復元前にも現在の状態を保存します。

一時ファイルを検証・同期してから Atomic Replace します。複数ファイルの処理には永続的な記録を残し、起動時に中断処理を復旧します。別アプリによる変更を検出した場合は上書きせず、バックアップを保護します。復旧が必要なバックアップを明示的に選択して復元できます。

保存先は OS のアプリデータディレクトリ内の `io.github.shake1227.prismrelay/backups` です。バックアップには元ファイル全体と元パスが含まれるため、端末内の個人データとして扱ってください。共有コードには含めません。

## Security

共有対象を検証済みのファイル・キー・値形式の許可リストに限定します。アカウント、認証情報、Token、Cookie、サーバーアドレス、ログ、未知項目、Waypoints、マクロは共有しません。

Minecraft の未知行と Lunar の未知 JSON 値を保持し、選択した既存項目だけ変更します。シンボリックリンク、不正パス、過大なファイルやコード、破損コード、未対応バージョンを拒否します。

SHA-256 は破損検出用で、送信者の認証ではありません。コードは暗号化されません。カスタムリソースパック名は選択すると含まれるため、必要に応じてそのカテゴリを外してください。

## Privacy

設定の読み取り・共有コード生成・Import・バックアップはローカルで動作します。Analytics、Telemetry、アカウント登録、共有用 Backend はありません。設定値やコード全体を診断ログに記録しません。

更新確認を押した場合だけ GitHub API に接続します。GitHub / ライセンス / リリースノートのボタンはブラウザで開きます。設定データは送信しません。

## Current limits

Lunar 対応は実ファイルを読み取り専用で調査したスキーマに基づきます。未検証の設定を推測で書き換えません。`optionsof.txt` は検出とローカルバックアップに対応し、初期版では共有・Import の対象外です。

HUD 座標は保存された形式のまま転送します。解像度による自動補正は未実装なので、適用後に位置を確認してください。アクティブプロファイルや Lunar のバージョンは非公開情報から推測しません。

ブラウザの `npm run dev` は明示されたサンプルモードです。合成データと `PRDEMO1:` を使い、実際の設定にアクセスしません。本番の `PRS1:` はデスクトップ版で操作してください。README の画像もサンプルデータです。

## Development

Node.js 22 以降、Rust 1.90 以降、[Tauri のプラットフォーム依存環境](https://v2.tauri.app/start/prerequisites/)が必要です。

```sh
npm ci
npm run tauri dev
```

```sh
npm run typecheck
npm run lint
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

`src/` は UI、`src-tauri/src/` は scanner / parser / safety / codec / importer / backup を担当します。テストは合成データと一時ディレクトリを使います。

## Build and releases

```sh
npm run tauri build -- --bundles app
```

macOS は `src-tauri/target/release/bundle/macos/Prism Relay.app` を生成します。Windows は `npm run tauri build -- --no-bundle` でポータブル実行ファイルを生成します。

GitHub Actions は Type Check、Lint、Test、Windows x64 / Windows ARM64 / macOS Apple Silicon / macOS Intel のビルドを実行します。package / Cargo / Tauri のバージョンを同期し、`v0.1.0` のようなタグを Push すると、プラットフォーム別 ZIP と `SHA256SUMS.txt` を Release に公開します。既存タグや Release は上書きしません。

依存ライブラリ更新後は `python3 scripts/generate-notices.py` でライセンス通知を更新してください。

## License

[GPL-3.0-or-later](LICENSE)。依存ライブラリの通知は [THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt) と About 画面に含まれます。

## Disclaimer

Lunar Client、Minecraft、Microsoft、Mojang とは独立した非公式ツールです。各社との提携・承認・公式な関係はありません。各製品名・商標はそれぞれの権利者に帰属します。
