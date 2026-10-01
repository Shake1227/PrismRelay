# Prism Relay 1.0.1

設定の読み込み先が複数あると、使用中とは別のMinecraft・Lunarプロフィールへ自動的に適用してしまう問題を修正しました。

- 保存先が複数ある場合は、適用するプロフィールを選択
- 差分・確認・完了画面に、対象のプロフィールと設定ファイルを表示
- 適用に失敗した後は、最新の差分を確認して再試行
- Mac・Windowsで、異なる共有コードの連続適用とバックアップからの復元を検証

Lunar Clientの操作・画面設定は、**Minecraft（ゲーム設定）**の適用先で使用中のLunarバージョンを選びます。**Lunar（HUD・MOD設定）**はHUDやMODのプロフィールです。

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
