# odquota

Open Design（Vela）のアカウント情報を **1 本の JSON** で返すネイティブ CLI。
クオータだけでなく、素性・ワークスペース属性・自動チャージ設定・モデル権利・到達可能なエンドポイントまでまとめて出す。
認証情報はプラットフォームごとの方式で暗号化して保存する（Windows は DPAPI、Linux/macOS はパスフレーズ由来の AES-256-GCM）。

- 実装: Rust（Windows / Linux / macOS、x86_64 と aarch64）
- 出力: stdout に JSON、診断は stderr。鍵は保存物にも出力にも現れない

## インストール

[Releases](https://github.com/heqvyfrogg/odquota/releases) から自分のプラットフォーム向けのアーカイブを取得し、展開して `odquota` を PATH の通った場所へ置く。

| プラットフォーム | ファイル |
| --- | --- |
| Windows x86_64 | `odquota-x86_64-windows.zip` |
| Linux x86_64 | `odquota-x86_64-linux.tar.gz` |
| Linux aarch64 | `odquota-aarch64-linux.tar.gz` |
| macOS x86_64 | `odquota-x86_64-macos.tar.gz` |
| macOS aarch64 | `odquota-aarch64-macos.tar.gz` |

各アーカイブに `.sha256` が併載されている。取得後は `sha256sum -c`（macOS は `shasum -a 256 -c`）で照合できる。

## ビルド

```
cargo build --release
```

ソースからビルドする場合、依存は crates.io から取得する。GitHub Actions の `CI` ワークフローが 3 OS でテストし、タグ `v*` の push で 5 ターゲットをビルドして Release に添付する。

## 使い方

```
odquota account import-amr [--profile prod] [--label NAME] [--workspace ID] [--force] [--config PATH]
odquota account add --label NAME (--control-key-env VAR | --stdin) [--runtime-key-env VAR]
                    [--api-url URL] [--link-url URL] [--workspace ID] [--force]
odquota account list [--pretty]
odquota account remove --label NAME
odquota quota [--label NAME | --all] [--model MODEL] [--entitled] [--catalog] [--surfaces] [--pretty]
```

`import-amr` は `~/.amr/config.json` の `profiles.<name>` から
`controlKey` / `runtimeKey` / `apiUrl` / `linkUrl` を取り込む。読み取りのみで、元ファイルは書き換えない。
`user` ブロック（メール・アバター）は保存しない。

鍵を argv に渡す手段は用意していない（プロセス一覧から見えるため）。
`--control-key-env <変数名>` で環境変数を参照するか、`--stdin` で 1 行読ませる。

### フラグの意味

| フラグ | 追加される情報 |
| --- | --- |
| `--model <id>` | `billing/preflight`（モデル別の使用量・残量・可否）を `preflight` と `windows[].usedCredits` に反映 |
| `--entitled` | Link の付与モデル一覧（`entitledModels`、context/output バジェットと入力モダリティ付き） |
| `--catalog` | 公開モデルカタログ全件（`catalog`、cost と capability） |
| `--surfaces` | 到達を試した読み取りエンドポイントの HTTP ステータス一覧（`surfaces`） |

## 出力

```json
{
  "schemaVersion": 1,
  "generatedAt": "2026-11-08T09:34:16.054Z",
  "storage": {
    "path": "/home/user/.local/share/odquota/accounts.dat",
    "encrypted": true,
    "protection": "passphrase"
  },
  "accounts": [
    {
      "label": "prod",
      "ok": true,
      "complete": true,
      "fetchedAt": "…",
      "identity": {
        "userId": "…", "authUserId": "…", "email": "…", "name": "…",
        "avatarUrl": "…", "status": "active", "plan": "free", "balanceUsd": "1.5000"
      },
      "workspace": {
        "workspaceId": "…", "workspaceMemberId": "…", "name": "…", "type": "personal",
        "role": "owner", "isDefault": true, "memberCount": 1, "iconKey": "person",
        "memberStatus": "active", "lifecycleState": "active", "ownerName": "…",
        "source": "resolved"
      },
      "quota": {
        "billing": { "billingState": "active", "planId": "go", "billingMode": "shared" },
        "plan": { "tier": "go", "interval": "month", "status": "active",
                  "currentPeriodStart": "…", "currentPeriodEnd": "…", "cancelAtPeriodEnd": false },
        "credits": { "subscriptionCredits": "10000", "rechargeCredits": "5000",
                     "totalAvailableCredits": "15000", "creditsPerUsd": 10000 },
        "wallet": { "balanceUsd": "1.5000", "expiresAt": "…",
                    "breakdown": [ { "kind": "signup_gift", "amountUsd": "0.5000",
                                     "expiresAt": "…", "allowedModels": ["public_model_deepseek_v4_1_flash"] } ] },
        "windows": [
          { "kind": "5h", "limitCredits": "50000", "remainingCredits": "44004",
            "usedCredits": "5996", "remainingPercent": 88.0, "exhausted": false,
            "policyId": "…", "durationSeconds": 18000,
            "windowStart": "…", "resetsAt": "…", "resetsInSeconds": 8259 }
        ],
        "consumption": { "usageCount": 9942, "todayConsumedUsd": "0.0000",
                         "totalConsumedUsd": "0.0000", "totalRechargedUsd": "0.5000" },
        "permissions": { "canUpgrade": true, "canRecharge": true,
                         "canConfigureAutoRecharge": true, "canManageSubscription": true },
        "autoRecharge": { "enabled": false, "lowerThresholdUsd": "10.00",
                          "topupAmountUsd": "50.00", "monthlyLimitUsd": "200.00",
                          "paymentMethodStatus": "ok", "hasLastFailure": false },
        "preflight": { "modelId": "deepseek-v4.1-flash", "modelCovered": true,
                       "funding": "coding_plan", "taskCheck": "ready" }
      },
      "surfaces": [
        { "path": "/api/v1/me", "httpStatus": 200 },
        { "path": "/api/v1/workspaces/{ws}/billing/ledger", "httpStatus": 403,
          "errorCode": "workspace_member_permission_denied" }
      ],
      "errors": []
    }
  ]
}
```

`{ws}` の部分は実際の workspaceId が入る。

### 値の扱い

- クレジット額は API が返す**文字列のまま**保持する。丸めも再計算もしない。
- `remainingPercent` は `remaining/limit` を整数演算して小数 2 桁で出す。
- `resetsInSeconds` は取得時刻との差。**過去日時は 0 に丸めず負値のまま**返す。
- 取得できなかった項目は**キーごと省略**する。`null` や `0` で埋めない。
- `windows` の土台は `billing/snapshot` の `designPlanWindows`。`--model` 指定時は `billing/preflight` の
  `usedCredits` / `policyId` / `durationSeconds` を同じ `kind` に重ね、残量は preflight の値を優先する。

### `ok` と `complete`

| フィールド | 意味 |
| --- | --- |
| `ok` | クオータ本体（snapshot か summary）が取れたか |
| `complete` | `ok` かつ、要求した全ステージが成功したか |

`--entitled` を付けて runtimeKey が未登録の場合、クオータは正常に取れるので `ok:true, complete:false` になる。
`quota` は出したまま `errors` に理由を残し、終了コードは 1。

逆に snapshot も summary も取れないアカウントは `ok:false` で `quota` を出さない（0 クレジットに見せない）。

## 保存と暗号化

保存先は単一のバイナリブロブ `accounts.dat`。書き込みは同一ディレクトリの一時ファイル → rename の原子置換。

| プラットフォーム | 方式 | `protection` | 追加の入力 |
| --- | --- | --- | --- |
| Windows | DPAPI（`CryptProtectData`） | `dpapi` | なし |
| Linux / macOS（パスフレーズあり） | AES-256-GCM + PBKDF2-HMAC-SHA256（20 万回） | `passphrase` | `ODQUOTA_PASSPHRASE` |
| Linux / macOS（パスフレーズなし） | 平文 JSON（0600） | `plain` | なし |

| 項目 | Windows | Linux / macOS |
| --- | --- | --- |
| 保存先 | `%APPDATA%\odquota\accounts.dat` | `$XDG_DATA_HOME/odquota` → `$HOME/.local/share/odquota` |
| ディレクトリ | （OS 既定） | 0700 |
| ファイル | DPAPI ブロブ | 0600 |
| 上書き | `ODQUOTA_HOME` でディレクトリごと差し替え | 同左 |

パスフレーズは環境変数か、保存済みファイルを開くときの入力に使う。**argv では受け取らない。**

- Unix でパスフレーズ無しに暗号化ストアを開くと `保存ファイルはパスフレーズで暗号化されています` で停止する（exit 4）
- 誤ったパスフレーズでは `復号に失敗しました` で停止する（exit 4）
- パスフレーズ無しで新規保存すると `encrypted:false` / `protection:"plain"` を明示する。暗号化済みと偽らない

DPAPI で保存したファイルは他のユーザー・他のマシンへコピーすると復号できない。
パスフレーズ方式のファイルは、パスフレーズを知っていれば別ホストでも開ける。

## 終了コード

| コード | 意味 |
| --- | --- |
| 0 | 要求したものがすべて取れた |
| 1 | 一部のアカウント、または要求した追加ステージが失敗した（JSON は出る） |
| 2 | 引数・設定の不備（JSON は出ない） |
| 3 | 保存済みアカウントが 0 件（stdout は空） |
| 4 | 保存ファイルの破損・復号失敗・入出力エラー（JSON は出ない） |

## エンドポイント

| 用途 | メソッドとパス |
| --- | --- |
| ワークスペース解決 | `GET /api/v1/workspaces` |
| 本人 | `GET /api/v1/me` |
| クオータ本体 | `GET /api/v1/workspaces/{ws}/billing/snapshot` |
| 残高・回数 | `GET /api/v1/billing/summary` |
| モデル可否 | `GET /api/v1/workspaces/{ws}/billing/preflight?modelId=` |
| 自動チャージ | `GET /api/v1/workspaces/{ws}/auto-recharge` |
| 付与モデル（Link） | `GET /v1/models`（runtimeKey + `x-vela-workspace-id`） |
| カタログ | `GET /api/v1/models` |

制御プレーンは `Authorization: Bearer <controlKey>`、Link は `Bearer <runtimeKey>`。
`User-Agent: odquota/<version>` を常に送る。`/api/v1/billing/summary` は Go 既定 UA を
Cloudflare が遮断するため、UA 無しのクライアントは 403 になる。

### 取得できないもの

以下は API 側の制約で、実装していない。`--surfaces` はこの事実をステータスとして記録する。

| エンドポイント | 応答 | 理由 |
| --- | --- | --- |
| `billing/ledger` | 403 `workspace_member_permission_denied` | 権限不足 |
| `billing/usage` | 401 `unauthenticated` | 制御プレーン鍵では通らない別系統の認証が必要 |
| `workspaces/{ws}/members` | 503 `database_unavailable` | サーバー側 DB 障害 |
| `billing/team-subscription/catalog` | 400 `team_workspace_required` | 個人ワークスペースでは不可 |
| `billing/request-concurrency` | 404 | 未実装 |

モデル別のクレジット内訳は API が返さないため取得できない。`usageCount` は回数のみ。

## 複数アカウント

`account add` / `import-amr --label` で複数登録でき、`quota` は既定で全件を 1 つの JSON にまとめる。
アカウントは `controlKey` 単位で閉じており、1 つのキーで他アカウントを引く経路は無い。
2 件目以降は、そのアカウント自身のキーが必要。

## テスト

```
cargo test
```

| 種別 | 内容 |
| --- | --- |
| 単体 | 百分率・リセット秒・window 突き合わせ・欠損を埋めないこと・URL/UA・エラー抽出 |
| crypto | AES-256-GCM の往復、誤鍵・誤 nonce の拒否、PBKDF2 の決定性と salt 敏感性（Unix のみ） |
| store | DPAPI / パスフレーズ往復、**保存バイトに鍵の平文が無いこと**、原子置換の残骸なし、破損検出、version 拒否 |
| mock_api | 200 正常、401 無リトライ、403 の Cloudflare 検出、503 のリトライ、接続拒否、enabled 絞り込み |
| cli | `--version`、空ストア、鍵を argv に取らないこと、e2e、部分失敗で exit 1、`import-amr` の重複拒否 |
| live | 実 API。`ODQUOTA_LIVE_CONTROL_KEY` 未設定なら SKIP を明示して return する |

ライブ検証は明示実行のみ:

```
ODQUOTA_LIVE_CONTROL_KEY=… ODQUOTA_LIVE_RUNTIME_KEY=… cargo test --test live -- --ignored --nocapture
```

モックは標準ライブラリの `TcpListener` だけで組んであり、dev-dependencies は無い。

## ライセンス

MIT
