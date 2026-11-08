# v0.1.0

Open Design（Vela）のアカウント情報を JSON で出力する CLI の初回リリース。

## 新機能

- `quota` コマンドで、保存済み全アカウントのクオータ・素性・ワークスペース属性を 1 本の JSON にまとめて出力
- `--model` でモデル別の `billing/preflight`（残量・使用量・可否）を取得
- `--entitled` で Link の付与モデル一覧（context/output バジェット、入力モダリティ付き）を取得
- `--catalog` で公開モデルカタログ全件を取得
- `--surfaces` で到達できる読み取りエンドポイントの HTTP ステータス一覧を出力
- `account import-amr` で `~/.amr/config.json` から認証情報を取り込み（元ファイルは変更しない）
- `account add` / `list` / `remove` で複数アカウントを管理。鍵は argv ではなく環境変数名か標準入力から受け取る

## 変更点

- Windows は DPAPI、Linux/macOS はパスフレーズ由来の AES-256-GCM（PBKDF2-HMAC-SHA256、20 万回）で保存物を暗号化
- パスフレーズが無い Unix 環境では平文 JSON（0600）にフォールバックし、`storage.encrypted: false` と `storage.protection: "plain"` で明示
- 保存は同一ディレクトリの一時ファイル → rename の原子置換。Unix はディレクトリ 0700・ファイル 0600
- 取得できなかった項目はキーごと省略し、`null` や `0` で埋めない
- `ok`（クオータ本体が取れたか）と `complete`（要求した全ステージが成功したか）を分離

## 注意点

- `billing/ledger`（403）と `billing/usage`（401）は制御プレーン鍵では取得できない。`--surfaces` はこの事実をステータスとして記録する
- `billing/request-concurrency` はサーバー側が 404 のため未提供
- モデル別のクレジット内訳は API が返さないため取得できない
- DPAPI で保存したファイルは他のユーザー・他のマシンへコピーすると復号できない
- `User-Agent` を常に送る。`/api/v1/billing/summary` は Go 既定 UA を Cloudflare が遮断するため、UA 無しでは 403 になる

## 配布物

- `odquota-x86_64-windows.zip`
- `odquota-x86_64-linux.tar.gz`
- `odquota-aarch64-linux.tar.gz`
- `odquota-x86_64-macos.tar.gz`
- `odquota-aarch64-macos.tar.gz`

各アーカイブに `.sha256` を併載。
