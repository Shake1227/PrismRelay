# Prism Relay 1.0.3

画面サイズが異なるPCへの読み込みで、LunarのHUD位置を自動調整できるようになりました。

- 通常のゲームウインドウを最大化した場合のサイズに合わせてX・Yを補正
- MinecraftのGUIサイズ、Windowsの表示設定、MacのRetina表示に対応
- 差分画面に補正後の値とウインドウサイズを表示
- 元の座標で読み込む切り替えと、サイズの手動入力を追加

自動調整はLunarでMinecraft 1.16.1以降を使う場合の確認済みバージョンが対象です。古いバージョンやサイズ・設定を取得できない場合は、補正をオフにして読み込めます。

共有する側と読み込む側を更新し、新しく共有コードを作成してください。旧版のコードも読み込めます。HUDの自動調整を使う場合は、共有元のサイズ情報を含む新しいコードが必要です。

ゲームを終了してから適用し、完了後に起動してください。

Windowsはx64・ARM64のMSIまたはsetup.exe、MacはApple Silicon・IntelのDMGを選んでインストールしてください。対応するソースは、添付の `prism-relay-v1.0.3-source.tar.gz` です。

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
