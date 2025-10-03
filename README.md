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

## Grafanaでのデータ可視化

gnt2influxで収集したネットワーク測定データをGrafanaで地図上に可視化できます。

### Grafanaセットアップ（Docker使用）

#### 1. InfluxDBとGrafanaを起動

```bash
# InfluxDB 1.8を起動
docker run -d --name influxdb \
  -p 8086:8086 \
  -e INFLUXDB_DB=gnettrack \
  influxdb:1.8

# Grafanaを起動
docker run -d --name grafana \
  -p 3000:3000 \
  grafana/grafana:latest
```

#### 2. データをInfluxDBにアップロード

```bash
# KMLファイルをアップロード
./gnt2influx -i your_data.kml

# または詳細ログ付きでアップロード
./gnt2influx -i your_data.kml -v
```

#### 3. Grafanaでデータソースを設定

1. ブラウザで http://localhost:3000 にアクセス
2. admin/admin でログイン（初回時はパスワード変更を要求される場合があります）
3. 左メニューの「Configuration」→「Data Sources」を選択
4. 「Add data source」をクリック
5. 「InfluxDB」を選択
6. 以下の設定を入力：
   - **Name**: InfluxDB
   - **URL**: http://host.docker.internal:8086
   - **Database**: gnettrack
   - **User**: （空白）
   - **Password**: （空白）
7. 「Save & Test」をクリックして接続を確認

#### 4. 地図ダッシュボードを作成

1. 左メニューの「+」→「Dashboard」を選択
2. 「Add panel」をクリック
3. パネルタイプを「Geomap」に変更
4. **Query Builder を使用（推奨）**:
   
   Raw Query Modeでは構文エラーが発生する場合があるため、Query Builderの使用を強く推奨します：
   
   1. クエリエディタで「Query Builder」モードになっていることを確認
   2. **FROM**: `network_measurements` を選択
   3. **SELECT**: 「+ Query」ボタンをクリックして以下のフィールドを個別に追加
      - `field(longitude)` 
      - `field(latitude)`
      - `field(level)` 
      - `field(speed)`
   4. **GROUP BY**: 設定不要（空のまま）
   5. **WHERE**: 時間フィルターは自動的に適用されます
   
   **追加情報が必要な場合:**
   - オペレーター情報: `operator_name` (tag)
   - 通信技術: `network_tech` (tag)
   
   **注意:** 
   - Raw Query Modeは避けてください（構文エラーの原因）
   - Query Builderなら確実に動作します

5. 「Query Options」で「Format as」を「Table」に設定
6. パネル設定で以下を調整：
   - **View**: 地図の中心座標（例：lat=35.7, lon=139.52）
   - **Zoom**: 適切なズームレベル（例：16）
   - **Layers**: データポイントの表示設定

#### 5. 信号強度による色分け設定

1. 「Field」タブを選択
2. 「Thresholds」で信号レベルに応じた色分けを設定：
   - 赤: -∞ to -110 dBm（弱い信号）
   - 黄: -110 to -95 dBm（中程度の信号）
   - 緑: -95 to +∞ dBm（強い信号）

#### 6. ダッシュボードの保存

1. 右上の「Save」ボタンをクリック
2. ダッシュボード名を入力（例：「Network Measurements Map」）
3. 「Save」をクリック

### 地図で確認できる情報

作成したダッシュボードでは以下の情報が地図上で確認できます：

- **測定ポイントの位置**: 経度・緯度による正確な位置
- **信号強度**: 色分けによる電波強度の視覚化
- **移動速度**: 各測定ポイントでの移動速度
- **通信事業者**: オペレーター名（KDDI、ドコモ等）
- **ネットワーク技術**: 3G、4G、5G等の技術情報
- **時系列変化**: タイムラインでの信号変化の追跡

### 自動設定スクリプト（オプション）

手動設定が面倒な場合は、以下のAPIコマンドで自動設定できます：

```bash
# データソースを自動設定
curl -X POST -H "Content-Type: application/json" -d '{
  "name": "InfluxDB",
  "type": "influxdb",
  "url": "http://host.docker.internal:8086",
  "access": "proxy",
  "database": "gnettrack",
  "user": "",
  "password": "",
  "basicAuth": false
}' http://admin:admin@localhost:3000/api/datasources

# 基本的な地図ダッシュボードを作成（Query Builder形式）
curl -X POST -H "Content-Type: application/json" -d '{
  "dashboard": {
    "title": "Network Measurements Map",
    "panels": [{
      "title": "Signal Strength Map",
      "type": "geomap",
      "targets": [{
        "measurement": "network_measurements",
        "select": [
          [{"type": "field", "params": ["longitude"]}],
          [{"type": "field", "params": ["latitude"]}],
          [{"type": "field", "params": ["level"]}],
          [{"type": "field", "params": ["speed"]}]
        ],
        "groupBy": [],
        "where": [],
        "rawQuery": false,
        "resultFormat": "table"
      }]
    }]
  }
}' http://admin:admin@localhost:3000/api/dashboards/db
```

### 高度な可視化設定

#### ヒートマップ表示
信号強度をヒートマップで表示する場合：
1. パネルタイプを「Heatmap」に変更
2. X軸：longitude、Y軸：latitude
3. 値：level（信号レベル）

#### 時系列アニメーション
移動経路をアニメーションで表示する場合：
1. 「Time range」を調整して特定時間範囲を選択
2. 「Refresh」を短い間隔（5秒など）に設定
3. Grafanaの「Playlist」機能で自動更新

### トラブルシューティング

#### クエリエラー「invalid statement: ,」が発生する場合

このエラーはRaw Query ModeでInfluxDBクエリを手動入力した際に発生します：

**🚫 問題のある方法: Raw Query Mode**
```sql
SELECT longitude, latitude, level, speed FROM network_measurements
-- または
SELECT longitude,latitude,level,speed FROM network_measurements
```

**✅ 推奨解決方法: Query Builder を使用**

1. クエリエディタで「Query Builder」モードを選択
2. Raw Query Mode（テキスト入力）は使用しない
3. GUIでフィールドを選択する方式で設定

**Query Builder の利点:**
- 構文エラーが発生しない
- Grafanaが自動的に正しいクエリを生成
- InfluxDBのバージョンや設定の違いに影響されない

#### その他の一般的な問題

1. **データソース接続エラー**
   - InfluxDBのURLが正しいか確認（Docker使用時は`http://host.docker.internal:8086`）
   - データベース名が正しいか確認（`gnettrack`）

2. **データが表示されない**
   - 時間範囲を確認（`WHERE time > now() - 24h`を`WHERE time > now() - 7d`に変更してテスト）
   - データが正常にInfluxDBにアップロードされているか確認

3. **地図が表示されない**
   - パネルタイプが「Geomap」になっているか確認
   - `longitude`と`latitude`フィールドが含まれているか確認

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