# gnt2influx

G-NetTrack Liteのログデータを解析し、InfluxDBに変換・転送するツールです。

## 機能

- G-NetTrack Liteのログファイル（テキスト形式）を解析
- KMLファイル（Google Earth形式）を解析
- InfluxDB 1.x および 2.x に対応
- バッチ処理による効率的なデータ転送
- エラー処理とスキップ機能
- macOS および Linux 対応

## インストール

### バイナリダウンロード

[Releases](https://github.com/your-repo/gnt2influx/releases)ページから、お使いのプラットフォーム用のバイナリをダウンロードしてください。

### ソースからビルド

```bash
git clone https://github.com/your-repo/gnt2influx.git
cd gnt2influx
cargo build --release
```

## 使用方法

### 基本的な使用方法

```bash
# テキストログファイルの場合
./gnt2influx -i /path/to/logfile.txt

# KMLファイルの場合
./gnt2influx -i /path/to/data.kml
```

### 設定ファイルを指定

```bash
./gnt2influx -i /path/to/logfile.txt -c /path/to/config.toml
```

### 接続テスト

```bash
./gnt2influx --test-connection
```

### ドライラン（解析のみ、アップロードなし）

```bash
./gnt2influx -i /path/to/logfile.txt --dry-run
```

### 詳細ログ出力

```bash
./gnt2influx -i /path/to/logfile.txt -v
```

## 設定ファイル

設定ファイル（`config.toml`）の例：

```toml
[influxdb]
url = "http://localhost:8086"
database = "gnettrack"
username = ""
password = ""
# InfluxDB 2.x用（オプション）
org = ""
token = ""

[logging]
level = "info"

[processing]
# 一度に処理するレコード数
batch_size = 1000
# 無効なレコードをスキップするかどうか
skip_invalid = true
```

### InfluxDB 1.x の設定

```toml
[influxdb]
url = "http://localhost:8086"
database = "gnettrack"
username = "your_username"
password = "your_password"
```

### InfluxDB 2.x の設定

```toml
[influxdb]
url = "http://localhost:8086"
database = "gnettrack"
org = "your_organization"
token = "your_api_token"
```

## G-NetTrack ログ形式

このツールは以下のG-NetTrackログフィールドをサポートしています：

- Timestamp（タイムスタンプ）
- Longitude/Latitude（経度/緯度）
- Speed（速度）
- Operator（通信事業者）
- CGI（Cell Global Identity）
- Cell ID（セルID）
- LAC（Location Area Code）
- Network Technology（ネットワーク技術：2G/3G/4G/5G）
- Signal Level（信号レベル）
- Signal Quality（信号品質）
- SNR（Signal-to-Noise Ratio）
- CQI（Channel Quality Indicator）
- ARFCN（Absolute Radio Frequency Channel Number）
- Bitrate（上り/下りビットレート）

## InfluxDB データ形式

データは以下の形式でInfluxDBに保存されます：

### Measurement: `network_measurements`

#### Tags（インデックス付きフィールド）
- `measurement_type`: "gnettrack"
- `operator_name`: 通信事業者名
- `operator_code`: 事業者コード（MCC-MNC）
- `cell_id`: セルID
- `network_tech`: ネットワーク技術
- `network_mode`: ネットワークモード
- `lac`: Location Area Code

#### Fields（値フィールド）
- `longitude`: 経度（float）
- `latitude`: 緯度（float）
- `speed`: 速度（float）
- `level`: 信号レベル（float）
- `qual`: 信号品質（float）
- `snr`: SNR（float）
- `cqi`: CQI（float）
- `dl_bitrate`: 下りビットレート（float）
- `ul_bitrate`: 上りビットレート（float）
- `cgi`: CGI（string）
- `cellname`: セル名（string）
- `node`: ノード情報（string）
- `arfcn`: ARFCN（string）

## コマンドラインオプション

```
gnt2influx [OPTIONS] -i <FILE>

OPTIONS:
    -i, --input <FILE>        G-NetTrackログファイルのパス
    -c, --config <FILE>       設定ファイルのパス [デフォルト: config.toml]
        --test-connection     データをアップロードせずにInfluxDB接続をテスト
        --dry-run            ログファイルを解析するがInfluxDBにアップロードしない
    -v, --verbose            詳細ログを有効にする
    -h, --help               ヘルプ情報を表示
    -V, --version            バージョン情報を表示
```

## 対応プラットフォーム

- macOS（x86_64, ARM64）
- Linux（x86_64）

## トラブルシューティング

### 接続エラー

InfluxDBへの接続でエラーが発生する場合：

1. InfluxDBサービスが起動していることを確認
   ```bash
   # Dockerで起動する場合
   docker run -d --name influxdb -p 8086:8086 -e INFLUXDB_DB=gnettrack influxdb:1.8
   ```
2. 設定ファイルのURL、ユーザー名、パスワードを確認
3. ネットワーク接続を確認
4. 接続テストを実行
   ```bash
   ./gnt2influx --test-connection
   ```

### ログ解析エラー

ログファイルの解析でエラーが発生する場合：

1. ファイル形式を確認
   - テキストファイル：タブ区切りまたはカンマ区切り
   - KMLファイル：Google Earth形式のXML
2. `--dry-run`オプションで解析をテスト
   ```bash
   ./gnt2influx -i your_file.kml --dry-run -v
   ```
3. `skip_invalid = true`設定で無効レコードをスキップ

## ライセンス

MIT License

## 貢献

Issue報告やプルリクエストを歓迎します。

## 開発

### InfluxDBの起動方法

#### Dockerを使用する場合（推奨）

```bash
# InfluxDB 1.8を起動
docker run -d --name influxdb \
  -p 8086:8086 \
  -e INFLUXDB_DB=gnettrack \
  influxdb:1.8

# データの確認
curl -G 'http://localhost:8086/query?pretty=true' \
  --data-urlencode "db=gnettrack" \
  --data-urlencode "q=SELECT * FROM network_measurements LIMIT 10"
```

#### InfluxDBクライアントでの確認

```bash
# InfluxDBクライアントに接続
influx -host localhost -port 8086

# データベースを選択
USE gnettrack

# データを確認
SELECT * FROM network_measurements LIMIT 10;

# 特定の時間範囲のデータを確認
SELECT * FROM network_measurements WHERE time > now() - 1h;

# オペレーター別の統計
SELECT COUNT(*) FROM network_measurements GROUP BY operator_name;
```

### 必要な環境

- Rust 1.70以上
- InfluxDB（テスト用）

### テスト実行

```bash
cargo test
```

### フォーマットチェック

```bash
cargo fmt --check
```

### Clippyチェック

```bash
cargo clippy -- -D warnings
```