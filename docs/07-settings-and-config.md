# Settings & Config: Claude Code ⇄ Codex

> Claude Code の `settings.json`（JSON, 多スコープ）と Codex CLI の `config.toml`（TOML, 多レイヤー）は「設定ファイル」として同役割を担うが、**権限モデルが根本的に異なる**（ツール軸 vs リソース軸）。model・env・sandbox・権限の部分集合のみ実用的変換の対象で、完全自動変換は非現実的。

---

## 0. 概要

Claude Code はすべての設定を JSON 形式の `settings.json` に一本化し、`permissions.allow/ask/deny` による **ツール名パターン**で権限を表現する。Codex CLI は TOML 形式の `config.toml` と複数補助ファイルに設定を分散し、`[permissions.<name>]` プロファイル（filesystem パス・network domain への **リソース軸**アクセス制御）と `approval_policy`（承認フェーズ制御）を組み合わせる。

変換難易度は **高**。一部フィールド（`editorMode:vim`→`tui.vim_mode_default` 等）は lossless だが、権限まわりは構造が根本的に異なりほとんどが lossy または dropped になる。CLI ツールが扱う現実的な変換対象は model / env / sandbox の基本項目・権限の近似マッピング・MCP・hooks に絞るべきであり、その旨を変換レポートに明記すること（hooks は docs/05-hooks.md、MCP は docs/06-mcp.md に委ねること）。

---

## 1. Claude Code 側の仕様

### 配置・ファイル・スコープ

| スコープ | パス | 優先順位 |
|---|---|---|
| managed（enterprise 強制） | managed settings 経由（UI/API） | 最高（1位） |
| user（全プロジェクト） | `~/.claude/settings.json` | 2位 |
| project | `.claude/settings.json` | 3位 |
| local（gitignore 推奨） | `.claude/settings.local.json` | 4位（最低） |

優先順位: Managed > CLI フラグ > Local > Project > User（高い方が wins）。

### 全フィールド表

#### コアモデル・動作

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `model` | string | — | all | デフォルトモデル上書き（例: `claude-opus-4-6`） |
| `modelOverrides` | object | — | user/project | プロバイダ別モデル ID マッピング |
| `effortLevel` | enum | — | all | 推論 effort（`low`/`medium`/`high`/`xhigh`/`max`） |
| `alwaysThinkingEnabled` | boolean | false | all | 拡張 thinking をデフォルト有効化 |
| `outputStyle` | string | — | all | 応答スタイル調整 |
| `language` | string | — | user | 応答言語（例: `"japanese"`） |
| `defaultShell` | string | `"bash"` | user/project | デフォルトシェル（`"bash"` / `"powershell"`） |

#### 権限

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `permissions.allow` | string[] | [] | all | 事前承認するツールパターン（例: `"Bash(npm run *)"`, `"Read(~/docs)"`, `"WebFetch(domain:example.com)"`） |
| `permissions.ask` | string[] | [] | all | 常にプロンプトするツールパターン |
| `permissions.deny` | string[] | [] | all | 常に拒否するツールパターン（評価順: deny → ask → allow） |
| `permissions.defaultMode` | enum | `"default"` | all | `"default"`/`"acceptEdits"`/`"auto"`/`"plan"`/`"dontAsk"`/`"bypassPermissions"` |
| `permissions.disableBypassPermissionsMode` | enum | — | all | `"disable"` でバイパスモード封印 |
| `permissions.additionalDirectories` | string[] | [] | all | スコープ拡張パス（`../docs/` 等） |
| `permissions.skipDangerousModePermissionPrompt` | boolean | false | all | dangerous モード時のプロンプトをスキップ |

#### サンドボックス

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `sandbox.enabled` | boolean | — | all | サンドボックス有効化 |
| `sandbox.failIfUnavailable` | boolean | false | all | サンドボックス非対応環境でエラー終了 |
| `sandbox.autoAllowBashIfSandboxed` | boolean | — | all | サンドボックス内 Bash を自動承認 |
| `sandbox.excludedCommands` | string[] | [] | all | サンドボックスをバイパスするコマンドパターン |
| `sandbox.allowUnsandboxedCommands` | boolean | — | all | 非サンドボックスコマンドを許可 |
| `sandbox.filesystem.allowWrite` | string[] | [] | all | 書き込み許可パス |
| `sandbox.filesystem.denyWrite` | string[] | [] | all | 書き込み拒否パス |
| `sandbox.filesystem.allowRead` | string[] | [] | all | 読み込み許可パス |
| `sandbox.filesystem.denyRead` | string[] | [] | all | 読み込み拒否パス |
| `sandbox.filesystem.allowManagedReadPathsOnly` | boolean | false | all | managed 読み込みパスのみ許可 |
| `sandbox.network.allowedDomains` | string[] | [] | all | 許可ドメイン配列 |
| `sandbox.network.allowAllUnixSockets` | boolean | false | all | Unix ソケット全許可 |
| `sandbox.network.allowUnixSockets` | string[] | [] | all | 許可 Unix ソケットパスリスト |
| `sandbox.network.allowLocalBinding` | boolean | false | all | ローカルバインディング許可 |
| `sandbox.network.allowMachLookup` | string[] | [] | all | Mach lookup 許可（macOS のみ） |

#### 環境変数

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `env` | object | {} | all | セッション全体に注入する環境変数（key-value） |

#### Hooks・MCP（詳細は別ドキュメント参照）

| フィールド | 型 | スコープ | 説明 |
|---|---|---|---|
| `hooks` | object | user/project | ライフサイクルイベント hooks（→ docs/05-hooks.md） |
| `mcpServers` / `enabledMcpjsonServers` / `disabledMcpjsonServers` | object/array | user/project/local | MCP サーバー設定（→ docs/06-mcp.md） |

#### メモリ・セッション管理

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `autoMemoryEnabled` | boolean | true | all | 自動メモリ保存有効化 |
| `autoMemoryDirectory` | string | `~/.claude/memory` | user/project/local | メモリ保存先カスタムパス |
| `cleanupPeriodDays` | integer | 30 | all | セッション保持日数（最小: 1） |
| `claudeMdExcludes` | string[] | [] | user/project | CLAUDE.md 除外 glob パターン |

#### モデル / エージェント連携

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `availableModels` | string[] | — | managed | 選択可能モデルを制限 |
| `agent` | string | — | user/project | 名前付きサブエージェントとして実行 |
| `skillOverrides` | object | — | user/project/local | skill 単位の表示/有効化上書き（v2.1.129+） |
| `maxSkillDescriptionChars` | integer | 1536 | user/project | skill 説明の文字数上限（v2.1.105+） |
| `skillListingBudgetFraction` | number | 0.01 | user/project | skill リスト用コンテキスト予算比率 |

#### Git・帰属

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `attribution.commit` | string | — | all | コミットへの co-authored-by 文字列 |
| `attribution.pr` | string | — | all | PR バッジ用文字列 |
| `includeCoAuthoredBy` | boolean | true | all | コミットに co-authored-by を含める（非推奨） |
| `includeGitInstructions` | boolean | true | user/project | システムプロンプトに git 指示を含める |
| `prUrlTemplate` | string | — | user/project | PR バッジ URL テンプレート |

#### UI・エディタ

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `editorMode` | enum | `"normal"` | user | キーバインドモード（`"normal"` / `"vim"`） |
| `tui` | enum | `"default"` | user | TUI レンダラー（`"fullscreen"` / `"default"`） |
| `viewMode` | enum | `"default"` | user | 表示モード（`"default"` / `"verbose"` / `"focus"`） |
| `statusLine` | object | — | user/project | カスタムステータスライン（→ dropped） |
| `spinnerTipsEnabled` | boolean | true | user | スピナーヒント表示 |
| `spinnerVerbs` | object | — | user/project | スピナー動詞カスタマイズ |
| `prefersReducedMotion` | boolean | false | user | UI アニメーション削減 |
| `syntaxHighlightingDisabled` | boolean | false | user | シンタックスハイライト無効化 |
| `showThinkingSummaries` | boolean | false | user | thinking サマリー表示 |
| `showTurnDuration` | boolean | true | user | ターン所要時間表示 |
| `autoScrollEnabled` | boolean | true | user | フルスクリーン出力のオートスクロール |

#### 自動更新・バージョン

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `autoUpdatesChannel` | enum | `"latest"` | user/project | 更新チャンネル（`"stable"` / `"latest"`） |
| `minimumVersion` | string | — | managed | バージョン下限（managed のみ） |

#### managed（enterprise）専用

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `forceLoginMethod` | enum | — | managed | ログイン制限（`"claudeai"` / `"console"`） |
| `forceLoginOrgUUID` | string/string[] | — | managed | 組織 UUID 強制 |
| `allowManagedPermissionRulesOnly` | boolean | false | managed | managed 権限ルールのみ有効化 |
| `allowManagedMcpServersOnly` | boolean | false | managed | managed MCP サーバーのみ |
| `allowManagedHooksOnly` | boolean | false | managed | managed hooks のみ |
| `allowedMcpServers` | string[] | — | managed | MCP サーバー許可リスト |
| `deniedMcpServers` | string[] | — | managed | MCP サーバー拒否リスト |
| `claudeMd` | string | — | managed | 組織強制メモリコンテンツ |
| `forceRemoteSettingsRefresh` | boolean | false | managed | 起動時に設定取得完了まで待機 |
| `disableAutoMode` | enum | — | managed | `"disable"` で auto mode 封印 |
| `disableAgentView` | boolean | false | managed | バックグラウンドエージェント無効化 |
| `disableRemoteControl` | boolean | false | managed | リモートコントロール無効化（v2.1.128+） |
| `disableSkillShellExecution` | boolean | false | managed | skill インラインシェル実行無効化 |
| `wslInheritsWindowsSettings` | boolean | false | managed | WSL が Windows ポリシーを読む |
| `parentSettingsBehavior` | enum | `"first-wins"` | managed | 親設定マージ動作（v2.1.133+） |
| `policyHelper` | object | — | managed | 設定を動的計算（v2.1.136+） |
| `companyAnnouncements` | array | — | managed | 起動時アナウンス |

#### その他

| フィールド | 型 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|
| `apiKeyHelper` | string | — | user/project | 認証値を出力するスクリプト |
| `awsCredentialExport` | string | — | user/project | AWS 認証情報エクスポートスクリプト |
| `awsAuthRefresh` | string | — | user/project | AWS 認証リフレッシュスクリプト |
| `gcpAuthRefresh` | string | — | user/project | GCP 認証リフレッシュスクリプト |
| `otelHeadersHelper` | string | — | user/project | OpenTelemetry ヘッダー生成スクリプト |
| `worktree` | object | — | user/project | git worktree 設定 |
| `voice` | object | — | user | 音声ディクテーション設定 |
| `voiceEnabled` | boolean | false | user | 音声ディクテーション有効化（legacy） |
| `autoMode` | object | — | user/project/local | カスタム auto モードルール |
| `plansDirectory` | string | `~/.claude/plans` | user/project | plan ファイル保存先 |
| `disableAllHooks` | boolean | false | all | 全 hooks 無効化 |
| `disableWorkflows` | boolean | false | all | 動的ワークフロー無効化 |
| `respectGitignore` | boolean | true | user/project | ファイルピッカーで .gitignore を尊重 |
| `fileSuggestion` | object | — | user/project | カスタム @ ファイル補完 |
| `enabledPlugins` | object | — | user/project | plugin の有効/無効マッピング |
| `allowedChannelPlugins` | array | — | managed | チャンネル plugin 許可リスト |
| `enableAllProjectMcpServers` | boolean | false | user/project/local | project MCP サーバーを自動承認 |

---

## 2. Codex 側の仕様

### 配置・ファイル・スコープ・レイヤー

| レイヤー（優先順） | パス / 方法 | 説明 |
|---|---|---|
| CLI フラグ | `--model`, `-c key=val` 等 | 最高優先（1位） |
| Project | `.codex/config.toml`（要 trust） | 2位。trust なしで無視 |
| Profile | `~/.codex/<name>.config.toml` | 3位。`--profile <name>` で活性化 |
| User | `~/.codex/config.toml` | 4位 |
| System | `/etc/codex/config.toml` | 5位 |
| Default | ビルトイン既定値 | 最低（6位） |
| Managed | `requirements.toml`（organization 強制） | 全レイヤーを上書き |

Project config が読まれるのは trust されたプロジェクトのみ。未 trust プロジェクトでは `.codex/` レイヤー（hooks・rules・configs）全体がスキップされる。

Project config で使用不可なキー: `openai_base_url`, `chatgpt_base_url`, `model_provider`, `model_providers`, `notify`, `profile`, `profiles`, テレメトリ関連。

### 全フィールド表

#### モデル設定

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `model` | string | — | 使用するモデル（例: `"gpt-5.5"`, `"o3"`） |
| `model_provider` | string | `"openai"` | プロバイダ ID（`model_providers` テーブルのキー） |
| `model_context_window` | integer | — | コンテキストウィンドウトークン数 |
| `model_auto_compact_token_limit` | integer | — | 自動コンパクト発動トークン閾値 |
| `model_reasoning_effort` | enum | — | 推論 effort（`minimal`/`low`/`medium`/`high`/`xhigh`） |
| `model_reasoning_summary` | enum | `"auto"` | 推論詳細（`auto`/`concise`/`detailed`/`none`） |
| `model_verbosity` | enum | — | GPT-5 応答冗長性（`low`/`medium`/`high`） |
| `model_supports_reasoning_summaries` | boolean | — | 推論メタデータ強制包含 |
| `model_instructions_file` | string | — | カスタム指示ファイルパス（`AGENTS.md` 代替） |
| `personality` | enum | `"none"` | スタイル（`none`/`friendly`/`pragmatic`） |

#### 承認・セキュリティ

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `approval_policy` | enum/object | `"on-request"` | `untrusted`/`on-request`/`never`/`{granular={...}}` |
| `approval_policy.granular.sandbox_approval` | boolean | — | sandbox エスカレーション承認 |
| `approval_policy.granular.rules` | boolean | — | execpolicy `prompt` ルール承認 |
| `approval_policy.granular.mcp_elicitations` | boolean | — | MCP tool 副作用承認 |
| `approval_policy.granular.request_permissions` | boolean | — | tool 権限リクエスト承認 |
| `approval_policy.granular.skill_approval` | boolean | — | skill スクリプト実行承認 |
| `approvals_reviewer` | enum | `"user"` | `"user"` / `"auto_review"` |
| `allow_login_shell` | boolean | true | ログインシェルセマンティクス許可 |

#### サンドボックス

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `sandbox_mode` | enum | `"workspace-write"` | `read-only`/`workspace-write`/`danger-full-access` |
| `default_permissions` | string | — | 使用するパーミッションプロファイル名（`:read-only`/`:workspace`/`:danger-full-access` または custom） |
| `sandbox_workspace_write.writable_roots` | string[] | [] | sandbox 内の追加書き込み許可パス |
| `sandbox_workspace_write.network_access` | boolean | false | sandbox 内のネットワークアクセス |
| `sandbox_workspace_write.exclude_slash_tmp` | boolean | false | `/tmp` を writable roots から除外 |
| `sandbox_workspace_write.exclude_tmpdir_env_var` | boolean | false | `$TMPDIR` を writable roots から除外 |
| `dangerously_allow_all_unix_sockets` | boolean | false | Unix ソケット全許可（危険） |

#### 権限プロファイル（`[permissions.<name>]`）

| フィールド | 型 | 説明 |
|---|---|---|
| `permissions.<name>.description` | string | プロファイル説明 |
| `permissions.<name>.extends` | string | 親プロファイル名（`:read-only`/`:workspace` 等） |
| `permissions.<name>.filesystem.<path>` | enum | パスへのアクセス（`read`/`write`/`deny`） |
| `permissions.<name>.filesystem.":workspace_roots".<subpath>` | enum | ワークスペースルート相対パスへのアクセス |
| `permissions.<name>.network.enabled` | boolean | ネットワークアクセス有効化 |
| `permissions.<name>.network.domains.<domain>` | enum | ドメインポリシー（`allow`/`deny`） |
| `permissions.<name>.network.unix_sockets.<path>` | enum | Unix ソケットポリシー（`allow`/`deny`） |
| `permissions.<name>.network.enable_socks5` | boolean | SOCKS5 サポート |
| `permissions.<name>.network.proxy_url` | string | sandbox ネットワーク用 HTTP プロキシ URL |
| `permissions.<name>.workspace_roots.<path>` | boolean | プロファイル定義のワークスペースルート |

#### シェル環境

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `shell_environment_policy.inherit` | enum | `"core"` | 環境変数継承基準（`all`/`core`/`none`） |
| `shell_environment_policy.set` | object | {} | サブプロセスへの環境変数上書き |
| `shell_environment_policy.include_only` | string[] | — | 保持する変数名パターン（whitelist） |
| `shell_environment_policy.exclude` | string[] | — | 除外する変数名 glob パターン |
| `shell_environment_policy.ignore_default_excludes` | boolean | false | KEY/SECRET/TOKEN 自動除外を無効化 |
| `shell_environment_policy.experimental_use_profile` | boolean | false | プロセス起動時にシェルプロファイル使用 |

#### エージェント管理

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `agents.max_threads` | integer | 6 | 同時エージェントスレッド上限 |
| `agents.max_depth` | integer | 1 | エージェントネスト最大深度 |
| `agents.job_max_runtime_seconds` | integer | 1800 | ジョブ最大実行時間（秒） |
| `agents.<name>.description` | string | — | エージェントロール説明 |
| `agents.<name>.config_file` | string | — | ロール用 TOML 設定ファイルパス |

#### メモリ

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `memories.use_memories` | boolean | true | 既存メモリをセッションに注入 |
| `memories.generate_memories` | boolean | true | 新規スレッドからメモリ生成 |
| `memories.max_rollout_age_days` | integer | 30 | メモリ生成対象の最大スレッド経過日数（0-90） |
| `memories.max_unused_days` | integer | 30 | 未使用メモリの有効期限日数（0-365） |
| `memories.max_raw_memories_for_consolidation` | integer | 256 | コンソリデーション対象メモリ数上限 |
| `memories.min_rollout_idle_hours` | integer | 6 | メモリ生成対象の最小アイドル時間（1-48） |
| `memories.disable_on_external_context` | boolean | false | MCP/web-search スレッドでのメモリ生成無効化 |

#### TUI

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `tui.vim_mode_default` | boolean | false | コンポーザを Vim モードで起動 |
| `tui.theme` | string | — | シンタックスハイライトテーマ |
| `tui.animations` | boolean | true | ターミナルアニメーション |
| `tui.notifications` | boolean/string[] | — | デスクトップ通知設定 |
| `tui.notification_method` | enum | `"auto"` | `auto`/`osc9`/`bel` |
| `tui.notification_condition` | enum | `"unfocused"` | `unfocused`/`always` |
| `tui.alternate_screen` | enum | `"auto"` | `auto`/`always`/`never` |
| `tui.raw_output_mode` | boolean | false | TUI をスクロールバックモードで起動 |
| `tui.show_tooltips` | boolean | true | オンボーディングツールチップ |
| `tui.status_line` | string[]/null | — | フッターステータスライン識別子配列 |
| `tui.keymap.<context>.<action>` | string/string[] | — | キーボードショートカットバインディング |

#### 履歴・ログ

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `history.persistence` | enum | `"save-all"` | `save-all`/`none` |
| `history.max_bytes` | integer | — | 履歴ファイル最大サイズ（バイト） |
| `log_dir` | string | `$CODEX_HOME/log` | ログファイルディレクトリ |

#### その他

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `developer_instructions` | string | — | セッションに注入する追加指示 |
| `commit_attribution` | string | `"Codex <noreply@openai.com>"` | git コミット co-author トレーラー |
| `file_opener` | enum | — | ファイルリンク URI スキーム（`vscode`/`cursor`/`windsurf`/`none`） |
| `project_doc_max_bytes` | integer | — | AGENTS.md 等の最大読み込みバイト数 |
| `web_search` | enum | `"cached"` | `disabled`/`cached`/`live` |
| `check_for_update_on_startup` | boolean | true | 起動時更新チェック |
| `hide_agent_reasoning` | boolean | false | 推論出力の抑制 |
| `show_raw_agent_reasoning` | boolean | false | 生の推論コンテンツ表示 |
| `notify` | string[] | — | 外部通知コマンド（JSON 引数付き） |
| `openai_base_url` | string | — | OpenAI データレジデンシー用 API ベース URL |

#### features フラグ

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `features.network_proxy` | boolean/object | false | sandbox ネットワークプロキシ（experimental） |
| `features.unified_exec` | boolean | true | 統合 PTY exec ツール（Windows 除く） |
| `features.shell_tool` | boolean | true | デフォルトシェルツール有効化 |
| `features.shell_snapshot` | boolean | true | シェル環境スナップショット |
| `features.multi_agent` | boolean | false | マルチエージェントコラボレーション |
| `features.memories` | boolean | false | メモリ機能有効化 |
| `features.hooks` | boolean | false | ライフサイクル hooks 有効化 |
| `features.undo` | boolean | false | アンドゥサポート |

#### OpenTelemetry

| フィールド | 型 | デフォルト | 説明 |
|---|---|---|---|
| `otel.environment` | string | `"dev"` | OTel 環境タグ |
| `otel.exporter` | enum | `"none"` | `none`/`otlp-http`/`otlp-grpc` |
| `otel.trace_exporter` | enum | — | トレースエクスポーター |
| `otel.log_user_prompt` | boolean | false | 生プロンプトのエクスポート |

---

## 3. 変換テーブル

`mappings/settings-config.yaml` の人間可読版。

### 3.1 モデル・動作

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `settings.model` | `model` | `model` | both | lossy | — | モデル ID 書式が別（`claude-opus-4-6` ⇔ `gpt-5.x`）。機械変換不可・要マップテーブル |
| `settings.effortLevel` | `effortLevel`（low/medium/high/xhigh/`max`） | `model_reasoning_effort`（`minimal`/low/medium/high/xhigh） | both | lossy | — | `enum_map:{max:xhigh}`（Codexに `max` なし）; Codex の `minimal` は Claude に対応なし（dropped） |
| `settings.language` | `language` | `developer_instructions`（追記） | claude_to_codex | lossy | — | 独立フィールドなし。`developer_instructions` への自然言語追記で近似 |
| `settings.defaultShell` | `defaultShell` | `shell_environment_policy.experimental_use_profile` | both | lossy | — | 意味が異なる（shell 選択 vs プロファイル使用）。近似的対応のみ |
| `settings.outputStyle` | `outputStyle` | `developer_instructions`（追記） | claude_to_codex | lossy | — | 独立フィールドなし。`developer_instructions` への追記で近似 |

### 3.2 権限（ツール軸 → リソース軸変換）

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `settings.permissions.allow.bash` | `permissions.allow: ["Bash(...)"]` | `[permissions.<name>].rules execpolicy allow` | claude_to_codex | lossy | project/session | ツールパターン → `.rules` の `decision="allow"` へ。skill スコープ→session スコープに拡大 |
| `settings.permissions.deny.bash` | `permissions.deny: ["Bash(...)"]` | `[permissions.<name>].rules execpolicy forbidden` | claude_to_codex | lossy | project/session | ツールパターン → `.rules` の `decision="forbidden"` へ |
| `settings.permissions.allow.read` | `permissions.allow: ["Read(path)"]` | `[permissions.<name>].filesystem.<path>="read"` | claude_to_codex | lossy | — | ツール軸→ファイルシステムリソース軸へ変換。パス正規化が必要 |
| `settings.permissions.deny.read` | `permissions.deny: ["Read(path)"]` | `[permissions.<name>].filesystem.<path>="deny"` | claude_to_codex | lossy | — | 同上 |
| `settings.permissions.allow.write` | `permissions.allow: ["Write(path)"]` | `[permissions.<name>].filesystem.<path>="write"` | claude_to_codex | lossy | — | 同上 |
| `settings.permissions.deny.webfetch` | `permissions.deny/allow: ["WebFetch(domain:x)"]` | `[permissions.<name>].network.domains.<domain>="allow/deny"` | claude_to_codex | lossy | — | str → TOML table へ変換。ワイルドカード記法差異に注意 |
| `settings.permissions.defaultMode` | `defaultMode: "bypassPermissions"` | `approval_policy="never"` + `sandbox_mode="danger-full-access"` | claude_to_codex | lossy | — | 2フィールドの組み合わせで近似。完全等価ではない |
| `settings.permissions.defaultMode.acceptEdits` | `defaultMode: "acceptEdits"` | `approval_policy="untrusted"` | claude_to_codex | lossy | — | 近似的対応のみ。意味に差異あり |
| `settings.permissions.defaultMode.auto` | `defaultMode: "auto"` | `approval_policy="on-request"` | claude_to_codex | lossy | — | 近似的対応のみ |
| `settings.permissions.defaultMode.plan` | `defaultMode: "plan"` | （対応なし） | claude_to_codex | dropped | — | Codex に plan モード相当なし |

### 3.3 サンドボックス

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `settings.sandbox.filesystem.allowWrite` | `sandbox.filesystem.allowWrite` | `[permissions.<name>].filesystem.<path>="write"` または `sandbox_workspace_write.writable_roots` | both | lossy | — | 配列 → TOML table（パスをキーに）へ変換 |
| `settings.sandbox.filesystem.denyWrite` | `sandbox.filesystem.denyWrite` | `[permissions.<name>].filesystem.<path>="deny"` | both | lossy | — | 同上 |
| `settings.sandbox.filesystem.allowRead` | `sandbox.filesystem.allowRead` | `[permissions.<name>].filesystem.<path>="read"` | both | lossy | — | 同上 |
| `settings.sandbox.filesystem.denyRead` | `sandbox.filesystem.denyRead` | `[permissions.<name>].filesystem.<path>="deny"` | both | lossy | — | 同上 |
| `settings.sandbox.network.allowedDomains` | `sandbox.network.allowedDomains`（string[]） | `[permissions.<name>].network.domains`（table） | both | lossy | — | `str_to_list` ↔ TOML table 変換。Claude は配列、Codex は `{domain="allow"}` テーブル |
| `settings.sandbox.network.allowAllUnixSockets` | `sandbox.network.allowAllUnixSockets` | `dangerously_allow_all_unix_sockets` | both | lossless | — | `rename` のみ |
| `settings.sandbox.network.allowMachLookup` | `sandbox.network.allowMachLookup` | （対応なし） | claude_to_codex | dropped | — | macOS Mach lookup。Codex に対応なし |

### 3.4 環境変数

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `settings.env` | `env`（object、セッション全体） | `shell_environment_policy.set`（サブプロセスのみ） | both | lossy | — | Claude はセッション全体、Codex はサブプロセスへの注入のみ。スコープ差に注意 |

### 3.5 メモリ・セッション

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `settings.autoMemoryEnabled` | `autoMemoryEnabled` | `memories.use_memories` + `memories.generate_memories` | both | lossy | — | Claude の 1 フィールド → Codex の 2 フィールド。`features.memories=true` も必要 |
| `settings.cleanupPeriodDays` | `cleanupPeriodDays`（days） | `history.max_bytes`（bytes）/ `memories.max_rollout_age_days`（days） | both | lossy | — | 意味が異なる（保持日数 vs サイズ上限）。`memories.max_rollout_age_days` で近似 |

### 3.6 Git・帰属

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `settings.attribution.commit` | `attribution.commit` | `commit_attribution` | both | lossy | — | `rename`。書式差異に注意（Claude は co-authored-by 文字列全体、Codex は `Name <email>` 形式） |
| `settings.includeCoAuthoredBy` | `includeCoAuthoredBy`（deprecated） | `commit_attribution`（空文字で無効化） | claude_to_codex | lossy | — | deprecated フィールド。`commit_attribution=""` で近似 |

### 3.7 UI・エディタ

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `settings.editorMode` | `editorMode: "vim"` | `tui.vim_mode_default: true` | both | lossless | — | `rename` + boolean 変換のみ |
| `settings.statusLine` | `statusLine` | （対応なし） | claude_to_codex | dropped | — | Codex に Claude の statusLine 等価物なし |

### 3.8 skills・plugins

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `settings.skillOverrides` | `skillOverrides`（on/off/name-only/user-invocable-only） | `skills.config[].enabled`（boolean） | both | lossy | — | Claude の細粒度制御（4 値）→ Codex の on/off のみ |
| `settings.enabledPlugins` | `enabledPlugins`（object） | `plugins.*`（lossy） | both | lossy | — | プラグイン体系が異なるため近似対応のみ |

### 3.9 Claude 固有 → Codex dropped

| id | Claude フィールド | 理由 |
|---|---|---|
| `settings.statusLine` | `statusLine` | Codex に等価物なし |
| `settings.viewMode` | `viewMode` | Codex に等価モード区分なし |
| `settings.worktree` | `worktree.*` | Codex に worktree 管理なし |
| `settings.autoUpdatesChannel` | `autoUpdatesChannel` | Codex に等価物なし |
| `settings.spinnerTipsEnabled` | `spinnerTipsEnabled`, `spinnerTipsOverride`, `spinnerVerbs` | UI 固有 |
| `settings.voice` | `voice.*`, `voiceEnabled` | Codex に音声機能なし |
| `settings.maxSkillDescriptionChars` | `maxSkillDescriptionChars`, `skillListingBudgetFraction` | Codex 非対応 |
| `settings.forceLoginMethod` | `forceLoginMethod`, `forceLoginOrgUUID` 等の managed 系 | 一部は `requirements.toml` で近似可（組織管理者のみ） |
| `settings.permissions.defaultMode.plan` | `defaultMode: "plan"` | Codex に plan モードなし |

### 3.10 Codex 固有 → Claude dropped/lossy

| id | Codex フィールド | 理由 |
|---|---|---|
| `settings.codex.profiles` | `profiles`（named config presets） | Claude にプロファイル概念なし |
| `settings.codex.permissions_inheritance` | `permissions.<name>.extends` | Claude に権限プロファイル継承なし |
| `settings.codex.granular_approval` | `approval_policy.granular.*`（skill_approval/mcp_elicitations 等） | Claude に個別承認カテゴリ分離なし |
| `settings.codex.agents_config` | `agents.max_threads`, `agents.max_depth` | Claude に直接対応なし |
| `settings.codex.otel` | `otel.*` | Claude は env 経由で OTel 設定（`env.OTEL_*`）。独立セクションなし |
| `settings.codex.tui_keymap` | `tui.keymap.*` | Claude に programmatic keymap 設定なし |
| `settings.codex.model_verbosity` | `model_verbosity`, `model_reasoning_summary` | Claude に対応フィールドなし |
| `settings.codex.service_tier` | `service_tier` | Claude に対応なし |
| `settings.codex.web_search` | `web_search` | Claude は WebFetch tool で代替 |

---

## 4. 権限モデルの軸違い（重要）

Claude Code と Codex CLI は権限の「軸」が根本的に異なる。この差異が変換難易度を最も高めている要因であり、CLI 実装者は必ず理解すること。

### ツール軸 vs リソース軸

| 観点 | Claude Code | Codex CLI |
|---|---|---|
| **軸** | **ツール名**（Bash / Read / Write / WebFetch / …） | **リソース**（ファイルシステムパス / ネットワークドメイン） + **承認フェーズ** |
| **表現形式** | `permissions.allow/ask/deny: ["ToolName(pattern)"]` | `[permissions.<name>].filesystem.<path>="read/write/deny"` + `approval_policy` |
| **評価順** | deny → ask → allow（最初に一致したルールが適用） | プロファイル継承チェーン → 承認フェーズ判定 |
| **スコープ** | allow/deny/ask はすべて同一オブジェクト内 | ファイルアクセス制御とコマンド承認フェーズが分離 |
| **ツール種別** | Bash・Read・Write・Edit・WebFetch・Glob・Grep 等を統一パターンで記述 | Bash コマンド → `.rules`（execpolicy）; ファイル → `filesystem`; ネットワーク → `network.domains` |
| **動的ツール（MCP）** | `mcp__<server>__<tool>` をパターンで制御 | `[mcp_servers.<id>].enabled_tools` / `default_tools_approval_mode` で制御 |

### 権限対応マトリクス

| Claude の権限対象 | Claude の表現 | Codex の対応物 | 変換品質 |
|---|---|---|---|
| Bash コマンド（allow） | `permissions.allow: ["Bash(npm run *)"]` | `.rules` の `prefix_rule decision="allow"` | △ lossy（スコープ拡大） |
| Bash コマンド（deny） | `permissions.deny: ["Bash(rm -rf *)"]` | `.rules` の `prefix_rule decision="forbidden"` | △ lossy（スコープ拡大） |
| Bash コマンド（ask） | `permissions.ask: ["Bash(git push *)"]` | `.rules` の `prefix_rule decision="prompt"` | △ lossy |
| ファイル読み込み許可 | `permissions.allow: ["Read(~/docs)"]` | `[permissions.X].filesystem."~/docs"="read"` | △ lossy（ツール境界なし） |
| ファイル読み込み拒否 | `permissions.deny: ["Read(~/.env)"]` | `[permissions.X].filesystem."~/.env"="deny"` | △ lossy |
| ファイル書き込み | `permissions.allow: ["Write(/tmp)"]` | `[permissions.X].filesystem."/tmp"="write"` | △ lossy |
| WebFetch ドメイン許可 | `permissions.allow: ["WebFetch(domain:api.x.com)"]` | `[permissions.X].network.domains."api.x.com"="allow"` | △ lossy（str→table） |
| WebFetch ドメイン拒否 | `permissions.deny: ["WebFetch(domain:bad.com)"]` | `[permissions.X].network.domains."bad.com"="deny"` | △ lossy |
| MCP ツール制御 | `permissions.allow: ["mcp__server__tool"]` | `[mcp_servers.server].enabled_tools=["tool"]` | △ lossy |
| 全権バイパス | `defaultMode: "bypassPermissions"` | `approval_policy="never"` + `sandbox_mode="danger-full-access"` | △ lossy（2フィールド組み合わせ） |
| plan モード | `defaultMode: "plan"` | （対応なし） | ✕ dropped |
| 組み込みツール禁止 | `permissions.deny: ["AskUserQuestion"]` | （対応なし） | ✕ dropped |
| 承認フェーズ分離 | （なし）| `approval_policy.granular.*`（skill_approval 等） | ✕ dropped（Claude→Codex）|
| プロファイル継承 | （なし）| `permissions.<name>.extends` | ✕ dropped（Claude→Codex）|

### 変換時の実装指針

1. **`permissions.allow/deny`（Bash 系）→ `.rules`** : `prefix_rule` の `decision` フィールドに変換。ただし元の skill スコープが session/project に昇格するため **必ず warn を出す**。
2. **`permissions.allow/deny`（Read/Write 系）→ `filesystem`**: パス文字列をキーにした TOML table に変換。ツール境界（Read vs Write vs Edit）は Codex では区別されないため **lossy**。
3. **`permissions.allow/deny`（WebFetch 系）→ `network.domains`**: 文字列配列 → ドメインをキーにした TOML table に変換（`str_to_list` ↔ table）。
4. **`defaultMode`** : `bypassPermissions` → `approval_policy="never"` + `sandbox_mode="danger-full-access"` の組み合わせで近似。`plan` は dropped。
5. **Claude 側 ask → Codex**: `approval_policy.granular.rules=true` への変換が最も近いが、粒度が異なる。
6. **Codex 側の承認フェーズ分離は Claude に移植不可**: `granular.skill_approval` / `mcp_elicitations` 等は Claude→Codex では **dropped**（codex_to_claude 方向）。

---

## 5. 変換時の注意・既知の落とし穴

### 5.1 完全自動変換は非現実的

settings.json ⇄ config.toml の完全自動変換は非現実的。CLI ツールが現実的に扱える変換対象は以下に絞るべき:
- model（ID マップテーブル必須）
- effortLevel ⇔ model_reasoning_effort（enum マップ）
- env ⇔ shell_environment_policy.set（スコープ差を明記）
- sandbox の基本項目（filesystem / network）
- editorMode ⇔ tui.vim_mode_default（lossless）
- attribution.commit ⇔ commit_attribution
- 権限の**近似マッピング**（すべて lossy として warn 必須）

変換レポートには dropped/lossy 全項目を必ず列挙すること。

### 5.2 権限評価順序の差異

Claude の `deny→ask→allow` の評価順と、Codex の `プロファイル継承チェーン→承認フェーズ` は評価方式が根本的に異なる。特に Claude で `deny` と `allow` が同じパスに対して並存するケースは、Codex の `deny` 優先ルールと一致しないことがある。

### 5.3 `sandbox_mode` と `default_permissions` の排他

Codex では `sandbox_mode`/`sandbox_workspace_write`（旧方式）と `default_permissions`/`[permissions.*]`（新方式）を併用してはならない（どちらか一方のみ有効）。新規変換では新方式を使用すること。

### 5.4 Project config の trust 要件

Codex の `.codex/config.toml`（project スコープ）は trust されたプロジェクトでのみ読み込まれる。未 trust 環境では権限設定が完全に無視されるため、変換後の動作確認時に注意すること。また project config で使用できないキー（`model_provider` 等）がある。

### 5.5 `env` のスコープ差

Claude の `env` はセッション全体（Claude 自身も含む）への環境変数注入だが、Codex の `shell_environment_policy.set` はサブプロセス（Bash 等）への注入のみ。変換後に Claude 自身が参照する変数（`ANTHROPIC_API_KEY` 等）は Codex 側では効かない。

### 5.6 モデル ID の非互換

`claude-opus-4-6` ⇔ `gpt-5.x` はモデル ID 体系が完全に異なり、機械的な文字列変換は不可。変換 CLI は別途モデルマッピングテーブルを保持する必要がある。

### 5.7 `effortLevel: "max"` と `minimal`

Claude の `effortLevel: "max"` は Codex に対応値がないため `xhigh` に丸める（lossy）。Codex の `model_reasoning_effort: "minimal"` は Claude に対応値がないため変換不可（dropped）。

### 5.8 hooks・MCP は別ドキュメント参照

hooks（`hooks` フィールド）と MCP サーバー（`mcpServers` 系フィールド）の詳細変換仕様は本ドキュメントの対象外。それぞれ `docs/05-hooks.md` および `docs/06-mcp.md` を参照すること。本 YAML（`mappings/settings-config.yaml`）にも hooks/mcp エントリは含めず、notes で相互参照に留める。

### 5.9 Codex 固有機能の Claude への移植不可

`tui.keymap`（プログラマブルキーバインド）、`otel.*`（独立 OTel セクション）、`agents.max_threads/max_depth`（エージェント並列度）、`service_tier`、`web_search`（専用フラグ）、`profiles`（named config）は Claude に対応機能がなく dropped。

---

## 5. 出典

- Claude Code settings: https://code.claude.com/docs/en/settings
- Claude Code settings JSON Schema: https://www.schemastore.org/claude-code-settings.json
- Codex CLI config reference: https://developers.openai.com/codex/config-reference
- Codex CLI config advanced: https://developers.openai.com/codex/config-advanced
- Codex CLI permissions: https://developers.openai.com/codex/permissions
- Codex CLI agent approvals & security: https://developers.openai.com/codex/agent-approvals-security
- Codex CLI sandboxing concepts: https://developers.openai.com/codex/concepts/sandboxing
- GitHub openai/codex: https://github.com/openai/codex
