<!--
docs/*.md の共通テンプレート。各領域ドキュメントはこの構成に従うこと。
- 表は GitHub-flavored Markdown。
- 「全フィールド表」は reference を兼ねるので、調査で判明した全フィールドを省略せず載せる。
- 変換テーブルは mappings/<domain>.yaml と1:1で対応させる（同じ id を併記推奨）。
- 損失記号: ◎ lossless / ○ 形式変換のみ / △ lossy・降格 / ✕ dropped
- 末尾に必ず一次情報の出典 URL を列挙。
-->

# MCP Servers: Claude Code ⇄ Codex

> Claude は `.mcp.json`（JSON, `{"mcpServers":{"name":{...}}}`）、Codex は `config.toml`（`[mcp_servers.<name>]`）で MCP サーバーを設定する。transport 方式・フィールド名・OAuth モデルに大きな差異があり、sse/ws トランスポートや動的ヘッダ生成などの Claude 固有機能は Codex 側で再現不可。

## 0. 概要

MCP サーバー設定は両者に存在するが、設定ファイルの形式・フィールド名・transport 判定ロジックが根本的に異なる。

**共通点**: stdio（command-based）と HTTP（url-based）の2トランスポートをサポート。Bearer 認証、OAuth、ツールフィルタリングの概念が両者に存在する。

**主要な差異**:

| 差異点 | Claude | Codex |
|---|---|---|
| ファイル形式 | JSON (`.mcp.json`) | TOML (`config.toml`) |
| transport 指定 | 明示的 `type` フィールド | `command` 有=stdio、`url` 有=http で暗黙判定 |
| sse/ws transport | 対応（sse は deprecated） | 非対応（dropped） |
| Bearer 認証 | `headers.Authorization: "Bearer ${VAR}"` | `bearer_token_env_var: "VAR"` |
| HTTP ヘッダキー | `headers` | `http_headers`（rename） |
| timeout 単位 | ms（`timeout`） | 秒（`tool_timeout_sec`） |
| 有効/無効 | フィールド削除で無効（`enabled` なし） | `enabled: false`（既定 `true`） |
| OAuth scopes | スペース区切り文字列 | 配列 |
| 動的ヘッダ生成 | `headersHelper`（シェルコマンド実行） | 非対応（dropped） |
| 起動タイムアウト | 非対応（グローバル `MCP_TIMEOUT` 環境変数） | `startup_timeout_sec`（既定 10 秒） |

**変換難易度**: Claude→Codex では sse/ws/`headersHelper`/`alwaysLoad` などが dropped となる。Codex→Claude では `enabled_tools`/`disabled_tools`/`default_tools_approval_mode`/`required`/`startup_timeout_sec` などが dropped となる。双方向の lossless 変換が可能なのは stdio の基本フィールド（command/args/env/cwd/url）と HTTP ヘッダ（rename のみ）に限られる。

## 1. Claude Code 側の仕様

### 配置・ファイル・スコープ

| スコープ | 保存先 | 共有 | 用途 |
|---|---|---|---|
| local（既定） | `~/.claude.json`（プロジェクトパスをキーに格納） | 非共有（個人） | 個人用・実験的設定、秘密情報を含む設定 |
| project | `<project-root>/.mcp.json` | バージョン管理で共有 | チーム共通設定 |
| user | `~/.claude.json`（全プロジェクト共通） | 非共有（個人） | 全プロジェクトで使うツール |

優先順位（高い順）: local > project > user > plugin-provided > claude.ai connectors。同名のサーバーが複数スコープで定義された場合、最高優先スコープのエントリ全体が採用される（フィールドのマージは行われない）。

環境変数展開は `command`/`args`/`env`/`url`/`headers` で有効。構文: `${VAR}` または `${VAR:-default}`。

### 全フィールド表（transport 別）

#### 共通フィールド（全 transport）

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `type` | `"stdio"` / `"http"` / `"streamable-http"` / `"sse"` / `"ws"` | 任意 | stdio (command 有の場合) | transport 種別。`"streamable-http"` は `"http"` の alias。`"sse"` は deprecated |
| `timeout` | number (ms) | 任意 | `MCP_TOOL_TIMEOUT` 環境変数または無制限 | ツール呼び出しごとの wall-clock タイムアウト（ms）。1000ms 未満は 1000ms に切り上げ |
| `alwaysLoad` | boolean | 任意 | `false` | `true` にするとセッション開始時に全ツールをコンテキストにロード（Tool Search による遅延ロードを無効化）。v2.1.121 以降 |
| `oauth` | object | 任意 | — | OAuth 設定オブジェクト（後述） |

#### stdio transport フィールド

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `command` | string | 必須 | — | MCP サーバー起動コマンド。`${VAR}` 展開可 |
| `args` | array\<string\> | 任意 | `[]` | コマンド引数。`${VAR}` 展開可 |
| `env` | object\<string, string\> | 任意 | `{}` | サーバーに渡す環境変数（リテラル値）。`${VAR}` 展開可 |
| `cwd` | string | 任意 | — | サーバープロセスの作業ディレクトリ |

#### http / streamable-http transport フィールド

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `url` | string | 必須 | — | MCP サーバーエンドポイント URL。`${VAR}` 展開可 |
| `headers` | object\<string, string\> | 任意 | — | HTTP リクエストに付加するヘッダ（静的値）。`${VAR}` 展開可 |
| `headersHelper` | string | 任意 | — | 接続時に実行するシェルコマンド。stdout に JSON オブジェクト（文字列 key-value）を出力すると動的ヘッダとして使用される。静的 `headers` と同名の場合は上書き。10 秒タイムアウト |

#### sse transport フィールド（deprecated）

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `url` | string | 必須 | — | SSE エンドポイント URL |
| `headers` | object\<string, string\> | 任意 | — | リクエストヘッダ |
| `headersHelper` | string | 任意 | — | 動的ヘッダ生成コマンド（http と同様） |

> SSE transport は deprecated。可能な限り http（streamable-http）への移行を推奨。

#### ws transport フィールド

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `url` | string | 必須 | — | WebSocket エンドポイント URL（`wss://` 推奨） |
| `headers` | object\<string, string\> | 任意 | — | 接続ヘッダ（静的） |
| `headersHelper` | string | 任意 | — | 動的ヘッダ生成コマンド |

> WebSocket は `claude mcp add --transport` フラグ非対応。`.mcp.json` または `claude mcp add-json` でのみ設定可能。OAuth 非対応。

#### oauth オブジェクト

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `oauth.clientId` | string | 任意 | — | OAuth クライアント ID（事前登録済みクライアント用） |
| `oauth.callbackPort` | number | 任意 | OS 任意選択 | OAuth コールバック用ローカルポート（`http://localhost:PORT/callback`） |
| `oauth.scopes` | string | 任意 | サーバー広告スコープ | 要求する OAuth スコープ（RFC 6749 §3.3 のスペース区切り文字列） |
| `oauth.authServerMetadataUrl` | string | 任意 | 自動探索 | OAuth 認可サーバーメタデータ URL の上書き（`https://` 必須）。v2.1.64 以降 |

## 2. Codex 側の仕様

### 配置・ファイル・スコープ

| スコープ | ファイル | 優先順位 |
|---|---|---|
| user | `~/.codex/config.toml` | 高 |
| project | `.codex/config.toml` | 中（`trust_level: "trusted"` 要） |
| managed | `requirements.toml` / managed config layer | 最高（管理者強制） |

`[mcp_servers.<name>]` の `<name>` がサーバー識別子。transport は `command` フィールドの有無（stdio）か `url` フィールドの有無（streamable HTTP）で自動判定され、`type` フィールドは存在しない。

### 全フィールド表

#### stdio transport フィールド（`command` 有で自動判定）

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `command` | string | stdio 必須 | — | サーバー起動コマンド |
| `args` | array\<string\> | 任意 | `[]` | コマンド引数 |
| `env` | object\<string, string\> | 任意 | — | サーバーに渡す環境変数（リテラル値）。http transport では使用不可 |
| `env_vars` | array\<string \| object\> | 任意 | `[]` | ホスト環境変数の転送リスト。要素は文字列（変数名）または `{name: "VAR", source: "local"|"remote"}`。`source: "remote"` は remote executor 環境向け |
| `cwd` | string (path) | 任意 | — | 作業ディレクトリ。remote stdio（`environment_id` 指定時）では絶対パス必須 |
| `environment_id` | string | 任意 | `"local"` | サーバー実行環境。`"remote"` でリモートエグゼキュータ使用（experimental） |

#### http transport フィールド（`url` 有で自動判定）

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `url` | string | http 必須 | — | MCP サーバーエンドポイント URL |
| `bearer_token_env_var` | string | 任意 | — | Bearer トークンを読み取る環境変数名。セットすると `Authorization: Bearer <token>` ヘッダを自動付与 |
| `http_headers` | object\<string, string\> | 任意 | — | 静的 HTTP ヘッダ（名前→値） |
| `env_http_headers` | object\<string, string\> | 任意 | — | 環境変数から値を取得する HTTP ヘッダ（ヘッダ名→環境変数名） |

#### 共通フィールド（全 transport）

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `enabled` | boolean | 任意 | `true` | `false` で設定を残したままサーバーを無効化 |
| `required` | boolean | 任意 | `false` | `true` の場合、サーバー初期化失敗時に `codex exec` を終了エラーにする |
| `startup_timeout_sec` | number (秒) | 任意 | `10.0` | サーバー起動・初回ツールリスト取得のタイムアウト（秒）。`startup_timeout_ms` も alias として存在 |
| `tool_timeout_sec` | number (秒) | 任意 | `60.0` | ツール呼び出しごとのタイムアウト（秒） |
| `enabled_tools` | array\<string\> | 任意 | — | サーバーが公開するツールのホワイトリスト。セットすると列挙されたツールのみ登録 |
| `disabled_tools` | array\<string\> | 任意 | — | ツールのブラックリスト。`enabled_tools` 適用後に除外 |
| `default_tools_approval_mode` | `"auto"` / `"prompt"` / `"approve"` | 任意 | — | サーバー全ツールのデフォルト承認モード。per-tool override がない場合に適用 |
| `tools.<name>.approval_mode` | `"auto"` / `"prompt"` / `"approve"` | 任意 | — | 特定ツールの承認モード上書き |
| `supports_parallel_tool_calls` | boolean | 任意 | `false` | `true` の場合、このサーバーの全ツールを並列ツール呼び出し安全とマーク |
| `scopes` | array\<string\> | 任意 | — | OAuth ログイン時に要求するスコープ（配列形式） |
| `oauth` | object | 任意 | — | OAuth クライアント設定 |
| `oauth.client_id` | string | 任意 | — | OAuth クライアント ID |
| `oauth_resource` | string | 任意 | — | OAuth Resource Parameter（RFC 8707） |

#### トップレベルフィールド（`config.toml` 直下）

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `mcp_oauth_callback_port` | number | 任意 | OS 任意選択 | OAuth コールバック用ローカルポート（全 MCP サーバー共通） |
| `mcp_oauth_callback_url` | string | 任意 | — | OAuth コールバック URI の上書き（ローカルリスナーは引き続き起動） |
| `mcp_oauth_credentials_store` | `"keyring"` / `"file"` / `"auto"` | 任意 | `"auto"` | OAuth クレデンシャルの保存先 |

## 3. 変換テーブル

`mappings/mcp.yaml` の人間可読版。

| id | Claude フィールド | Codex フィールド | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `mcp.format` | `{"mcpServers":{...}}` (JSON) | `[mcp_servers.<name>]` (TOML) | both | ○ | — | `format:json_to_toml` / `format:toml_to_json` |
| `mcp.transport_type` | `type: "stdio"/"http"/"streamable-http"/"sse"/"ws"` | なし（`command`/`url` で暗黙判定） | both | △ | — | Claude→Codex: `type` フィールドを削除し `command`/`url` で代替。Codex→Claude: `command` 有→`type:"stdio"`、`url` 有→`type:"http"` を補完 |
| `mcp.transport_sse` | `type: "sse"` | なし | claude_to_codex | ✕ | — | Codex は SSE 非対応。`url` ベース設定に変換試行するが SSE プロトコルとの互換性は保証外 |
| `mcp.transport_ws` | `type: "ws"` | なし | claude_to_codex | ✕ | — | Codex は WebSocket 非対応。dropped + warn |
| `mcp.command` | `command` | `command` | both | ◎ | — | 値そのまま。環境変数展開構文は両者とも対応 |
| `mcp.args` | `args` | `args` | both | ◎ | — | 値そのまま |
| `mcp.env` | `env` (object, リテラル値) | `env` (object, リテラル値) | both | ◎ | — | 値そのまま。Codex の `env` は stdio 専用（http transport では使用不可） |
| `mcp.env_vars` | なし | `env_vars` (array) | codex_to_claude | ✕ | — | ホスト環境変数転送リスト。Claude に対応フィールドなし。dropped + warn |
| `mcp.cwd` | `cwd` | `cwd` | both | ◎ | — | 値そのまま。Codex の remote stdio では絶対パス必須 |
| `mcp.url` | `url` | `url` | both | ◎ | — | 値そのまま。環境変数展開対応 |
| `mcp.headers` | `headers` (object) | `http_headers` (object) | both | ○ | — | `rename`: `headers` ↔ `http_headers`。Bearer 認証は `mcp.bearer` で別処理 |
| `mcp.bearer` | `headers.Authorization: "Bearer ${VAR}"` | `bearer_token_env_var: "VAR"` | both | △ | — | `extract:bearer_env`: Bearer トークン用環境変数名を抽出/展開。Claude→Codex: `"Bearer ${VAR}"` から `VAR` を抽出。Codex→Claude: `VAR` を `"Bearer ${VAR}"` として `headers.Authorization` に展開 |
| `mcp.env_http_headers` | なし | `env_http_headers` (object) | codex_to_claude | △ | — | ヘッダ名→環境変数名マッピング。Claude→Codex 変換時は `headers` の `${VAR}` 参照を `env_http_headers` に変換試行（lossy）。Codex→Claude 変換時は `headers` に `${VAR}` 形式で展開 |
| `mcp.timeout` | `timeout` (ms, number) | `tool_timeout_sec` (秒, number) | both | ○ | — | `unit:ms_to_sec` / `unit:sec_to_ms`。例: `60000` ↔ `60.0` |
| `mcp.startup_timeout` | なし | `startup_timeout_sec` (秒, number) | codex_to_claude | ✕ | — | Codex 固有フィールド。Claude では `MCP_TIMEOUT` 環境変数（グローバル）のみ。dropped |
| `mcp.enabled` | なし（フィールド削除で無効） | `enabled` (boolean, 既定 `true`) | codex_to_claude | ✕ | — | Codex→Claude: `enabled: false` のエントリは変換先 `.mcp.json` から除外。Claude→Codex: 常に `enabled: true`（既定値なので省略可） |
| `mcp.alwaysLoad` | `alwaysLoad` (boolean) | なし | claude_to_codex | ✕ | — | Claude 固有（Tool Search 制御）。Codex に対応機構なし。dropped + warn |
| `mcp.headersHelper` | `headersHelper` (string, コマンド) | なし | claude_to_codex | ✕ | — | 動的ヘッダ生成コマンド。Codex に対応なし。dropped + warn（Bearer 認証の場合は `mcp.bearer` での変換を提案） |
| `mcp.oauth.client_id` | `oauth.clientId` | `oauth.client_id` | both | ○ | — | `rename`: `clientId` ↔ `client_id` |
| `mcp.oauth.callback_port` | `oauth.callbackPort` | `mcp_oauth_callback_port`（トップレベル） | both | △ | — | Claude は per-server フィールド。Codex はトップレベル（全サーバー共通）。lossy（複数サーバーが異なる `callbackPort` を持つ場合、最後の値で上書きされる） |
| `mcp.oauth.scopes` | `oauth.scopes` (string, スペース区切り) | `scopes` (array\<string\>) | both | ○ | — | `str_to_list:space` / `list_to_str:space`。例: `"repo read:org"` ↔ `["repo", "read:org"]` |
| `mcp.oauth.auth_server_metadata_url` | `oauth.authServerMetadataUrl` (string) | なし | claude_to_codex | ✕ | — | Claude 固有の OAuth 探索 URL 上書き。Codex に対応なし。dropped + warn |
| `mcp.oauth_resource` | なし | `oauth_resource` (string) | codex_to_claude | ✕ | — | RFC 8707 OAuth Resource Parameter。Claude に対応なし。dropped |
| `mcp.enabled_tools` | なし | `enabled_tools` (array\<string\>) | codex_to_claude | ✕ | — | ツールホワイトリスト。Claude に対応なし。dropped |
| `mcp.disabled_tools` | なし | `disabled_tools` (array\<string\>) | codex_to_claude | ✕ | — | ツールブラックリスト。Claude に対応なし。dropped |
| `mcp.default_tools_approval_mode` | なし | `default_tools_approval_mode` | codex_to_claude | ✕ | — | サーバーレベルの承認モード設定。Claude に対応なし。dropped |
| `mcp.tools_approval_mode` | なし | `tools.<name>.approval_mode` | codex_to_claude | ✕ | — | per-tool 承認モード。Claude に対応なし。dropped |
| `mcp.required` | なし | `required` (boolean) | codex_to_claude | ✕ | — | 起動失敗時の強制終了フラグ。Claude に対応なし。dropped |
| `mcp.supports_parallel_tool_calls` | なし | `supports_parallel_tool_calls` (boolean) | codex_to_claude | ✕ | — | 並列ツール呼び出し安全マーク。Claude に対応なし。dropped |
| `mcp.environment_id` | なし | `environment_id` (string) | codex_to_claude | ✕ | — | サーバー実行環境指定（experimental）。Claude に対応なし。dropped |

## 4. 変換例（JSON ⇄ TOML）

### 例1: stdio 基本（双方向 lossless）

**Claude `.mcp.json`:**
```json
{
  "mcpServers": {
    "context7": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@upstash/context7-mcp"],
      "env": {
        "CACHE_DIR": "/tmp/context7"
      },
      "cwd": "/path/to/project"
    }
  }
}
```

**Codex `config.toml`:**
```toml
[mcp_servers.context7]
command = "npx"
args = ["-y", "@upstash/context7-mcp"]
cwd = "/path/to/project"

[mcp_servers.context7.env]
CACHE_DIR = "/tmp/context7"
```

変換変換規則: `format:json_to_toml`。`type` フィールドは Codex では不要（`command` 有で stdio と暗黙判定）。

---

### 例2: HTTP + Bearer 認証（token 変数名の抽出・展開）

**Claude `.mcp.json`:**
```json
{
  "mcpServers": {
    "github": {
      "type": "http",
      "url": "https://api.githubcopilot.com/mcp/",
      "headers": {
        "Authorization": "Bearer ${GITHUB_PAT}",
        "X-Custom-Header": "static-value"
      },
      "timeout": 30000
    }
  }
}
```

**Codex `config.toml`:**
```toml
[mcp_servers.github]
url = "https://api.githubcopilot.com/mcp/"
bearer_token_env_var = "GITHUB_PAT"
tool_timeout_sec = 30.0

[mcp_servers.github.http_headers]
X-Custom-Header = "static-value"
```

変換規則:
- `headers` → `http_headers`（`rename`）
- `headers.Authorization: "Bearer ${GITHUB_PAT}"` → `bearer_token_env_var = "GITHUB_PAT"`（`extract:bearer_env`）
- `timeout: 30000` → `tool_timeout_sec = 30.0`（`unit:ms_to_sec`）
- `type: "http"` は削除（`url` の有無で暗黙判定）

---

### 例3: OAuth（scopes 型変換 + フィールド rename）

**Claude `.mcp.json`:**
```json
{
  "mcpServers": {
    "slack": {
      "type": "http",
      "url": "https://mcp.slack.com/mcp",
      "oauth": {
        "clientId": "A0123456789",
        "callbackPort": 8080,
        "scopes": "channels:read chat:write search:read"
      }
    }
  }
}
```

**Codex `config.toml`:**
```toml
# トップレベル（per-server フィールドではない）
mcp_oauth_callback_port = 8080

[mcp_servers.slack]
url = "https://mcp.slack.com/mcp"
scopes = ["channels:read", "chat:write", "search:read"]

[mcp_servers.slack.oauth]
client_id = "A0123456789"
```

変換規則:
- `oauth.clientId` → `oauth.client_id`（`rename`）
- `oauth.callbackPort` → トップレベル `mcp_oauth_callback_port`（フィールド移動、lossy: 複数サーバー設定時は衝突の恐れ）
- `oauth.scopes: "channels:read chat:write search:read"` → `scopes = ["channels:read", "chat:write", "search:read"]`（`str_to_list:space`）

---

### 例4: 動的ヘッダ生成（`headersHelper` → Codex で dropped）

**Claude `.mcp.json`:**
```json
{
  "mcpServers": {
    "internal-api": {
      "type": "http",
      "url": "https://mcp.internal.example.com",
      "headersHelper": "/opt/bin/get-mcp-auth-headers.sh",
      "alwaysLoad": true
    }
  }
}
```

**Codex `config.toml`:**
```toml
# [WARNING] headersHelper は Codex 非対応。dropped。
# 変換エンジンは Bearer 認証として再現できないか確認を促すこと。
# [WARNING] alwaysLoad は Codex 非対応。dropped。
[mcp_servers.internal-api]
url = "https://mcp.internal.example.com"
# bearer_token_env_var = "<ENV_VAR>"  # headersHelper を Bearer 形式に手動変換した場合
```

変換時の警告事項:
- `headersHelper`: Codex は動的ヘッダ生成をサポートしない（dropped）。Bearer トークンの場合は `bearer_token_env_var` への手動変換を推奨。Kerberos/SSO など他の認証方式は代替手段なし。
- `alwaysLoad`: Codex に Tool Search 制御機構がないため dropped。

---

### 例5: Codex 固有フィールドを含む設定（Codex → Claude 変換）

**Codex `config.toml`:**
```toml
[mcp_servers.figma]
url = "https://mcp.figma.com/mcp"
bearer_token_env_var = "FIGMA_OAUTH_TOKEN"
enabled = true
required = true
startup_timeout_sec = 15.0
tool_timeout_sec = 120.0
enabled_tools = ["read_file", "search_components"]
disabled_tools = ["delete_file"]
default_tools_approval_mode = "prompt"
scopes = ["files:read"]
oauth_resource = "https://api.figma.com/"
supports_parallel_tool_calls = true
```

**Claude `.mcp.json`:**
```json
{
  "mcpServers": {
    "figma": {
      "type": "http",
      "url": "https://mcp.figma.com/mcp",
      "headers": {
        "Authorization": "Bearer ${FIGMA_OAUTH_TOKEN}"
      },
      "timeout": 120000,
      "oauth": {
        "scopes": "files:read"
      }
    }
  }
}
```

変換時の dropped フィールド（変換レポートに列挙必須）:
- `required: true` → dropped（Claude 非対応）
- `startup_timeout_sec: 15.0` → dropped（Claude はグローバル `MCP_TIMEOUT` 環境変数のみ）
- `enabled_tools: [...]` → dropped（Claude 非対応）
- `disabled_tools: [...]` → dropped（Claude 非対応）
- `default_tools_approval_mode: "prompt"` → dropped（Claude 非対応）
- `oauth_resource: "..."` → dropped（Claude 非対応）
- `supports_parallel_tool_calls: true` → dropped（Claude 非対応）
- `enabled: true` → 省略（Claude では `enabled` フィールド自体が存在しないため、エントリの存在 = 有効）

## 4. 変換時の注意・既知の落とし穴

1. **transport 自動判定の競合**: Codex は `command` と `url` が同時に存在するとエラー（`invalid transport`）。Claude のエントリに誤って両方設定されている場合は変換前に要修正。

2. **sse/ws → Codex 変換不可**: `type: "sse"` および `type: "ws"` は Codex に対応トランスポートがない。HTTP エンドポイントが存在する場合は `url` ベース設定への手動移行を提案すること。

3. **Bearer 認証のセマンティクス差**: Claude の `headers.Authorization` は `${VAR}` 形式で任意の値を渡せるが、Codex の `bearer_token_env_var` は環境変数名のみを受け取り `Authorization: Bearer <value>` ヘッダを自動生成する。`"Bearer ${VAR}"` 以外の Authorization ヘッダ（例: `"Token ${VAR}"`）は Codex 側で再現不可（`http_headers.Authorization` に `"Token ${VAR}"` として渡すか手動対応）。

4. **`env` と `env_vars` のセマンティクス差**: Claude の `env` はリテラル値をサーバーの環境変数に直接セット。Codex の `env` も同様だが、`env_vars` は「ホスト環境変数を転送する許可リスト」であり意味が異なる（lossy）。`env: {KEY: "${HOST_VAR}"}` のように Claude 側で変数参照を使っている場合、Codex への変換では `env_vars: ["HOST_VAR"]` に移行が望ましい。

5. **OAuth `callbackPort` のスコープ差**: Claude の `oauth.callbackPort` は per-server フィールドだが、Codex の `mcp_oauth_callback_port` はトップレベル（全サーバー共通）。複数サーバーが異なる `callbackPort` を要求する場合、Codex では表現不可能（最後の値が残る）。

6. **`timeout` 単位変換の精度**: Claude は ms 整数、Codex は秒の浮動小数点。`unit:ms_to_sec` 変換時は `60000 → 60.0` のように正確に変換されるが、1000ms 未満の値は Claude 内部で 1000ms に切り上げられるため Codex 変換後も `1.0` 秒以上になる。

7. **`enabled: false` の除外**: Codex の `enabled: false` エントリは Claude 側の `.mcp.json` に出力しない（Claude には `enabled` フィールドがなく、エントリが存在することが「有効」を意味するため）。変換レポートに除外したサーバー名を列挙すること。

8. **`headersHelper` の代替なし**: Kerberos・短命トークン・内部 SSO などの認証方式は Codex で再現不可能。変換エンジンは dropped として警告し、可能な場合（Bearer 形式）は `bearer_token_env_var` への移行を提案すること。

9. **`authServerMetadataUrl` の dropped**: Claude v2.1.64 以降で使用可能な OAuth 探索 URL 上書き機能は Codex に対応なし。OAuth フローが正常動作しない場合は手動確認が必要。

10. **Codex の `env_http_headers`**: ヘッダ名→環境変数名のマッピング（例: `{"X-Auth": "AUTH_ENV"}` → `X-Auth: <value of $AUTH_ENV>`）。Claude→Codex 変換では `headers` 内の `${VAR}` 参照（例: `X-Auth: "${AUTH_ENV}"`）を `env_http_headers: {X-Auth: "AUTH_ENV"}` に変換可能だが、変数参照でない静的値は `http_headers` に残す。

11. **`env_vars` の `source: "remote"`**: Codex の remote executor 向け機能。通常のローカル変換では無視して `env` への変換を試みるか dropped とする。

## 5. 出典

- Claude Code MCP ドキュメント: https://code.claude.com/docs/en/mcp
- Codex MCP 設定リファレンス（ウェブ）: https://developers.openai.com/codex/config-reference
- Codex MCP 設定ドキュメント（ウェブ）: https://developers.openai.com/codex/mcp
- Codex MCP 型定義（Rust ソース）: https://github.com/openai/codex/blob/main/codex-rs/config/src/mcp_types.rs
- Codex config.toml 型定義（Rust ソース）: https://github.com/openai/codex/blob/main/codex-rs/config/src/config_toml.rs
- MCP Streamable HTTP 仕様: https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#streamable-http
- MCP stdio 仕様: https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#stdio
