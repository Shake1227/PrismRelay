# Prism Relay 0.1.0

MinecraftとLunar Clientの設定を、共有コードで別のPCへ持ち運べるアプリです。共有する項目を選び、適用前に差分を確認できます。

- 設定フォルダの自動検出、プロファイル選択、検索付きツリー、プリセット
- 圧縮した共有コード、ファイル保存、QR生成と画像・カメラからの読み取り
- 部分Import、差分表示、適用前の自動バックアップ、復元
- 日本語・英語・韓国語・中国語・フィンランド語・スペイン語・ドイツ語
- ダーク・ライト・OS連動テーマ、アニメーションの軽減
- 端末内のMinecraft・Lunar Clientアイコンを表示
- 作者: Shake_1227 · X: @shake_1227

Windowsではx64またはARM64のMSI / setup.exeを選んでインストールしてください。macOSではApple SiliconまたはIntelのDMGを開き、Prism RelayをApplicationsへドラッグしてください。

Discordには共有コードを貼り付けて送れます。2000文字を超える場合は、`.prism`ファイルの添付が便利です。

対応しているLunar設定は `docs/lunar-schema.md` に記載しています。HUD座標は元の値を引き継ぐため、解像度の違う端末では適用後に位置を確認してください。`optionsof.txt` は検出とバックアップに対応しています。

開発者署名・macOS公証は未取得です。OSの確認画面が出ることがあります。各インストーラーのSHA-256は `SHA256SUMS.txt` で確認できます。

GPL-3.0-or-later。Lunar Client、Minecraftの非公式ツールです。
