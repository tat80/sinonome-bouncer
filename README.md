# Sinonome Bouncer

PNGタイルを読み込み、デスクトップ上をバウンドするアニメーションを表示するWindows向けアプリです。
GPUレンダリングによりそこそこ低負荷です。たぶん。

## ダウンロード

最新版は [Releases](https://github.com/tat80/sinonome-bouncer/releases/) からダウンロードできます。

ダウンロードしたzipファイルを展開したフォルダの中の `sinonome_bouncer.exe` を実行してください。

## 実行

- `Microsoft Visual C++ 2015-2022 再頒布可能パッケージ（x64）`が必要です。入っていない場合は、予めインストールしておいてください。
- 終了する場合は、タスクトレイ（システムトレイ）のアイコンを左クリックor右クリックから「終了」を選択してください。
- 動作の設定は `config.txt` で行えます。
- 実行時に実行ディレクトリにある `anim.png` を読み込み描画します。PNGの透明部分は透過されます。
    `anim.png` は、各フレームを250x250ピクセルのタイルとして左から右方向へ並べたPNGを用意すれば、別の画像でもアニメーション可能です。
- Windows 11で動作を確認しています。うまく動かない時は、ウイルス対策ソフトが止めていないか確認してみてください。

### ソースから実行・ビルドする場合

Rust（`rustup`）と `Visual Studio Build Tools（Desktop development with C++）` が必要です。

```powershell
cargo run --release
```

## 設定

### 設定例

```text
frame_count=5
scale=1.0
anim_fps=5.0
frame_delays_ms=200,100,100,100,140
speed=1.7
```

### 設定項目の説明

- `frame_count`はPNG内のタイル数以下にしてください。フレームは左上を0番目として、行優先順で読み込まれます。
- `scale=2.0`なら表示サイズが縦横2倍、`scale=0.5`なら半分になります。
- `anim_fps=5.0`はアニメーションの1秒あたりのフレーム数を指定します。`frame_delays_ms`が指定されている場合は、無視されます。
- `frame_delays_ms`は各フレームの表示時間をミリ秒単位で指定します。例えば`frame_delays_ms=200,100,100,100,140`のように指定すると、各フレームの表示時間を個別に設定できます。指定する個数は`frame_count`と一致させてください。この設定がある場合は`anim_fps`より優先されます。設定範囲は1以上65535以下 (ミリ秒) です。
- `speed`はデスクトップ内の移動速度で、`speed=2.0`なら基準速度の2倍、`speed=0.5`なら基準速度の半分になります。

## Special thanks

- しののめにこ
- うぇいん さん

## License

[Nicochi License](LICENSE).

権利類は元の作者に帰属します。空気を読んで常識的な範囲で使用することが求められます。
