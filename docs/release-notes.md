# Prism Relay 1.0.0

MinecraftとLunar Clientの設定を、共有コードで別のPCへ持ち運べるアプリです。共有する項目を選び、適用前に差分を確認できます。

- 設定フォルダの自動検出、プロファイル選択、検索付きツリー、プリセット
- 圧縮した共有コード、ファイル保存、QR生成と画像・カメラからの読み取り
- 部分Import、差分表示、適用前の自動バックアップ、復元
- 左メニュー下部のボタンで通常表示・アイコン表示を滑らかに切り替え、状態を保存。アニメーション軽減に対応し、設定一覧の表示幅も拡大
- 日本語・英語・韓国語・中国語・フィンランド語・スペイン語・ドイツ語
- ダーク・ライト・OS連動テーマ、アニメーションの軽減
- 端末内のMinecraft・Lunar Clientアイコンを表示
- Windows用の角丸アプリアイコン
- 見出し周囲の背景色を統一
- GitHub APIで最新バージョンを確認
- 作者: Shake_1227 · X: @shake_1227

Windowsではx64またはARM64のMSI / setup.exeを選んでインストールしてください。Windows・Mac版の対応するソースは、添付の `prism-relay-v1.0.0-source.tar.gz` です。

### Macで初めて開くとき

Apple SiliconまたはIntelのDMGを開き、Prism RelayをApplicationsへドラッグしてください。現在のMac版はAppleの公証を受けていないため、初回起動時に「悪質なソフトウェアかどうかをAppleで確認できない」と表示されます。

このGitHubリリースから入手したPrism Relayを開く場合は、次の手順で許可できます。

1. ApplicationsのPrism Relayを開き、警告を閉じます。
2. **システム設定 → プライバシーとセキュリティ**を開き、下にスクロールします。
3. Prism Relayの **「このまま開く」** を押します。
4. 確認画面で **「開く」** を選びます。

許可後は通常どおり起動できます。詳しくは[Appleの案内](https://support.apple.com/ja-jp/102445#openanyway)を参照してください。

Discordには共有コードを貼り付けて送れます。2000文字を超える場合は、`.prism`ファイルの添付が便利です。

対応しているLunar設定は `docs/lunar-schema.md` に記載しています。HUD座標は元の値を引き継ぐため、解像度の違う端末では適用後に位置を確認してください。`optionsof.txt` は検出とバックアップに対応しています。

各インストーラーのSHA-256は `SHA256SUMS.txt` で確認できます。

GPL-3.0-or-later。Lunar Client、Minecraftの非公式ツールです。
