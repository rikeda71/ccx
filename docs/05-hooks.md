<!--
docs/*.md の共通テンプレートに準拠。
損失記号: ◎ lossless / ○ 形式変換のみ / △ lossy・降格 / ✕ dropped
-->

# Hooks: Claude Code ⇄ Codex

> 両者ともライフサイクル Hook をサポートするが、Claude は 30 イベント・4 hook タイプを持つのに対し Codex は 10 イベント・`command` のみ（`prompt`/`agent` は parse のみで実行不可）。共通 10 イベントは形式変換のみで相互変換可能。Claude 固有 20 イベントおよび `http`/`mcp_tool` タイプは Codex に対応物がなく dropped となる。

## 0. 概要

Claude Code と Codex はどちらも「シェルコマンドをライフサイクルの特定時点に割り込ませる」という Hook の基本思想を共有する。しかし Claude が 30 イベント・4 タイプを持つのに対し、Codex はソースコードで確認できる実装イベントが 10 種類・実行可能タイプが `command` のみという非対称な状況にある。

設定フォーマットも異なる。Claude は `settings.json`（または `hooks.json`）に JSON で記述するのに対し、Codex は `config.toml` に TOML の array-of-tables 記法（`[[hooks.EventName]]`）で記述する。入力 JSON はほぼ共通だが、Codex には `turn_id` / `model` が追加され、ツール実行結果フィールドは Claude が `tool_output` と呼ぶのに対し Codex は `tool_response` と呼ぶ（rename、lossy）。出力 JSON はどちらも `hookSpecificOutput` ネストを使うが、Codex は一部フラット形式（`decision: "block"` など）も受け付ける。

変換の難易度は「共通 10 イベントの `command` hook → 低（形式変換のみ）」「Claude 固有イベントや http/mcp_tool タイプ → 高（dropped）」と大きく二分される。

---

## 1. Claude Code 側の仕様

### 配置・ファイル・スコープ

| ファイルパス | スコープ | 共有可否 |
|---|---|---|
| `~/.claude/settings.json` の `hooks` キー | ユーザー全プロジェクト | 不可（ローカル） |
| `.claude/settings.json` の `hooks` キー | 単一プロジェクト | 可（リポジトリコミット） |
| `.claude/settings.local.json` の `hooks` キー | 単一プロジェクト | 不可（gitignore） |
| プラグイン `hooks/hooks.json` | プラグイン有効時 | 可（プラグイン同梱） |
| スキル/エージェント frontmatter | コンポーネント有効中 | 可（コンポーネント内） |
| マネージドポリシー設定 | 組織全体 | 可（管理者制御） |

設定構造:
```json
{
  "hooks": {
    "EventName": [
      {
        "matcher": "pattern",
        "hooks": [
          { "type": "command", "command": "script.sh" }
        ]
      }
    ]
  }
}
```

### Claude Hook イベント全 30 件（ブロック可否・Codex 対応有無）

| イベント | カテゴリ | exit 2 でブロック | Codex 対応 |
|---|---|---|---|
| `SessionStart` | セッション | 不可（stderr 表示のみ） | あり（共通） |
| `Setup` | セッション | 不可 | なし（Claude 固有） |
| `UserPromptSubmit` | ターン | 可 | あり（共通） |
| `UserPromptExpansion` | ターン | 可 | なし（Claude 固有） |
| `PreToolUse` | ツールループ | 可 | あり（共通） |
| `PermissionRequest` | ツールループ | 可 | あり（共通） |
| `PermissionDenied` | ツールループ | 不可 | なし（Claude 固有） |
| `PostToolUse` | ツールループ | 不可 | あり（共通） |
| `PostToolUseFailure` | ツールループ | 不可 | なし（Claude 固有） |
| `PostToolBatch` | ツールループ | 可 | なし（Claude 固有） |
| `Notification` | 非同期 | 不可 | なし（Claude 固有） |
| `MessageDisplay` | 非同期 | 不可 | なし（Claude 固有） |
| `SubagentStart` | サブエージェント | 不可 | あり（共通） |
| `SubagentStop` | サブエージェント | 可 | あり（共通） |
| `TaskCreated` | タスク | 可 | なし（Claude 固有） |
| `TaskCompleted` | タスク | 可 | なし（Claude 固有） |
| `Stop` | ターン終了 | 可 | あり（共通） |
| `StopFailure` | ターン終了 | 不可（出力・exit code 無視） | なし（Claude 固有） |
| `TeammateIdle` | チーム | 可 | なし（Claude 固有） |
| `InstructionsLoaded` | 設定 | 不可 | なし（Claude 固有） |
| `ConfigChange` | 設定 | 可 | なし（Claude 固有） |
| `CwdChanged` | ファイルシステム | 不可 | なし（Claude 固有） |
| `FileChanged` | ファイルシステム | 不可 | なし（Claude 固有） |
| `WorktreeCreate` | Worktree | 可 | なし（Claude 固有） |
| `WorktreeRemove` | Worktree | 不可 | なし（Claude 固有） |
| `PreCompact` | コンパクション | 可 | あり（共通） |
| `PostCompact` | コンパクション | 不可 | あり（共通） |
| `Elicitation` | MCP | 可 | なし（Claude 固有） |
| `ElicitationResult` | MCP | 可 | なし（Claude 固有） |
| `SessionEnd` | セッション | 不可 | なし（Claude 固有） |

### Claude Hook タイプ全フィールド表

**共通フィールド（全タイプ）**

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `type` | `"command"\|"http"\|"mcp_tool"\|"prompt"\|"agent"` | 必須 | — | hook 実行方式 |
| `timeout` | integer（秒） | 任意 | command/http/mcp_tool: 600、UserPromptSubmit 時は 30、prompt: 30、agent: 60 | タイムアウト秒数 |
| `statusMessage` | string | 任意 | null | スピナー表示メッセージ |
| `once` | boolean | 任意 | false | スキル/エージェント内でセッション 1 回のみ実行 |
| `if` | string | 任意 | null | ツールイベントのみ有効。パーミッションルール構文でフィルタ（v2.1.85+） |

**`command` タイプ追加フィールド**

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `command` | string | 必須 | — | 実行するシェルコマンド（shell form）または実行ファイルパス（exec form） |
| `args` | string[] | 任意 | null | 指定時は exec form。`command` を実行ファイルとして spawn し、args を引数として渡す（シェルを経由しない） |
| `shell` | `"bash"\|"powershell"` | 任意 | bash（macOS/Linux）/ powershell（Windows） | shell form 時のシェル指定 |
| `async` | boolean | 任意 | false | バックグラウンド実行（ブロックしない） |
| `asyncRewake` | boolean | 任意 | false | バックグラウンド実行かつ exit 2 で Claude を再起動 |

**`http` タイプ追加フィールド**

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `url` | string | 必須 | — | POST 先エンドポイント URL |
| `headers` | object | 任意 | null | リクエストヘッダー（`$VAR` 展開可） |
| `allowedEnvVars` | string[] | 任意 | [] | ヘッダー内で展開を許可する環境変数名のリスト |

**`mcp_tool` タイプ追加フィールド**

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `server` | string | 必須 | — | 接続済み MCP サーバー名 |
| `tool` | string | 必須 | — | ツール名 |
| `input` | object | 任意 | null | ツール引数（`${path}` 置換あり） |

**`prompt` タイプ追加フィールド**

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `prompt` | string | 必須 | — | LLM に送るプロンプト（`$ARGUMENTS` プレースホルダ） |
| `model` | string | 任意 | 高速モデル（Haiku 系） | 使用するモデル ID |

**`agent` タイプ追加フィールド**

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `prompt` | string | 必須 | — | サブエージェントへの指示（`$ARGUMENTS` プレースホルダ） |
| `timeout` | integer（秒） | 任意 | 60 | タイムアウト（最大 50 ツール呼び出しターン） |

### Claude Matcher 仕様

| matcher 値 | 評価方法 | 例 | マッチ対象 |
|---|---|---|---|
| `*`、`""`、省略 | 全マッチ | — | 全イベント |
| 英数字・`_`・`\|` のみ | exact or list | `Bash`、`Edit\|Write` | ツール名完全一致・OR |
| その他の文字を含む | JavaScript regex | `^mcp__.*`、`mcp__.*__write.*` | パターンマッチ |

イベントごとのマッチ対象フィールドは§3変換テーブルの注記を参照。

### Claude 入力 JSON（hook stdin）

全イベント共通:

| フィールド | 型 | 説明 |
|---|---|---|
| `session_id` | string | セッション識別子 |
| `transcript_path` | string | トランスクリプト JSONL ファイルパス |
| `cwd` | string | カレントディレクトリ |
| `hook_event_name` | string | イベント名 |
| `permission_mode` | string | `default\|plan\|acceptEdits\|auto\|dontAsk\|bypassPermissions` |
| `effort` | object | `{ "level": "low\|medium\|high\|xhigh\|max" }` |
| `agent_id` | string? | サブエージェント ID（サブエージェント内のみ） |
| `agent_type` | string? | サブエージェント種別 |

ツール系イベント追加フィールド（Claude）:

| フィールド | 型 | 説明 |
|---|---|---|
| `tool_name` | string | ツール名 |
| `tool_input` | object | ツール引数 |
| `tool_output` | object | ツール実行結果（PostToolUse） |

### Claude 出力 JSON（hook stdout、exit 0 時）

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `continue` | boolean | true | false でセッション停止 |
| `stopReason` | string | null | `continue: false` 時のメッセージ |
| `suppressOutput` | boolean | false | hook の stdout をトランスクリプトから隠す |
| `systemMessage` | string | null | ユーザーへの警告メッセージ |
| `terminalSequence` | string | null | ターミナルエスケープシーケンス（OSC 0/1/2/9/99/777、BEL） |
| `hookSpecificOutput` | object | null | イベント固有の決定（下記） |

`hookSpecificOutput` の代表例:

**PreToolUse（`hookSpecificOutput.hookEventName: "PreToolUse"`）**

| フィールド | 型 | 説明 |
|---|---|---|
| `permissionDecision` | `"allow"\|"deny"\|"ask"\|"defer"` | ツール実行の許可/拒否/昇格/defer |
| `permissionDecisionReason` | string | 理由（Claude へのフィードバック） |
| `updatedInput` | object | ツール引数の書き換え（並列フック時は最後のものが優先） |
| `additionalContext` | string | Claude へ渡す追加コンテキスト |

**SessionStart（`hookSpecificOutput.hookEventName: "SessionStart"`）**

| フィールド | 型 | 説明 |
|---|---|---|
| `additionalContext` | string | Claude へ渡す追加コンテキスト |
| `initialUserMessage` | string | 非インタラクティブ時の最初のユーザーメッセージ |
| `sessionTitle` | string | セッション名 |
| `watchPaths` | string[] | 監視する絶対パスのリスト |
| `reloadSkills` | boolean | スキルディレクトリを再スキャンするか |

**PermissionRequest（`hookSpecificOutput.hookEventName: "PermissionRequest"`）**

| フィールド | 型 | 説明 |
|---|---|---|
| `decision.behavior` | `"allow"\|"deny"` | 許可/拒否 |
| `decision.updatedInput` | object | 入力書き換え（将来の機能） |
| `decision.updatedPermissions` | array | パーミッションモード変更 |
| `decision.addPermissionRules` | string[] | パーミッションルールの追加 |

### 環境変数

| 変数 | 説明 |
|---|---|
| `CLAUDE_PROJECT_DIR` | プロジェクトルートパス |
| `CLAUDE_PLUGIN_ROOT` | プラグインインストールディレクトリ |
| `CLAUDE_PLUGIN_DATA` | プラグイン永続データディレクトリ |
| `CLAUDE_ENV_FILE` | 書き込み可能なファイルパス（Bash 実行前のプリアンブル）。SessionStart/Setup/CwdChanged/FileChanged で利用可 |
| `CLAUDE_EFFORT` | effortレベル文字列（`low\|medium\|high\|xhigh\|max`） |
| `CLAUDE_CODE_REMOTE` | Web 環境では `"true"` |

---

## 2. Codex 側の仕様

### 配置・ファイル・スコープ

| ファイルパス | スコープ | 共有可否 |
|---|---|---|
| `~/.codex/hooks.json` の `hooks` キー | ユーザー全プロジェクト | 不可（ローカル） |
| `<repo>/.codex/hooks.json` の `hooks` キー | 単一プロジェクト | 可（リポジトリコミット）※プロジェクト trust 要 |
| `config.toml` の `[hooks]` セクション | ユーザーまたはプロジェクト | 可 |
| `requirements.toml` の `[hooks]` セクション | 組織全体（マネージド） | 可（管理者制御） |

有効化: `features.hooks` はデフォルト有効。無効化する場合のみ `[features] hooks = false` を記述する。`features.codex_hooks` は deprecated alias。Claude→Codex 変換時に `config.toml` へ追記する場合、hooks は省略（デフォルト有効のまま）で構わない。

設定構造（TOML）:
```toml
[[hooks.EventName]]
matcher = "^Bash$"

[[hooks.EventName.hooks]]
type = "command"
command = '/absolute/path/to/script.py'
timeout = 30
statusMessage = "Checking..."
```

設定構造（JSON `hooks.json`）:
```json
{
  "hooks": {
    "EventName": [
      {
        "matcher": "^Bash$",
        "hooks": [
          {
            "type": "command",
            "command": "/absolute/path/to/script.py",
            "timeout": 30,
            "statusMessage": "Checking..."
          }
        ]
      }
    ]
  }
}
```

### Codex Hook イベント全 10 件

| イベント | 説明 | exit 2 でブロック |
|---|---|---|
| `SessionStart` | セッション開始（startup/resume/clear/compact） | 不可 |
| `UserPromptSubmit` | ユーザーがプロンプト送信 | 可 |
| `PreToolUse` | ツール実行前 | 可 |
| `PermissionRequest` | 承認ダイアログ表示前 | 可 |
| `PostToolUse` | ツール実行成功後 | 不可 |
| `PreCompact` | コンパクション実行前 | 可 |
| `PostCompact` | コンパクション完了後 | 不可 |
| `SubagentStart` | サブエージェント起動時 | 不可 |
| `SubagentStop` | サブエージェント終了時 | 可 |
| `Stop` | 会話終了時 | 可 |

### Codex Hook タイプ全フィールド表

Codex が実際に実行できるタイプは `command` のみ。`prompt` と `agent` はスキーマ上 parse されるが実行されない。

**`command` タイプ（実行可能）**

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `type` | `"command"` | 必須 | — | hook タイプ |
| `command` | string | 必須 | — | 実行コマンド（POSIX/macOS/Linux） |
| `commandWindows` / `command_windows` | string | 任意 | null | Windows 専用コマンド上書き（`commandWindows` または TOML alias `command_windows`） |
| `timeout` | integer（秒） | 任意 | null（未指定時はデフォルト） | タイムアウト秒数 |
| `statusMessage` | string | 任意 | null | スピナー表示メッセージ |
| `async` | boolean | 任意 | false | バックグラウンド実行 |

**`prompt` タイプ（parse のみ・実行不可）**

スキーマ上は `{ "type": "prompt" }` が受け付けられるが、実行エンジンでは no-op。

**`agent` タイプ（parse のみ・実行不可）**

スキーマ上は `{ "type": "agent" }` が受け付けられるが、実行エンジンでは no-op。

### Codex Matcher 仕様

Codex は常に **regex** として評価する（`*` ワイルドカードは使用不可）。完全一致には `^EventName$` のように記述する。

### Codex 入力 JSON（hook stdin）

全イベント共通:

| フィールド | 型 | 説明 |
|---|---|---|
| `session_id` | string | セッション識別子 |
| `transcript_path` | string\|null | トランスクリプトパス（nullable） |
| `cwd` | string | カレントディレクトリ |
| `hook_event_name` | string | イベント名 |
| `model` | string | 使用中のモデル名 |
| `permission_mode` | string | `default\|acceptEdits\|plan\|dontAsk\|bypassPermissions` |
| `turn_id` | string | ターンスコープのイベントで追加（Codex 独自拡張） |
| `agent_id` | string? | サブエージェント ID |
| `agent_type` | string? | サブエージェント種別 |

ツール系イベント追加フィールド（Codex）:

| フィールド | 型 | 説明 |
|---|---|---|
| `tool_name` | string | ツール名 |
| `tool_input` | object | ツール引数 |
| `tool_response` | object | ツール実行結果（PostToolUse）※Claude の `tool_output` に相当 |
| `tool_use_id` | string | ツール呼び出し識別子（PreToolUse/PostToolUse）。Claude にはない |

SessionStart 追加:

| フィールド | 型 | 説明 |
|---|---|---|
| `source` | string | `startup\|resume\|clear\|compact` |

Stop/SubagentStop 追加:

| フィールド | 型 | 説明 |
|---|---|---|
| `stop_hook_active` | boolean | 前回の Stop hook がブロックしたか（無限ループ防止） |
| `last_assistant_message` | string\|null | 最後のアシスタントメッセージ |

SubagentStop 追加:

| フィールド | 型 | 説明 |
|---|---|---|
| `agent_transcript_path` | string\|null | サブエージェント固有のトランスクリプトパス |

### Codex 出力 JSON（hook stdout、exit 0 時）

共通フィールド（flat 構造、camelCase）:

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `continue` | boolean | true | false でセッション停止 |
| `stopReason` | string | null | `continue: false` 時のメッセージ |
| `suppressOutput` | boolean | false | hook stdout を非表示 |
| `systemMessage` | string | null | ユーザーへの警告メッセージ |

ブロック用（PostToolUse/UserPromptSubmit/Stop/SubagentStop でフラット可）:

| フィールド | 型 | 説明 |
|---|---|---|
| `decision` | `"block"` | ブロック指示（`reason` と併用推奨） |
| `reason` | string | ブロック理由 |

`hookSpecificOutput` ネスト（Claude と共通の構造で受け付ける）:

- `PreToolUse`: `permissionDecision`（`allow`/`deny`/`ask`）、`permissionDecisionReason`、`updatedInput`、`additionalContext`
- `PermissionRequest`: `decision.behavior`（`allow`/`deny`）、`decision.message`
- `PostToolUse`: `additionalContext`、`updatedMCPToolOutput`
- `SessionStart` / `SubagentStart` / `UserPromptSubmit`: `additionalContext`

### 環境変数

Codex は hook 実行時に以下の環境変数を後方互換目的で設定する（Claude との互換維持のため）。

| 変数 | 説明 |
|---|---|
| `CLAUDE_PLUGIN_ROOT` | プラグインインストールディレクトリ（後方互換） |
| `CLAUDE_PLUGIN_DATA` | プラグイン永続データディレクトリ（後方互換） |

上記以外の Claude 環境変数（`CLAUDE_PROJECT_DIR`、`CLAUDE_ENV_FILE`、`CLAUDE_EFFORT` 等）は Codex では設定されない。フック入力の主体は stdin JSON。

---

## 3. 変換テーブル

`mappings/hooks.yaml` の人間可読版。

### 3-1. 設定フォーマット

| id | Claude | Codex | 方向 | 損失 | 書式変換・注記 |
|---|---|---|---|---|---|
| `hooks.config_format` | `settings.json` / `hooks.json`（JSON） | `config.toml`（TOML）または `hooks.json`（JSON） | both | ○ | `format:json_to_toml`。Codex は JSON でも受け付けるため JSON→JSON はそのまま |
| `hooks.features_opt_in` | 常時有効（設定不要） | `[features] hooks`（デフォルト有効。`false` で無効化。省略時は有効） | codex_to_claude | △ | Codex 側のみ存在するキー。Claude→Codex 変換時は省略可（デフォルト有効のため追記不要）。明示的に有効化したい場合のみ `hooks = true` を追記 |

### 3-2. イベント対応

**共通 10 イベント（相互変換可）**

| id | Claude イベント | Codex イベント | 方向 | 損失 | 書式変換・注記 |
|---|---|---|---|---|---|
| `hooks.event.SessionStart` | `SessionStart` | `SessionStart` | both | ○ | 形式変換のみ |
| `hooks.event.UserPromptSubmit` | `UserPromptSubmit` | `UserPromptSubmit` | both | ○ | 形式変換のみ |
| `hooks.event.PreToolUse` | `PreToolUse` | `PreToolUse` | both | ○ | 形式変換のみ |
| `hooks.event.PermissionRequest` | `PermissionRequest` | `PermissionRequest` | both | ○ | 形式変換のみ |
| `hooks.event.PostToolUse` | `PostToolUse` | `PostToolUse` | both | ○ | 形式変換のみ |
| `hooks.event.PreCompact` | `PreCompact` | `PreCompact` | both | ○ | 形式変換のみ |
| `hooks.event.PostCompact` | `PostCompact` | `PostCompact` | both | ○ | 形式変換のみ |
| `hooks.event.SubagentStart` | `SubagentStart` | `SubagentStart` | both | ○ | 形式変換のみ |
| `hooks.event.SubagentStop` | `SubagentStop` | `SubagentStop` | both | ○ | 形式変換のみ |
| `hooks.event.Stop` | `Stop` | `Stop` | both | ○ | 形式変換のみ |

**Claude 固有 20 イベント（Codex へ変換不可）**

| id | Claude イベント | 方向 | 損失 | 注記 |
|---|---|---|---|---|
| `hooks.event.Setup` | `Setup` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.UserPromptExpansion` | `UserPromptExpansion` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.PermissionDenied` | `PermissionDenied` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.PostToolUseFailure` | `PostToolUseFailure` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.PostToolBatch` | `PostToolBatch` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.Notification` | `Notification` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.MessageDisplay` | `MessageDisplay` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.TaskCreated` | `TaskCreated` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.TaskCompleted` | `TaskCompleted` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.StopFailure` | `StopFailure` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.TeammateIdle` | `TeammateIdle` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.InstructionsLoaded` | `InstructionsLoaded` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.ConfigChange` | `ConfigChange` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.CwdChanged` | `CwdChanged` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.FileChanged` | `FileChanged` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.WorktreeCreate` | `WorktreeCreate` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.WorktreeRemove` | `WorktreeRemove` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.Elicitation` | `Elicitation` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.ElicitationResult` | `ElicitationResult` | claude_to_codex | ✕ | Codex 未対応 |
| `hooks.event.SessionEnd` | `SessionEnd` | claude_to_codex | ✕ | Codex 未対応 |

### 3-3. Hook タイプ対応

| id | Claude タイプ | Codex タイプ | 方向 | 損失 | 注記 |
|---|---|---|---|---|---|
| `hooks.type.command` | `command` | `command` | both | ○ | 形式変換のみ（フィールドの lossy/dropped は下表参照） |
| `hooks.type.http` | `http` | なし | claude_to_codex | ✕ | Codex 未対応。dropped |
| `hooks.type.mcp_tool` | `mcp_tool` | なし | claude_to_codex | ✕ | Codex 未対応。dropped |
| `hooks.type.prompt` | `prompt`（実行可） | `prompt`（parse のみ・実行不可） | claude_to_codex | ✕ | Codex は schema 上 parse するが実行しない。実質 dropped |
| `hooks.type.agent` | `agent`（実行可） | `agent`（parse のみ・実行不可） | claude_to_codex | ✕ | Codex は schema 上 parse するが実行しない。実質 dropped |

### 3-4. `command` タイプのフィールド対応

| id | Claude フィールド | Codex フィールド | 方向 | 損失 | 書式変換・注記 |
|---|---|---|---|---|---|
| `hooks.command.type` | `type: "command"` | `type: "command"` | both | ◎ | lossless |
| `hooks.command.command` | `command` | `command` | both | ◎ | lossless |
| `hooks.command.timeout` | `timeout`（秒） | `timeout`（秒） | both | ◎ | lossless |
| `hooks.command.statusMessage` | `statusMessage` | `statusMessage` | both | ◎ | lossless |
| `hooks.command.commandWindows` | なし（`shell: "powershell"` で代替） | `commandWindows` / `command_windows` | codex_to_claude | △ | Claude→Codex: shell form で `command` を PowerShell コマンドとして記述するか分岐が必要。lossy |
| `hooks.command.args` | `args` | なし | claude_to_codex | ✕ | Codex は exec form 未対応。dropped（コマンドを shell form に合成して変換） |
| `hooks.command.shell` | `shell` | なし | claude_to_codex | ✕ | Codex は常に OS デフォルトシェル。dropped |
| `hooks.command.if` | `if` | なし | claude_to_codex | ✕ | Codex は matcher のみ。dropped |
| `hooks.command.once` | `once` | なし | claude_to_codex | ✕ | Codex 未対応。dropped |
| `hooks.command.async` | `async` | `async` | both | ◎ | lossless |
| `hooks.command.asyncRewake` | `asyncRewake` | なし | claude_to_codex | ✕ | Codex 未対応。dropped |

### 3-5. Matcher 対応

| id | Claude matcher | Codex matcher | 方向 | 損失 | 書式変換・注記 |
|---|---|---|---|---|---|
| `hooks.matcher.regex` | regex（その他文字を含む時に regex 評価） | 常に regex | both | ○ | Claude の regex は Codex でそのまま使用可 |
| `hooks.matcher.exact` | 英数字・`_`・`\|` のみ → exact/list 評価 | regex として評価（`^Bash$` 等に変換要） | both | △ | Claude→Codex: `"Bash"` は `"^Bash$"` に変換。`"Edit\|Write"` は `"^(Edit\|Write)$"` に変換 |
| `hooks.matcher.wildcard` | `*`、`""`、省略 → 全マッチ | `*` 不可。省略/`""` は全マッチ | both | △ | Claude の `*` は Codex では削除または `".*"` に変換 |

### 3-6. 入力 JSON フィールド対応

| id | Claude フィールド | Codex フィールド | 方向 | 損失 | 書式変換・注記 |
|---|---|---|---|---|---|
| `hooks.input.session_id` | `session_id` | `session_id` | both | ◎ | lossless |
| `hooks.input.transcript_path` | `transcript_path` | `transcript_path`（nullable） | both | ◎ | lossless |
| `hooks.input.cwd` | `cwd` | `cwd` | both | ◎ | lossless |
| `hooks.input.hook_event_name` | `hook_event_name` | `hook_event_name` | both | ◎ | lossless |
| `hooks.input.permission_mode` | `permission_mode` | `permission_mode` | both | ◎ | lossless |
| `hooks.input.effort` | `effort` オブジェクト | なし | claude_to_codex | ✕ | Codex は環境変数非使用・入力 JSON のみ。dropped |
| `hooks.input.model` | なし | `model` | codex_to_claude | ✕ | Claude は環境変数 `CLAUDE_MODEL` 等で得る。dropped |
| `hooks.input.turn_id` | なし | `turn_id` | codex_to_claude | ✕ | Codex 独自拡張。dropped |
| `hooks.input.tool_name` | `tool_name` | `tool_name` | both | ◎ | lossless |
| `hooks.input.tool_input` | `tool_input` | `tool_input` | both | ◎ | lossless |
| `hooks.input.tool_output` | `tool_output`（PostToolUse） | `tool_response`（PostToolUse） | both | △ | キー名が異なる。`rename`。コンテンツ意味は同等だが名前不一致で lossy |
| `hooks.input.tool_use_id` | なし | `tool_use_id` | codex_to_claude | ✕ | Codex 独自フィールド。dropped |
| `hooks.input.agent_id` | `agent_id` | `agent_id` | both | ◎ | lossless |
| `hooks.input.agent_type` | `agent_type` | `agent_type` | both | ◎ | lossless |
| `hooks.input.stop_hook_active` | `stop_hook_active`（Stop） | `stop_hook_active`（Stop/SubagentStop） | both | ◎ | lossless |
| `hooks.input.last_assistant_message` | なし（非公式） | `last_assistant_message`（Stop/SubagentStop） | codex_to_claude | ✕ | Codex 独自。dropped |

### 3-7. 出力 JSON フィールド対応

| id | Claude フィールド | Codex フィールド | 方向 | 損失 | 書式変換・注記 |
|---|---|---|---|---|---|
| `hooks.output.continue` | `continue` | `continue` | both | ◎ | lossless |
| `hooks.output.stopReason` | `stopReason` | `stopReason` | both | ◎ | lossless |
| `hooks.output.systemMessage` | `systemMessage` | `systemMessage` | both | ◎ | lossless |
| `hooks.output.suppressOutput` | `suppressOutput` | `suppressOutput` | both | ◎ | lossless |
| `hooks.output.hookSpecificOutput` | `hookSpecificOutput.*`（ネスト） | `hookSpecificOutput.*`（同形式）または flat `decision`/`reason` | both | ○ | Codex は flat も受け付ける。Claude→Codex はネスト形式そのまま使用可 |
| `hooks.output.permissionDecision` | `hookSpecificOutput.permissionDecision`（PreToolUse）`"allow"\|"deny"\|"ask"\|"defer"` | `hookSpecificOutput.permissionDecision`（同）`"allow"\|"deny"\|"ask"` | both | △ | `defer` は Claude のみ（非インタラクティブ `-p` 限定）。Codex→Claude では `defer` が存在しない |
| `hooks.output.additionalContext` | `hookSpecificOutput.additionalContext` | `hookSpecificOutput.additionalContext` | both | ◎ | lossless |
| `hooks.output.updatedInput` | `hookSpecificOutput.updatedInput` | `hookSpecificOutput.updatedInput` | both | ◎ | lossless |
| `hooks.output.terminalSequence` | `terminalSequence` | なし | claude_to_codex | ✕ | Claude のみ。dropped |
| `hooks.output.sessionTitle` | `hookSpecificOutput.sessionTitle`（SessionStart） | なし | claude_to_codex | ✕ | Claude のみ。dropped |
| `hooks.output.watchPaths` | `hookSpecificOutput.watchPaths`（SessionStart） | なし | claude_to_codex | ✕ | Claude のみ。dropped |
| `hooks.output.reloadSkills` | `hookSpecificOutput.reloadSkills`（SessionStart） | なし | claude_to_codex | ✕ | Claude のみ。dropped |
| `hooks.output.initialUserMessage` | `hookSpecificOutput.initialUserMessage`（SessionStart） | なし | claude_to_codex | ✕ | Claude のみ。dropped |
| `hooks.output.updatedMCPToolOutput` | なし | `hookSpecificOutput.updatedMCPToolOutput`（PostToolUse） | codex_to_claude | ✕ | Codex 独自。dropped |

### 3-8. JSON ⇄ TOML 変換例

**Claude JSON → Codex TOML（`command` hook、共通 10 イベント）**

```json
// Claude: .claude/settings.json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "/path/to/guard.sh",
            "timeout": 30,
            "statusMessage": "Checking command..."
          }
        ]
      }
    ]
  }
}
```

```toml
# Codex: .codex/config.toml
# matcher は exact "Bash" → regex "^Bash$" に変換

[[hooks.PreToolUse]]
matcher = "^Bash$"

[[hooks.PreToolUse.hooks]]
type = "command"
command = "/path/to/guard.sh"
timeout = 30
statusMessage = "Checking command..."
```

**Codex TOML → Claude JSON（共通 10 イベント）**

```toml
# Codex: config.toml
[[hooks.PostToolUse]]
matcher = "^(Edit|Write)$"

[[hooks.PostToolUse.hooks]]
type = "command"
command = "npx prettier --write $(jq -r '.tool_input.file_path')"
timeout = 60
statusMessage = "Formatting..."
commandWindows = "npx.cmd prettier --write ..."
```

```json
// Claude: .claude/settings.json
// commandWindows は shell: "powershell" の別エントリとして分岐、または dropped
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "npx prettier --write $(jq -r '.tool_input.file_path')",
            "timeout": 60,
            "statusMessage": "Formatting..."
          }
        ]
      }
    ]
  }
}
```

---

## 4. 変換時の注意・既知の落とし穴

### dropped される主なケース

- **Claude 固有 20 イベント**（`Setup`、`FileChanged`、`CwdChanged`、`ConfigChange`、`Elicitation` 等）を含む hook は Codex に変換できない。変換レポートに dropped として列挙し、ユーザーが手動で代替手段を検討する必要がある。
- **`http`/`mcp_tool` タイプ**は Codex に対応物がなく、dropped。HTTP の場合は Codex 側で `command` タイプ + curl スクリプトへの書き換えを手動で検討。
- **`prompt`/`agent` タイプ**: Codex はスキーマ上 parse するが実行しない。変換後に設定ファイルに残ってもサイレントに無視されるため、実質 dropped として警告が必要。
- **出力 JSON の Claude 固有フィールド**（`terminalSequence`、`sessionTitle`、`watchPaths`、`reloadSkills`、`initialUserMessage`）は Codex が無視する。これらを返すスクリプトをそのまま Codex に持ち込んでも機能しないが、エラーにはならない。

### lossy になるケース

- **matcher の exact 評価**: Claude の `"Bash"` は Codex で regex として評価されるため、`"Bash"` という文字列は部分マッチになる可能性がある。Claude→Codex 変換では `"^Bash$"` に変換すること。`"Edit|Write"` は `"^(Edit|Write)$"` に変換。
- **matcher のワイルドカード `*`**: Codex では `*` が regex の任意文字として解釈されず使用不可。`""` または省略で全マッチに変換する。
- **`tool_output` → `tool_response` のリネーム**: 意味は同等だが、スクリプトが `.tool_output` を参照している場合は `.tool_response` に書き換えが必要。
- **`commandWindows` ⇔ `shell: "powershell"`**: Codex には Windows/非 Windows を分岐するフィールドがあるが、Claude は `shell: "powershell"` で単一コマンドを指定する。Windows 向け hook を厳密に変換するには手動でコマンドを分岐させる必要がある。
- **`permissionDecision: "defer"`**: Claude の非インタラクティブモード（`-p`）限定の値。Codex には存在しないため、Codex→Claude 変換では `defer` が出力されることはないが、Claude→Codex では `defer` を含む hook 設定を変換すると dropped になる。

### `async` フィールドの注意

Claude の `async: true` と `asyncRewake: true` は別の意味を持つ。後者（exit 2 で Claude を再起動）は Codex に対応物がなく dropped。Codex の `async: true` は Claude の `async: true` に相当する（exit 2 でも再起動しない）。

### `if` フィールドの削除

Claude v2.1.85 以降で使える `if` フィールドは Codex に存在しない。`if` を使って精密にフィルタしている hook を Codex に変換すると、matcher レベルのフィルタのみが残り、本来はフィルタすべきだったケースも hook プロセスが起動する（性能影響・意図しない実行）。

### 環境変数の非互換

Claude の hook は `CLAUDE_PROJECT_DIR`、`CLAUDE_ENV_FILE`、`CLAUDE_EFFORT` 等の環境変数を利用できる。Codex は後方互換目的で `CLAUDE_PLUGIN_ROOT` と `CLAUDE_PLUGIN_DATA` を hook プロセスに設定するが、それ以外の Claude 環境変数は設定されない。`CLAUDE_PROJECT_DIR`、`CLAUDE_ENV_FILE`、`CLAUDE_EFFORT` 等に依存するスクリプトは Codex では動作しない。Codex 側では `cwd` フィールドから情報を取得するよう書き換えが必要。

### plugin 同梱 hooks がロードされない（Issue #16430）

Claude・Codex とも plugin マニフェスト内に `hooks/hooks.json` を持てる設計だが、**Codex には実装バグが存在する（Issue #16430、2026-04 報告、open）**。Codex の plugin マニフェストパーサーは `skills` と `mcp_servers` はパースするが、**`hooks` キーをパースしない**。hook discovery はユーザー設定フォルダ（`~/.codex/hooks.json` 等）のみをスキャンし、インストール済み plugin root の `hooks.json` はスキャンされない。

このため、Claude plugin（または Codex plugin）を Codex にインストールしても、その同梱 hooks は現状ロードされない可能性が高い。Claude の plugin 同梱 hooks を Codex に移行する場合は、hooks の定義内容をユーザー設定の `~/.codex/hooks.json` または `config.toml` に手動でコピーする必要がある。

### Codex イベントパリティは進行中

Codex は Issue #21753（umbrella tracker）で Claude の 29+ イベント全対応を目標に開発が進行中。変換ツールの判定上は現状の共通 10 イベント（SessionStart/UserPromptSubmit/PreToolUse/PermissionRequest/PostToolUse/SubagentStart/SubagentStop/PreCompact/PostCompact/Stop）を「相互変換可」としているが、今後 Codex 側のイベント追加により対応状況が変わる可能性がある。

### `features.codex_hooks` の状態

Codex では `features.hooks` が正式名（デフォルト有効）で、`features.codex_hooks` は deprecated alias として残っている。ソースコード（`codex-rs/config/src/config_toml.rs`）で確認済み。

### Windows サポート

Codex の `commandWindows` / `command_windows` フィールドは Windows 専用コマンド上書き機能だが、Codex CLI 自体の Windows サポート状況は現時点（2026-05）では未確認。

### Stop hook の無限ループ防止

両者ともに `stop_hook_active` フィールドを入力 JSON に含める。Stop hook がブロックを繰り返す場合、このフラグが `true` になる。スクリプト側で `stop_hook_active` をチェックして早期 exit 0 することで無限ループを防ぐ（Claude はデフォルト 8 回で上書き）。

---

## 5. 出典

- https://code.claude.com/docs/en/hooks （Claude Code Hooks Reference）
- https://code.claude.com/docs/en/hooks-guide （Claude Code Hooks Guide）
- https://developers.openai.com/codex/hooks （Codex Hooks Reference）
- https://developers.openai.com/codex/config-reference （Codex Config Reference）
- https://developers.openai.com/codex/config-sample （Codex Config Sample）
- https://github.com/openai/codex/blob/main/codex-rs/config/src/hook_config.rs （Codex HookHandlerConfig ソース）
- https://github.com/openai/codex/blob/main/codex-rs/hooks/src/schema.rs （Codex hooks schema.rs: 入出力 JSON スキーマ）
- https://github.com/openai/codex/pull/18893 （PR: TOML/requirements.toml への hooks 対応）
- https://github.com/openai/codex/issues/16430 （Bug: plugin-local hooks がロードされない、2026-04 報告・open）
- https://github.com/openai/codex/issues/21753 （Umbrella: Codex イベントパリティ tracker）
- https://developers.openai.com/codex/plugins/build （Codex Plugin Build Reference）
