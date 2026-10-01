# Prism Relay 1.0.1

Lunar Clientで、共有コードの読み込みに成功してもゲーム設定が反映されない問題と、適用先が複数あると別のプロフィールが自動で選ばれる問題を修正しました。

- 保存先が複数ある場合は、適用するプロフィールを選択
- Lunarの `optionsLC.txt`（JSON形式）に対応。バージョン別フォルダでは `options.txt` とまとめて更新・バックアップ
- LunarとMinecraftで保存形式が異なるFOVを変換
- 差分・確認・完了画面に、対象のプロフィールと設定ファイルを表示
- 適用に失敗した後は、最新の差分を確認して再試行
- Mac・Windowsで、異なる共有コードの連続適用とバックアップからの復元を検証
- 適用時アニメーションを追加しました。

Lunar Clientの操作・画面設定は、**Minecraft（ゲーム設定）**の適用先で使用中のLunarバージョンを選びます。**Lunar（HUD・MOD設定）**はHUDやMODのプロフィールです。

ゲームを終了してから適用し、完了後に起動してください。Lunar 1.21.11の設定フォルダが `1.21` の場合は、適用先で「Lunar 1.21」を選びます。

Windowsはx64・ARM64のMSIまたはsetup.exe、MacはApple Silicon・IntelのDMGを選んでインストールしてください。対応するソースは、添付の `prism-relay-v1.0.1-source.tar.gz` です。

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
