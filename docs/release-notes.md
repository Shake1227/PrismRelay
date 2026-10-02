# Prism Relay 1.0.4

LunarのMOD設定の共有項目を増やしました。

- 確認済みの表示オプション、切り替え、キー設定を共有
- 文字色・背景色・押下時の色、不透明度、虹色の動きに対応
- HUDや表示ボックスの大きさ・余白を共有
- ファイルに省略された確認済み既定値も一覧に表示し、対象のMODやHUDがある共有先に適用
- 色の差分を見本と不透明度で表示
- 適用時アニメーションを追加しました。

共有する側と読み込む側を更新し、新しく共有コードを作成してください。旧版のコードも読み込めます。旧形式の一部項目は、差分画面で適用対象外と表示される場合があります。未確認の項目やプライベートなデータは共有対象外です。

ゲームを終了してから適用し、完了後に起動してください。

Windowsはx64・ARM64のMSIまたはsetup.exe、MacはApple Silicon・IntelのDMGを選んでインストールしてください。対応するソースは、添付の `prism-relay-v1.0.4-source.tar.gz` です。

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
