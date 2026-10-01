# Prism Relay 1.0.2

共有コードの読み込みで設定が反映されない問題を修正しました。

- Minecraftの起動構成選択を廃止し、既定の設定フォルダを使用
- 同じフォルダの `options.txt` と `optionsLC.txt` をまとめて更新・バックアップ
- Lunarのランチャーや補助プロセスをゲーム起動中と判定する問題を修正
- 使用中のLunar GUIプリセットを初期選択
- Lunar MODの有効・無効を、保存先で省略された項目にも適用
- MODの有効・無効の対応範囲と、一部の数値設定の互換性を改善
- 完了画面を表示位置に戻し、適用後の演出を修正
- 適用時アニメーションを追加しました。

LunarのHUD・MOD設定は、使用中のGUIプリセットを初期選択します。必要に応じて変更してください。Minecraftのゲーム設定は、Windowsでは `%APPDATA%/.minecraft`、Macでは `~/Library/Application Support/minecraft` が対象です。設定フォルダを変更している場合は、アプリの環境設定からそのフォルダを指定できます。

Lunar独自MODは、確認済みの有効・無効と設定項目を共有します。共有元で省略された既定値はコードに含まれません。

ゲームを終了してから適用し、完了後に起動してください。

Windowsはx64・ARM64のMSIまたはsetup.exe、MacはApple Silicon・IntelのDMGを選んでインストールしてください。対応するソースは、添付の `prism-relay-v1.0.2-source.tar.gz` です。

### Macで初めて開くとき

Apple SiliconまたはIntelのDMGを開き、Prism RelayをApplicationsへドラッグしてください。現在のMac版はAppleの公証を受けていないため、初回起動時に「悪質なソフトウェアかどうかをAppleで確認できない」と表示されます。

このGitHubリリースから入手したPrism Relayを開く場合は、次の手順で許可できます。

1. ApplicationsのPrism Relayを開き、警告を閉じます。
2. **システム設定 → プライバシーとセキュリティ**を開き、下にスクロールします。
3. Prism Relayの **「このまま開く」** を押します。
4. 確認画面で **「開く」** を選びます。

許可後は通常どおり起動できます。詳しくは[Appleの案内](https://support.apple.com/ja-jp/102445#openanyway)を参照してください。
各ファイルのSHA-256は `SHA256SUMS.txt`、ビルド元は `BUILD_INFO.json` で確認できます。

作者: Shake_1227 · [X: @shake_1227](https://x.com/shake_1227)

GPL-3.0-or-later。Lunar Client、Minecraftの非公式ツールです。
