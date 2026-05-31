# Claude Code ⇄ OpenAI Codex CLI: 機能対応マスター表

## 総括

| 分類 | エントリ数 | 割合 |
|---|---|---|
| 無損失 (lossless) | 71 | 25 % |
| 形式/スコープ変換あり (lossy) | 87 | 30 % |
| 破棄 (dropped) | 129 | 45 % |
| **合計** | **287** | **100 %** |

**方向性の非対称**:
- **Codex → Claude Code はほぼ無損失**。Codex の語彙・フィールド数が少なく、Claude 側に多くの受け皿がある。
- **Claude Code → Codex は損失が大きい**。Claude は機能リッチ（skill スコープ・引数機構・動的注入・豊富な hook イベント等）であり、Codex にはこれらの受け皿がない、または fail open で黙って無視されるだけ。

**価値の中心**: Skills / Hooks / Plugins の3領域に「降格エンジン・JSON⇄TOML 構造変換・コンポーネント統合」が集中する。MCP・Memory は機械的変換で済む軽量領域。Settings⇄Config は権限の軸違い（ツール軸 vs リソース軸）で最難関。

---

## 凡例

| 記号 | 意味 |
|---|---|
| ◎ | 無損失（lossless）—— フィールド値が完全に保存される |
| ○ | 形式変換のみ（lossy）—— rename / 単位変換 / 形式変換など意味は保持 |
| △ | 降格・部分対応・警告あり（lossy/degrade）—— スコープが変わるか意味が変わるか近似のみ |
| ✕ | 不可・破棄（dropped）—— 相手側に等価機構がなく変換不能 |
| — | 該当なし（その方向の変換が定義されていない） |
| ⏳ | 将来追従候補（Codex 実装待ち）—— 現状 ✕ だが Codex に機能実装されれば対応可能 |

> **「将来追従」列**は現状 ✕/dropped だが **Codex 側への機能実装で対応可能になる項目**に ⏳ を付ける。
> 原理的に対応不能（設計思想の差）なものは「—」。詳細は §8「将来追従の仕組み」を参照。

---

## 1. Skills (SKILL.md)

> ソース: `mappings/skills.yaml` / 詳細: `docs/02-skills.md`

| 機能 | c2x (Claude→Codex) | x2c (Codex→Claude) | 将来追従 | 備考 |
|---|---|---|---|---|
| **ディレクトリパス** | ◎ | ◎ | — | `.claude/skills/<n>/` ⇄ `.agents/skills/<n>/`、ファイル名 SKILL.md 共通 |
| **name** | ◎ | ◎ | — | 両者必須。Claude は lowercase 英数ハイフン・64 文字以下制約 |
| **description** | ◎ | ◎ | — | 自動発火トリガーを兼ねる必須フィールド |
| **when_to_use** | ○ | — | — | Codex に独立フィールドなし → description 末尾に連結 |
| **disable-model-invocation** | ○ | ◎ | — | 極性反転: `disable-model-invocation: true` ⇔ `policy.allow_implicit_invocation: false`。挙動差あり（明示呼び出しの扱いが Claude と異なる） |
| **user-invocable** | ✕ | — | ⏳ | Codex に「モデル専用・ユーザー非表示」概念なし。fail open で黙って無視。Codex 側に実装されれば対応可能 |
| **allowed-tools** | △ | — | ⏳ | skill スコープ不可 → `.codex/rules/<skill>.rules`（execpolicy allow）/ `mcp_servers.<id>.enabled_tools` へ降格（session/project スコープ）。組み込みツール承認は代替なし→✕ |
| **disallowed-tools** | △ | — | ⏳ | 同上、forbiddenルールへ降格。組み込みツール禁止は✕ |
| **model** | △ | — | ⏳ | SKILL.md frontmatter では fail open → `.codex/agents/<skill>.toml` に降格（subagent スコープ）。自動 fork なし |
| **effort** | △ | — | ⏳ | 同上。`max` → `xhigh` に丸め。subagent TOML への降格が必要。Codex に `paths` 等が実装されれば skill スコープで対応可能になる |
| **context: fork** | △ | — | — | 自動 fork は Codex に原理的になし（spawn_agent 明示が設計方針）。subagent TOML への降格。`features.multi_agent=true` 必須 |
| **skill-scoped hooks** | △ | — | ⏳ | skill スコープ不可 → session/project hooks へ降格。command タイプのみ移植可 |
| **paths（glob 自動発火）** | ✕ | — | ⏳ | ファイル操作イベント駆動の自動発火は Codex に等価機構なし。Codex 側に実装されれば対応可能 |
| **argument-hint** | ✕ | — | — | Codex に引数機構なし（deprecated Custom Prompts のみ有効） |
| **arguments** | ✕ | — | — | 同上 |
| **shell** | ○ | — | — | `shell: powershell` → `commandWindows` フィールドで近似（hooks 側） |
| **呼び出し記法 /name ⇄ $name** | ◎ | ◎ | — | 本文・README 内の記法変換が必要（自動変換は誤検出リスクあり） |
| **interface (display_name/icon)** | — | △ | — | Claude の skill に UI メタデータなし → warn + 手動確認 |
| **default_prompt** | — | △ | — | 本文先頭への追記で近似 |
| **dependencies.tools** | — | △ | — | Claude skill レベルの直接受け皿なし → warn |
| **本文動的注入 !`cmd`** | ✕ | — | — | Codex Issue #5019 で "not planned"。リテラル化して誤動作リスク |
| **${CLAUDE_*} 変数** | ✕ | — | — | Codex に同等変数なし。変換後にリテラル文字列として残ると誤動作 |

**Skills 小計**: lossless 5 / lossy 12 / dropped 5 (計 22 エントリ)

---

## 2. Hooks

> ソース: `mappings/hooks.yaml` / 詳細: `docs/05-hooks.md`

### 2-1. 設定形式・features

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **設定フォーマット（JSON⇄TOML）** | ◎ | ◎ | — | JSON→JSON はそのまま、JSON→TOML は format:json_to_toml |
| **features.hooks opt-in** | — | △ | — | Claude は常時有効、Codex はデフォルト有効（`false` で無効化）。Claude→Codex では追記不要 |

### 2-2. 共通イベント（◎/両方向）

以下の 10 イベントは両者に存在し、形式変換のみで lossless:

| イベント | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| SessionStart | ◎ | ◎ | — | — |
| UserPromptSubmit | ◎ | ◎ | — | — |
| PreToolUse | ◎ | ◎ | — | — |
| PermissionRequest | ◎ | ◎ | — | — |
| PostToolUse | ◎ | ◎ | — | — |
| PreCompact | ◎ | ◎ | — | — |
| PostCompact | ◎ | ◎ | — | — |
| SubagentStart | ◎ | ◎ | — | — |
| SubagentStop | ◎ | ◎ | — | — |
| Stop | ◎ | ◎ | — | — |

### 2-3. Claude 固有イベント（✕/Claude→Codex は dropped）

以下の 20 イベントは Claude 固有で Codex に未実装:

| イベント | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| Setup | ✕ | — | ⏳ | `--init-only` 専用 |
| UserPromptExpansion | ✕ | — | ⏳ | スラッシュコマンド展開フック |
| PermissionDenied | ✕ | — | ⏳ | — |
| PostToolUseFailure | ✕ | — | ⏳ | — |
| PostToolBatch | ✕ | — | ⏳ | 並列ツールバッチ完了後 |
| Notification | ✕ | — | ⏳ | — |
| MessageDisplay | ✕ | — | ⏳ | — |
| TaskCreated | ✕ | — | ⏳ | — |
| TaskCompleted | ✕ | — | ⏳ | — |
| StopFailure | ✕ | — | ⏳ | API エラー時のターン終了 |
| TeammateIdle | ✕ | — | ⏳ | エージェントチーム専用 |
| InstructionsLoaded | ✕ | — | ⏳ | CLAUDE.md 読み込み時 |
| ConfigChange | ✕ | — | ⏳ | 設定ファイル変更検知 |
| CwdChanged | ✕ | — | ⏳ | 作業ディレクトリ変更 |
| FileChanged | ✕ | — | ⏳ | ファイル変更監視 |
| WorktreeCreate | ✕ | — | ⏳ | git worktree 作成 |
| WorktreeRemove | ✕ | — | ⏳ | git worktree 削除 |
| Elicitation | ✕ | — | ⏳ | MCP サーバーのユーザー入力要求 |
| ElicitationResult | ✕ | — | ⏳ | MCP elicitation 応答後 |
| SessionEnd | ✕ | — | ⏳ | セッション終了 |

> Codex の hook イベントパリティは Issue #21753 で開発中。上記 20 イベントは Codex に実装されれば対応可能。

### 2-4. Hook タイプ

| タイプ | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **command** | ◎ | ◎ | — | 両者で実行可能な唯一の共通タイプ |
| **http** | ✕ | — | ⏳ | Codex に HTTP hook タイプなし。代替: command + curl |
| **mcp_tool** | ✕ | — | ⏳ | Codex に mcp_tool hook タイプなし |
| **prompt** | ✕ | — | ⏳ | Codex はスキーマ上 parse するが実行エンジン未実装。変換後にサイレント無視 |
| **agent** | ✕ | — | ⏳ | 同上（実行エンジン未実装）。experimental |

### 2-5. command フィールド・Matcher・I/O JSON

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **command** | ◎ | ◎ | — | — |
| **timeout** | ◎ | ◎ | — | 両者とも秒単位 |
| **statusMessage** | ◎ | ◎ | — | — |
| **async** | ◎ | ◎ | — | — |
| **commandWindows** | — | △ | — | Codex→Claude: `shell: "powershell"` で近似 |
| **args（exec form）** | ✕ | — | — | command + args を shell form に合成 |
| **shell** | ✕ | — | — | Codex は OS デフォルトシェルのみ |
| **if（条件式）** | ✕ | — | ⏳ | Codex 未対応 |
| **once** | ✕ | — | ⏳ | セッション 1 回のみ実行フラグ |
| **asyncRewake** | ✕ | — | — | exit 2 で Claude を再起動する特殊フラグ。Codex の設計では不要 |
| **matcher（regex）** | ◎ | ◎ | — | — |
| **matcher（exact/list）** | ○ | ○ | — | Codex は常に regex 評価 → `^Bash$` 形式に変換必要 |
| **matcher（wildcard）** | ○ | ○ | — | `"*"` → `""` または省略に変換 |
| **入力: session_id** | ◎ | ◎ | — | — |
| **入力: tool_name** | ◎ | ◎ | — | — |
| **入力: tool_input** | ◎ | ◎ | — | — |
| **入力: tool_output / tool_response** | ○ | ○ | — | キー名が異なる（rename 変換が必要） |
| **入力: agent_id / agent_type** | ◎ | ◎ | — | — |
| **入力: stop_hook_active** | ◎ | ◎ | — | 無限ループ防止フラグ |
| **入力: effort** | ✕ | — | — | Claude のみ |
| **入力: model / turn_id / tool_use_id** | — | ✕ | — | Codex のみ |
| **出力: continue / stopReason** | ◎ | ◎ | — | — |
| **出力: systemMessage** | ◎ | ◎ | — | — |
| **出力: suppressOutput** | ◎ | ◎ | — | — |
| **出力: hookSpecificOutput** | ◎ | ◎ | — | ネスト構造共通 |
| **出力: permissionDecision** | ○ | ◎ | — | Claude の `defer` は Codex に存在しない（Claude→Codex: dropped） |
| **出力: additionalContext / updatedInput** | ◎ | ◎ | — | — |
| **出力: terminalSequence** | ✕ | — | — | Claude のみ |
| **出力: sessionTitle / watchPaths / reloadSkills / initialUserMessage** | ✕ | — | ⏳ | SessionStart 専用。Codex 未対応 |
| **出力: updatedMCPToolOutput** | — | ✕ | — | Codex PostToolUse のみ |
| **環境変数: CLAUDE_PROJECT_DIR** | ✕ | — | — | Codex は stdin JSON のみ（環境変数非使用）。cwd フィールドで代替 |
| **環境変数: CLAUDE_ENV_FILE / CLAUDE_EFFORT** | ✕ | — | — | Codex 未対応 |

> **重要バグ**: Plugin 同梱 hooks.json が Codex でロードされない Issue #16430（2026-04 報告、open）。hooks 定義は手動でユーザー設定にコピーが必要。

**Hooks 小計**: lossless 34 / lossy 6 / dropped 43 (計 83 エントリ)

---

## 3. MCP Servers

> ソース: `mappings/mcp.yaml` / 詳細: `docs/06-mcp.md`

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **設定フォーマット（JSON⇄TOML）** | ◎ | ◎ | — | `mcpServers` ⇔ `mcp_servers`、JSON⇄TOML |
| **transport_type（type フィールド）** | ○ | ○ | — | Claude: 明示 `type`、Codex: `command`/`url` の有無で暗黙判定。変換で補完 |
| **transport: sse** | ✕ | — | — | Codex は SSE 非対応（Claude 側でも deprecated）|
| **transport: ws** | ✕ | — | — | Codex は WebSocket 非対応 |
| **command** | ◎ | ◎ | — | stdio 起動コマンド |
| **args** | ◎ | ◎ | — | コマンド引数 |
| **env** | ◎ | ◎ | — | 環境変数（リテラル値）|
| **env_vars** | — | ✕ | — | Codex 固有。ホスト環境変数の転送許可リスト。Claude に対応なし |
| **cwd** | ◎ | ◎ | — | 作業ディレクトリ |
| **url** | ◎ | ◎ | — | HTTP/SSE/WS エンドポイント |
| **headers / http_headers** | ◎ | ◎ | — | rename のみ（`headers` ⇔ `http_headers`）|
| **Bearer 認証** | ○ | ○ | — | `headers.Authorization: "Bearer ${VAR}"` ⇔ `bearer_token_env_var`。`extract:bearer_env` 変換 |
| **env_http_headers** | ○ | ○ | — | ヘッダ値を環境変数から動的取得。Claude の `${VAR}` 形式と相互変換 |
| **timeout** | ◎ | ◎ | — | 単位変換: ms（Claude）⇔ 秒 float（Codex）|
| **startup_timeout_sec** | — | ✕ | — | Codex 固有。起動タイムアウト |
| **enabled** | — | ✕ | — | Codex 固有。`false` のエントリは変換時に除外 |
| **alwaysLoad** | ✕ | — | — | Claude 固有。Tool Search / 遅延ロード機構が Codex にない |
| **headersHelper** | ✕ | — | — | Claude 固有。動的ヘッダ生成（Kerberos 等）。Codex に機構なし |
| **oauth.client_id** | ◎ | ◎ | — | rename のみ（`clientId` ⇔ `client_id`）|
| **oauth.callback_port** | ○ | ○ | — | Claude: per-server、Codex: グローバル（複数サーバー衝突リスク）|
| **oauth.scopes** | ◎ | ◎ | — | スペース区切り文字列 ⇔ 配列。`str_to_list:space` 変換 |
| **oauth.authServerMetadataUrl** | ✕ | — | — | Claude 固有（v2.1.64+）。OAuth 認可サーバーメタデータ URL の上書き |
| **oauth_resource** | — | ✕ | — | Codex 固有。RFC 8707 OAuth Resource Parameter |
| **enabled_tools / disabled_tools** | — | ✕ | — | Codex 固有。ツールのホワイトリスト・ブラックリスト。Claude に対応なし |
| **default_tools_approval_mode** | — | ✕ | — | Codex 固有。デフォルト承認モード（auto/prompt/approve）|
| **tools.<name>.approval_mode** | — | ✕ | — | Codex 固有。per-tool 承認モード上書き |
| **required** | — | ✕ | — | Codex 固有。起動失敗時に `codex exec` を終了エラーにする |
| **supports_parallel_tool_calls** | — | ✕ | — | Codex 固有。並列ツール呼び出し安全マーク |
| **environment_id** | — | ✕ | — | Codex 固有（experimental）。実行環境指定 |

**MCP 小計**: lossless 10 / lossy 4 / dropped 16 (計 30 エントリ)

---

## 4. Plugins (plugin.json / marketplace.json)

> ソース: `mappings/plugins.yaml` / 詳細: `docs/03-plugins.md`

### 4-1. マニフェスト基本メタデータ

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **マニフェストパス** | ◎ | ◎ | — | `.claude-plugin/plugin.json` ⇔ `.codex-plugin/plugin.json`（デュアルマニフェスト戦略） |
| **name** | ◎ | ◎ | — | Codex は kebab-case ≤64 文字必須 |
| **version** | ○ | ○ | — | Codex は strict semver 必須。省略時は補完が必要 |
| **description** | ◎ | ◎ | — | Codex では必須 |
| **author / homepage / repository / license / keywords** | ◎ | ◎ | — | 両者スキーマ同一 |
| **displayName** | ◎ | ◎ | — | rename のみ（`displayName` ⇔ `interface.displayName`）|
| **short-description** | ○ | ○ | — | `description` ⇔ `interface.shortDescription`（意味的に近いが 1:1 ではない）|

### 4-2. コンポーネントパス

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **skills** | ◎ | ◎ | — | デフォルト `./skills/` 共通。Claude は複数パスを許可（Codex は単一のみ）|
| **mcpServers** | ○ | ○ | — | インライン指定の場合は `.mcp.json` として書き出して参照に変換 |
| **hooks** | ○ | ○ | — | パス付け替え。イベント・タイプのマッピングは hooks 領域ルールを適用。#16430 バグに注意 |
| **commands** | △ | — | — | `skills/` ディレクトリへの移植で近似（SKILL.md ラッパー追加が必要）|
| **agents** | △ | — | — | frontmatter スキーマは subagents 領域の対応表で変換。マニフェストレベルの `agents` キーは Codex に未確認 |
| **apps** | — | ✕ | — | Codex 固有。`.app.json` OAuth コネクタ。Claude の `channels` と概念が近いがスキーマが異なり変換不可 |

### 4-3. Claude 固有フィールド（→Codex は dropped）

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **lspServers** | ✕ | — | — | Codex に LSP サポートなし |
| **outputStyles** | ✕ | — | — | Codex に出力スタイル機能なし |
| **experimental.themes** | ✕ | — | — | Codex にカラーテーマ機能なし |
| **experimental.monitors** | ✕ | — | — | Codex にバックグラウンドモニター機能なし |
| **userConfig** | ✕ | — | — | Codex にユーザー設定プロンプト機能なし |
| **channels** | ✕ | — | — | MCP サーバーを会話注入チャネルとしてバインドする機能。Codex の `.app.json` と概念は近いがスキーマ全異 |
| **settings** | ✕ | — | — | プラグイン有効時にユーザー設定にマージされる値。Codex に機能なし |
| **dependencies** | ✕ | — | — | Codex にプラグイン依存解決機能なし |
| **defaultEnabled** | △ | — | — | `policy.installation: INSTALLED_BY_DEFAULT` で近似可能だが完全非等価 |

### 4-4. Codex 固有 interface フィールド（→Claude は dropped/lossy）

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **interface.brandColor** | — | ✕ | — | Claude にブランドカラー概念なし |
| **interface.composerIcon / logo** | — | ✕ | — | Claude に受け皿なし |
| **interface.capabilities** | — | ✕ | — | `Interactive/Read/Write` の能力宣言。Claude に受け皿なし |
| **interface.screenshots** | — | ✕ | — | Claude に受け皿なし |
| **interface.websiteURL** | △ | ○ | — | Codex→Claude: `homepage` に近似変換 |
| **interface.privacyPolicyURL / termsOfServiceURL** | — | ✕ | — | Claude に受け皿なし |
| **interface.defaultPrompt** | — | △ | — | Codex→Claude: スキル本文への注記で近似 |
| **interface.developerName** | — | △ | — | Codex→Claude: `author.name` に近似 |
| **interface.category** | — | △ | — | Codex→Claude: `keywords` 配列への追加で近似 |
| **interface.longDescription** | △ | △ | — | `description` との相互近似変換 |

### 4-5. marketplace.json

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **marketplace パス** | ◎ | ◎ | — | `.claude-plugin/marketplace.json` は Codex が互換読み込みする |
| **marketplace.name** | ◎ | ◎ | — | 両者必須 |
| **marketplace.owner** | ✕ | — | — | Codex marketplace に対応なし |
| **marketplace.plugins[].source** | ○ | ○ | — | タイプ正規化が必要（Claude `relative` → Codex `{source:"local",...}`、npm は dropped） |
| **marketplace.plugins[].policy** | — | ✕ | — | Codex 固有（installation/authentication/products）。Claude marketplace にない概念 |
| **marketplace.plugins[].category** | ◎ | ◎ | — | — |
| **marketplace: interface.displayName** | △ | — | — | — |
| **allowCrossMarketplaceDependenciesOn** | ✕ | — | — | Codex 対応なし |
| **forceRemoveDeletedPlugins** | ✕ | — | — | Codex 対応なし |

**Plugins 小計**: lossless 13 / lossy 15 / dropped 20 (計 48 エントリ)

---

## 5. Memory（CLAUDE.md ⇄ AGENTS.md）

> ソース: `mappings/memory.yaml` / 詳細: `docs/08-memory-files.md`

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **ファイル名リネーム** | ◎ | ◎ | — | CLAUDE.md ⇄ AGENTS.md（AGENTS.md はオープン標準: Agentic AI Foundation）|
| **プロジェクトパス** | ◎ | ◎ | — | `./CLAUDE.md` → `./AGENTS.md`、パス付け替えのみ |
| **ユーザーグローバルパス** | ◎ | ◎ | — | `~/.claude/CLAUDE.md` ⇄ `~/.codex/AGENTS.md` |
| **@import 構文** | △ | — | — | Codex に @import 等価機構なし → インライン展開（inline_imports）。展開後 32 KiB 超過リスクあり |
| **CLAUDE.local.md** | ✕ | — | — | Codex に非コミット個人ファイルの概念なし（AGENTS.override.md はリポジトリ内配置前提）|
| **managed policy (/etc/)** | ✕ | — | — | Codex に組織強制 AGENTS.md 機構なし |
| **AGENTS.override.md** | — | ✕ | — | Codex 固有。同階層の AGENTS.md を完全置換。Claude に同等概念なし |
| **サブディレクトリ on-demand ロード** | △ | — | — | Codex は CWD より深い階層を走査しない（lossy）|
| **rules/*.md の paths frontmatter** | ✕ | — | ⏳ | パス条件付きロード。Codex に等価機構なし。Codex 実装で対応可能 |
| **HTML コメント** | ○ | ○ | — | Claude はコンテキスト注入前に除去、Codex は未定義（そのままモデルに渡る可能性）|
| **claudeMdExcludes** | ✕ | — | — | 特定ファイルを glob パターンで除外。Codex に対応なし |
| **project_doc_max_bytes** | — | △ | — | Codex の 32 KiB 上限（28 KiB 超過で warn）。Claude に上限なし |
| **project_doc_fallback_filenames** | — | ✕ | — | Codex 固有。代替ファイル名リスト |
| **features.child_agents_md** | — | ✕ | — | Codex 固有フィーチャーフラグ |
| **マージ順序（後勝ち）** | ○ | ○ | — | 両者共通だが Codex は 32 KiB 上限あり |
| **Auto memory（MEMORY.md）** | ✕ | — | — | Claude の自律書き込みメモリ。Codex に対応物なし |

**Memory 小計**: lossless 3 / lossy 5 / dropped 8 (計 16 エントリ)

---

## 6. Subagents

> ソース: `mappings/subagents.yaml` / 詳細: `docs/04-subagents.md`

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **ファイルパス** | ◎ | ◎ | — | `agents/<n>.md` ⇔ `agents/<n>.toml`（形式変換: markdown+yaml-frontmatter ⇔ toml）|
| **name** | ◎ | ◎ | — | 両者必須 |
| **description** | ◎ | ◎ | — | Claude は description セマンティックマッチで自動委譲（重要な機能差）。Codex は明示 spawn 時のガイダンスのみ |
| **本文（system prompt）** | ◎ | ◎ | — | Claude: 本文 Markdown ⇔ Codex: `developer_instructions`。内容 lossless、形式変換のみ |
| **model** | △ | △ | — | モデルプロバイダーが異なるため完全等価マッピングなし（lossy）。エイリアス非対応 |
| **effort** | ○ | ○ | — | `max` → `xhigh`、`minimal` → `low` に丸め |
| **tools（許可リスト）** | △ | — | — | Codex に個別ツール許可リスト概念なし → `sandbox_mode` で粗く近似 |
| **disallowedTools** | ✕ | — | — | Codex に個別ツール拒否リスト概念なし |
| **permissionMode** | △ | — | — | `bypassPermissions` → `danger-full-access`、`plan` → `read-only` 等で近似 |
| **maxTurns** | ✕ | — | — | Codex に直接対応なし（`job_max_runtime_seconds` は粒度が根本的に異なる）|
| **skills** | △ | △ | — | 意味が大きく異なる（Claude: 内容 inject、Codex: 有効化オーバーライド）|
| **mcpServers** | ○ | ○ | — | rename + 形式変換。名前参照形式は standalone TOML では表現困難 |
| **hooks** | △ | — | — | agent スコープ不可 → session/project hooks へ降格（`features.hooks=true` 必要）|
| **memory** | △ | — | — | 3 スコープ（user/project/local）→ グローバル boolean に降格 |
| **background** | ✕ | — | — | 常時バックグラウンド実行フラグ。Codex に対応なし |
| **isolation: worktree** | ✕ | — | — | git worktree 分離実行。Codex に概念なし |
| **color** | ✕ | — | — | UI 装飾。Codex に受け皿なし（機能影響なし）|
| **initialPrompt** | △ | — | — | auto-submit 挙動・コマンド処理は dropped → `developer_instructions` 末尾への付記で近似 |
| **自動委譲モデル** | ✕ | — | — | Claude は description セマンティックマッチで自動委譲。Codex は `spawn_agent` 明示呼び出し必須（設計の根本差）|
| **nickname_candidates** | — | ✕ | — | Codex 固有。表示ニックネーム候補プール |
| **config_file** | — | ✕ | — | Codex 固有のロールレイヤー機構。変換時はインライン展開が必要 |
| **agents.max_threads / max_depth** | — | ✕ | — | Codex グローバル設定。Claude に対応なし |
| **agents.job_max_runtime_seconds** | — | ✕ | — | Codex グローバル設定。Claude に対応なし |
| **plugin エージェント制約** | △ | — | — | Claude の plugin エージェントは hooks/mcpServers/permissionMode が無視される（Codex にこの制約なし）|

**Subagents 小計**: lossless 4 / lossy 10 / dropped 11 (計 25 エントリ)

---

## 7. Settings & Config（settings.json ⇄ config.toml）

> ソース: `mappings/settings-config.yaml` / 詳細: `docs/07-settings-and-config.md`

### 7-1. モデル・動作

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **model** | △ | △ | — | モデル ID 体系が完全に別（`claude-opus-4-6` ⇔ `gpt-5.x`）。機械変換不可 |
| **effortLevel / model_reasoning_effort** | ○ | ○ | — | `max` → `xhigh`、`minimal` → dropped |
| **language** | △ | — | — | Codex に独立フィールドなし → `developer_instructions` に追記で近似 |
| **defaultShell** | △ | △ | — | 意味が異なる。`bash` → ほぼ変換不要（既定値）。`powershell` は対応なし |
| **outputStyle** | △ | — | — | `developer_instructions` に追記で近似 |
| **autoMemoryEnabled** | △ | △ | — | Claude の 1 フィールド → Codex の `use_memories` + `generate_memories` 2 フィールド |
| **cleanupPeriodDays / max_rollout_age_days** | △ | △ | — | 意味が近いが完全同一ではない。値の範囲（0-90）に clamp |

### 7-2. 権限・サンドボックス

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **permissions.allow（Bash）** | △ | — | — | `.codex/rules/<skill>.rules`（prefix_rule allow）へ降格。スコープ拡大のため warn 必須 |
| **permissions.deny（Bash）** | △ | — | — | 同上（forbidden）|
| **permissions.ask（Bash）** | △ | — | — | 同上（prompt）|
| **permissions.allow（Read/Write）** | △ | — | — | ツール軸 → リソース軸（filesystem path）への変換。Claude の Read/Write/Edit 境界が失われる |
| **permissions.allow/deny（WebFetch）** | △ | — | — | `permissions.<n>.network.domains` へ変換。`network.enabled=true` も必要 |
| **defaultMode: bypassPermissions** | △ | — | — | `approval_policy="never"` + `sandbox_mode="danger-full-access"` で近似 |
| **defaultMode: acceptEdits** | △ | — | — | `approval_policy="untrusted"` で近似（意味に差異あり）|
| **defaultMode: auto** | △ | — | — | `approval_policy="on-request"` で近似（Codex 既定値）|
| **defaultMode: plan** | ✕ | — | — | Codex に plan モード相当なし |
| **sandbox.filesystem（allowWrite/denyWrite/allowRead/denyRead）** | △ | △ | — | `permissions.<n>.filesystem` へ変換。Claude での read/write の区別が失われる |
| **sandbox.network.allowedDomains** | △ | △ | — | 配列 → TOML table。ワイルドカード記法差に注意 |
| **sandbox.network.allowAllUnixSockets** | ◎ | ◎ | — | rename のみ（`allowAllUnixSockets` ⇔ `dangerously_allow_all_unix_sockets`）|
| **sandbox.network.allowMachLookup** | ✕ | — | — | macOS 固有の Mach lookup 許可リスト。Codex に対応なし |

### 7-3. Git・帰属・UI

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **attribution.commit / commit_attribution** | ○ | ○ | — | rename。書式差（自由記述 vs `Name <email>` 推奨）|
| **includeCoAuthoredBy** | △ | — | — | deprecated フィールド。`false` → `commit_attribution=""` で近似 |
| **editorMode / tui.vim_mode_default** | ◎ | ◎ | — | `vim` ⇔ `true`（lossless な rename + enum→bool）|
| **statusLine** | ✕ | — | — | Codex に等価物なし（`tui.status_line` は意味が異なる）|

### 7-4. Skills・Plugins 関連

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **skillOverrides** | △ | △ | — | Claude は `on/off/name-only/user-invocable-only` 4 値 → Codex は `enabled: true/false` 2 値のみ |
| **enabledPlugins** | △ | △ | — | プラグイン体系が異なる。同一プラグインが両方に存在する場合のみ変換可 |

### 7-5. 環境変数・その他

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **env** | △ | △ | — | Claude: セッション全体への注入、Codex: サブプロセスのみ。`ANTHROPIC_API_KEY` 等は Codex では効かない |

### 7-6. Claude 固有（→Codex は dropped）

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **viewMode** | ✕ | — | — | Codex に対応する表示モード区分なし |
| **worktree** | ✕ | — | — | Codex に git worktree 管理機能なし |
| **autoUpdatesChannel** | ✕ | — | — | Codex に等価設定なし |
| **spinnerTips** | ✕ | — | — | Claude UI 固有 |
| **voice / voiceEnabled** | ✕ | — | — | Claude の音声ディクテーション機能 |
| **maxSkillDescriptionChars** | ✕ | — | — | Claude 固有の skill コンテキスト予算調整 |

### 7-7. Codex 固有（→Claude は dropped/lossy）

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **profiles（~/.codex/<name>.config.toml）** | — | ✕ | — | Codex の名前付き設定プロファイル。Claude にプロファイル概念なし |
| **permissions.<n>.extends** | — | ✕ | — | 権限プロファイル継承。Claude に概念なし |
| **approval_policy.granular.\*** | — | ✕ | — | granular 承認ポリシー。Claude に個別承認カテゴリ分離なし |
| **agents.max_threads / max_depth** | — | ✕ | — | エージェント並列度・深度設定。Claude に対応なし |
| **otel.\*** | — | △ | — | Claude は環境変数 `OTEL_*` 経由で近似可能 |
| **tui.keymap.\*** | — | ✕ | — | プログラマブルキーバインド。Claude に対応なし |
| **model_verbosity / model_reasoning_summary** | — | ✕ | — | GPT-5 応答冗長性設定 |
| **web_search** | — | ✕ | — | Codex の web search 専用フラグ |
| **project_doc_max_bytes** | — | △ | — | Memory 領域と重複。AGENTS.md サイズ制限 |
| **developer_instructions** | — | △ | — | CLAUDE.md（プロジェクト/ユーザー）への追記で近似 |

**Settings 小計**: lossless 2 / lossy 30 / dropped 17 (計 49 エントリ)

---

## 8. Variables & Templating

> ソース: `mappings/variables.yaml` / 詳細: `docs/09-variables-and-templating.md`

| 機能 | c2x | x2c | 将来追従 | 備考 |
|---|---|---|---|---|
| **$ARGUMENTS（全体）** | △ | △ | — | 記法共通だが Codex Skill 本体では展開なし（Custom Prompts のみ有効・deprecated）|
| **$ARGUMENTS[N]（0-indexed）** | ○ | ○ | — | インデックスずれ: Claude 0 基点 ⇔ Codex 1 基点（`index_shift:+1`）。最重要の落とし穴 |
| **$N（位置引数 shorthand）** | ○ | ○ | — | 同上。`$0→$1`, `$1→$2` の変換が必要 |
| **$name（名前付き引数）** | ○ | ○ | — | Claude: frontmatter 宣言 + 位置渡し、Codex: KEY=value 形式。rename（小文字→大文字）|
| **${CLAUDE_SESSION_ID}** | ✕ | — | — | Codex に同等変数なし。リテラル化して誤動作リスク |
| **${CLAUDE_EFFORT}** | ✕ | — | — | 同上 |
| **${CLAUDE_SKILL_DIR}** | ✕ | — | — | 同上 |
| **${CLAUDE_PROJECT_DIR}** | ✕ | — | — | Codex では `$PWD` / `$REPO_ROOT` で代替検討 |
| **${CLAUDE_PLUGIN_ROOT}** | ✕ | — | — | Codex に plugin スコープ変数なし |
| **!`cmd`（インライン動的注入）** | ✕ | — | — | Codex Issue #5019 で "not planned"。リテラル化して誤動作リスク（高危険）|
| **\`\`\`! ブロック動的注入** | ✕ | — | — | 同上（高危険）|
| **$$ エスケープ** | — | ✕ | — | Codex Custom Prompts のエスケープシーケンス。Claude に同等機構なし |
| **/skill-name ⇄ $skill-name（呼び出し記法）** | ○ | ○ | — | 本文・README の呼び出し記法変換。自動変換は誤検出リスクあり |
| **/plugin:skill（名前空間付き記法）** | ✕ | — | — | Codex に名前空間記法なし。名前空間除去すると同名 skill 衝突リスク |

**Variables 小計**: lossless 0 / lossy 5 / dropped 9 (計 14 エントリ)

---

## 9. 将来追従の仕組み

本リポジトリの変換テーブルは `mappings/*.yaml` のエントリ単位で管理されているため、Codex に機能が実装されたら対応する YAML エントリの `loss` を `dropped` → `both/lossy` に、`direction` を `claude_to_codex` → `both` に更新するだけで変換 CLI 本体の改修なく追従できる。たとえば `user-invocable`・`paths` 自動発火・`http`/`mcp_tool` hook タイプ・Claude 固有 hook イベント（20 件）・`once`/`if`/`asyncRewake` 等の各エントリはすべて「Codex 側実装待ち」として ⏳ を付与しており、Codex の実装状況を定期確認してエントリを更新するだけで対応できる設計になっている。このワークフローの詳細は [`docs/12-cli-spec.md`](12-cli-spec.md) §9 を参照。
