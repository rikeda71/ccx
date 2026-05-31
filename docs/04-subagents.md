# Subagents: Claude Code ⇄ Codex

> Claude Code の `agents/*.md`（Markdown + YAML frontmatter、全 16 フィールド）と Codex の 2 系統（config.toml `[agents]` ロールレイヤー / standalone `.toml`）は概念的に対応するが、起動モデル・フィールド体系・スコープ設計が大きく異なる。最大の差は **Claude がマッチした description から自動委譲するのに対し、Codex は `spawn_agent` の明示呼び出しが必須**という点で、多くの自動委譲フローが再現不可になる。

## 0. 概要

両者に「サブエージェント」概念は存在し、固有の system prompt・ツール制限・モデルで独立した context window を持つ点も共通する。しかし設計思想が異なる。

- **Claude Code**: `agents/*.md` という Markdown + YAML frontmatter ファイルを置くだけでエージェントが登録され、description のセマンティックマッチによって自動委譲される。`permissionMode`・`mcpServers`・`hooks`・`isolation` など 16 フィールドで細粒度に制御可能。
- **Codex**: 2 系統の設定を持つ。**系統 A** は `config.toml` の `[agents.<name>]`（ロール宣言 + `config_file` で別 TOML レイヤーを参照）。**系統 B** は standalone TOML（`~/.codex/agents/<name>.toml` / `.codex/agents/<name>.toml`）で、名前・説明・developer_instructions を持つ。どちらの場合もエージェントは自動委譲されず、**`spawn_agent` ツールを明示的に呼び出した場合のみ起動する**。

変換の非対称性: Codex→Claude は比較的 lossless。Claude→Codex は `permissionMode`（一部対応のみ）・`hooks`（粒度違い）・`mcpServers`（形式変換）・`maxTurns`（time-based 近似のみ）・`background`・`isolation:worktree`・`color`・`initialPrompt`（auto-submit 挙動）など多くが lossy または dropped になる。

---

## 1. Claude Code 側の仕様

### 配置・ファイル・スコープ

| スコープ | パス | 優先順位 |
|---|---|---|
| managed（組織強制） | managed settings の `.claude/agents/` 経由 | 1（最高） |
| CLI（セッション限定） | `--agents` フラグで JSON 渡し | 2 |
| project | `.claude/agents/<name>.md`（再帰探索可） | 3 |
| user（全プロジェクト） | `~/.claude/agents/<name>.md`（再帰探索可） | 4 |
| plugin | `<plugin-root>/agents/<name>.md` | 5（最低） |

- 同名エージェントは優先順位の高い方が勝つ
- サブディレクトリ構成は identity に影響しない（`name` frontmatter が識別子）
- plugin 内エージェントのみ `hooks`・`mcpServers`・`permissionMode` の 3 フィールドが**無視**される（セキュリティ制約）
- セッション開始時にロード（`/agents` UI 経由の作成・編集は即時反映）

### 全フィールド表

| フィールド | 型 | 必須 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|---|
| `name` | string（kebab-case、lowercase 推奨） | 必須 | — | agent | エージェントの一意識別子。hooks の `agent_type` としても使われる |
| `description` | string | 必須 | — | agent | Claude がいつ委譲するかを決める説明文。セマンティックマッチによる自動委譲トリガー |
| `tools` | string / list | 任意 | （継承） | agent | 許可するツールの allowlist。`Agent(worker,researcher)` 構文で spawn 可能なサブエージェントを制限できる |
| `disallowedTools` | string / list | 任意 | — | agent | 禁止するツールの denylist。`tools` より先に適用 |
| `model` | string | 任意 | `inherit` | agent | 使用モデル。`sonnet`/`opus`/`haiku`/フル model ID（例: `claude-opus-4-8`）/`inherit` |
| `permissionMode` | enum | 任意 | （親継承） | agent | `default`/`acceptEdits`/`auto`/`dontAsk`/`bypassPermissions`/`plan`。plugin エージェントでは無視 |
| `maxTurns` | integer | 任意 | — | agent | エージェントが停止するまでの最大ターン数 |
| `skills` | list（string） | 任意 | — | agent | 起動時にコンテキストに full inject するスキル名リスト |
| `mcpServers` | list | 任意 | — | agent | このエージェントに紐付ける MCP サーバー定義（inline または名前参照）。plugin エージェントでは無視 |
| `hooks` | object | 任意 | — | agent | このエージェント実行中にのみ有効な lifecycle hooks。plugin エージェントでは無視 |
| `memory` | enum | 任意 | — | agent | 永続メモリのスコープ: `user`/`project`/`local` |
| `background` | boolean | 任意 | `false` | agent | `true` にすると常にバックグラウンド実行（並列、権限プロンプトは auto-deny） |
| `effort` | enum | 任意 | （セッション継承） | agent | `low`/`medium`/`high`/`xhigh`/`max`。セッションの effort 設定を上書き |
| `isolation` | enum | 任意 | — | agent | `worktree` のみ有効。一時的な git worktree で分離実行 |
| `color` | enum | 任意 | — | agent | UI 表示色: `red`/`blue`/`green`/`yellow`/`purple`/`orange`/`pink`/`cyan` |
| `initialPrompt` | string | 任意 | — | agent | `--agent` フラグや `agent` 設定でメインセッションとして起動した場合、最初の user turn として自動 submit される初期プロンプト。コマンド・スキルも処理される |

**本文（Markdown body）**

ファイルの frontmatter 以降の Markdown が、エージェントの system prompt になる。Claude Code 標準 system prompt の代わりに使用される（CLAUDE.md やメモリは通常通りロード）。

### permissionMode 詳細

| 値 | 動作 |
|---|---|
| `default` | 標準の権限チェック＋プロンプト |
| `acceptEdits` | ワーキングディレクトリ内のファイル編集を自動承認 |
| `auto` | バックグラウンド分類器がコマンドをレビュー |
| `dontAsk` | 権限プロンプトを自動拒否（明示許可ツールは動作） |
| `bypassPermissions` | 全権限プロンプトをスキップ（.git 等への書き込みも可） |
| `plan` | 読み取り専用の計画モード |

親が `bypassPermissions` / `acceptEdits` を使っている場合はその設定が優先され、エージェント側の `permissionMode` は無視される。

---

## 2. Codex 側の仕様

### 配置・ファイル・スコープ

Codex のエージェント設定は 2 系統が共存する。

#### 系統 A: config.toml ロール宣言

| 設定箇所 | パス | 説明 |
|---|---|---|
| グローバル `[agents]` | `~/.codex/config.toml` / `.codex/config.toml` | エージェント全体の上限設定 |
| ロール `[agents.<name>]` | 同上（config layer stack でオーバーレイ可） | 個別エージェントロール定義（`config_file` で別 TOML を参照） |

#### 系統 B: standalone エージェント TOML

| スコープ | パス | 説明 |
|---|---|---|
| user | `~/.codex/agents/<name>.toml` | 全プロジェクト共通のユーザーエージェント |
| project | `.codex/agents/<name>.toml` | プロジェクトローカルのエージェント |

standalone TOML は discovery 時に自動で `[agents.<name>]` ロールとして認識される（`config_file` 経由と等価）。サブディレクトリも再帰探索される。

### 全フィールド表（グローバル `[agents]`）

| フィールド | 型 | 必須 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|---|
| `max_threads` | integer（≥1） | 任意 | 6 | global | 同時に開けるエージェントスレッドの最大数 |
| `max_depth` | integer（≥1） | 任意 | 1 | global | spawned agent のネスト深度上限（root は深度 0） |
| `job_max_runtime_seconds` | integer（≥1） | 任意 | 1800 | global | `spawn_agents_on_csv` のワーカー 1 件あたりのデフォルトタイムアウト（秒） |
| `interrupt_message` | boolean | 任意 | `true` | global | エージェントターンが中断された際にモデル可視メッセージを記録するか |

### 全フィールド表（`[agents.<name>]` ロール宣言）

| フィールド | 型 | 必須 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|---|
| `description` | string | 必須（ロールファイル参照がない場合） | — | subagent | `spawn_agent` 呼び出し時のガイダンステキスト。standalone TOML の `name` が設定された場合そちらが優先 |
| `config_file` | string（パス） | 任意 | — | subagent | ロール別 config TOML レイヤーへのパス。standalone TOML ファイルを指定することで系統 B と連携 |
| `nickname_candidates` | list（string） | 任意 | — | subagent | spawn されたエージェントインスタンスの表示ニックネーム候補プール |

### 全フィールド表（standalone エージェント TOML）

standalone TOML は `RawAgentRoleFileToml` として解釈され、`name`・`description`・`nickname_candidates` を除く残りの全フィールドは **ConfigToml 互換キー**（通常の `config.toml` と同じキー）として扱われる。

| フィールド | 型 | 必須 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|---|
| `name` | string | 必須（discovery 経由の場合） | — | subagent | エージェントの識別名。`[agents.<name>]` のロール名として登録される |
| `description` | string | 必須 | — | subagent | `spawn_agent` 時のガイダンステキスト |
| `developer_instructions` | string | 必須（`config_file` 直参照の場合） | — | subagent | エージェントの system prompt 相当。`developer` ロールメッセージとして挿入される |
| `nickname_candidates` | list（string） | 任意 | — | subagent | 表示ニックネーム候補 |
| `model` | string（model ID） | 任意 | （親セッション継承） | subagent | 使用モデル。フル model ID 直書き（エイリアス `sonnet`/`opus`/`haiku` は不可） |
| `model_reasoning_effort` | enum | 任意 | — | subagent | `minimal`/`low`/`medium`/`high`/`xhigh`（`max` は存在しない点に注意） |
| `sandbox_mode` | enum | 任意 | （親継承） | subagent | `read-only`/`workspace-write`/`danger-full-access` |
| `approval_policy` | enum / object | 任意 | （親継承） | subagent | `untrusted`/`on-request`/`never` または granular object |
| `mcp_servers` | object | 任意 | — | subagent | MCP サーバー定義（ConfigToml の `mcp_servers` と同じ形式） |
| `skills.config` | list（object） | 任意 | — | subagent | スキル有効化オーバーライド（`path`・`enabled` フィールドを持つ object の配列） |

### エージェント起動方式（重要）

Codex のエージェントは **`features.multi_agent = true`（既定 on）の状態で `spawn_agent` ツールを明示的に呼び出した場合にのみ起動する**。Claude Code のように description のセマンティックマッチによる自動委譲は行われない。

```toml
# config.toml
[features]
multi_agent = true   # 既定で有効

[agents.researcher]
description = "Research-focused role."
config_file = "./agents/researcher.toml"
nickname_candidates = ["Herodotus", "Ibn Battuta"]
```

組み込みロール（built-in）: `default`（汎用）、`worker`（実装重視）、`explorer`（読み取り重視のコード解析）の 3 種が用意されており、同名のカスタム定義で上書き可能。

---

## 3. 変換テーブル

`mappings/subagents.yaml` の人間可読版。

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `subagents.path` | `.claude/agents/<name>.md` / `~/.claude/agents/<name>.md` | `.codex/agents/<name>.toml` / `~/.codex/agents/<name>.toml` | both | lossless | — | `path:remap`（`.claude/agents/` ⇄ `.codex/agents/`）+ 拡張子変換（`.md` ⇄ `.toml`）+ 形式変換 |
| `subagents.name` | `name`（frontmatter） | `name`（standalone TOML トップレベル） | both | lossless | — | kebab-case 規約は Claude 側の慣習。Codex 側の制約は緩い |
| `subagents.description` | `description`（frontmatter） | `description`（standalone TOML / `[agents.<name>].description`） | both | lossless | — | 意味一致。Claude は自動委譲トリガー、Codex は `spawn_agent` 呼び出し時のガイダンス |
| `subagents.body` | 本文 Markdown（system prompt） | `developer_instructions`（standalone TOML） | both | lossless | — | Claude 本文 ⇄ Codex `developer_instructions`。Markdown→TOML 形式変換（`format:json_to_toml`） |
| `subagents.model` | `model`（`sonnet`/`opus`/`haiku`/フル ID/`inherit`） | `model`（フル model ID 直書き） | both | lossy | — | `enum_map:{sonnet:claude-sonnet-...,opus:claude-opus-...,haiku:claude-haiku-...,inherit:（省略）}` でエイリアス ⇄ フル ID 変換。`inherit` は Codex では省略に相当。warn:true |
| `subagents.effort` | `effort`（`low`/`medium`/`high`/`xhigh`/`max`） | `model_reasoning_effort`（`minimal`/`low`/`medium`/`high`/`xhigh`） | both | lossy | — | `enum_map:{max:xhigh}`。Claude の `max` は Codex に存在しないため `xhigh` に丸め。`minimal` は Claude に存在せず Codex→Claude は `low` に丸め。warn:true |
| `subagents.tools` | `tools`（個別ツール allowlist、`Agent(x,y)` 構文含む） | `sandbox_mode` で粗く近似 | claude_to_codex | lossy | — | Codex に個別ツール許可リストがない。`sandbox_mode`（`read-only`/`workspace-write`/`danger-full-access`）で粗く近似。`Agent(x,y)` 構文のサブエージェント spawn 制限は dropped。warn:true |
| `subagents.disallowedTools` | `disallowedTools`（denylist） | （対応なし） | claude_to_codex | dropped | — | Codex に個別ツール拒否リストがない。warn:true |
| `subagents.permissionMode` | `permissionMode`（6 値） | `sandbox_mode` + `approval_policy` で部分対応 | claude_to_codex | lossy | — | `default`→`workspace-write` + `on-request`。`bypassPermissions`→`danger-full-access`。`plan`→`read-only` + `approval_policy:never`（近似）。`acceptEdits`/`auto`/`dontAsk` は対応なし→dropped。warn:true |
| `subagents.maxTurns` | `maxTurns`（ターン数） | （直接対応なし）`agents.job_max_runtime_seconds`（時間ベース） | claude_to_codex | dropped | — | ターン数と秒数は粒度が異なり等価変換不可。warn:true |
| `subagents.skills` | `skills`（list）、起動時 full inject | `skills.config`（有効化オーバーライドのみ） | both | lossy | — | Claude は full inject、Codex は `enabled`/`path` のみで内容 inject ではない。近似にとどまる。warn:true |
| `subagents.mcpServers` | `mcpServers`（inline 定義または名前参照） | `mcp_servers`（standalone TOML 内） | both | lossy | — | `rename`（`mcpServers` ⇄ `mcp_servers`）+ `format:json_to_toml`。スコープ差: Claude は当該エージェント実行中のみ接続/切断、Codex は subagent 設定内で同様に管理。plugin エージェントでは Claude 側も無視。warn:true |
| `subagents.hooks` | `hooks`（エージェント frontmatter 内、全イベント対応） | `hooks.<Event>`（config.toml / hooks.json、粒度違い） | claude_to_codex | lossy | — | Claude はエージェントスコープ、Codex は session/project スコープに降格。`features.hooks = true` が必要。エージェント終了時の `Stop` → `SubagentStop` 変換は Codex では設定レベルに移る。warn:true |
| `subagents.memory` | `memory`（`user`/`project`/`local` スコープ） | `features.memories` + `memories.generate_memories` / `memories.use_memories` | claude_to_codex | lossy | — | Claude は 3 スコープ（user/project/local）でディレクトリも分離。Codex は memories 機能全体の on/off のみでスコープ分離なし。warn:true |
| `subagents.background` | `background`（boolean） | （対応なし） | claude_to_codex | dropped | — | Codex に「常時バックグラウンド実行」フラグがない。warn:true |
| `subagents.isolation` | `isolation: worktree` | （対応なし） | claude_to_codex | dropped | — | Codex に git worktree isolation フラグがない。warn:true |
| `subagents.color` | `color`（UI 表示色） | （対応なし） | claude_to_codex | dropped | — | UI 装飾フィールド。Codex に受け皿なし |
| `subagents.initialPrompt` | `initialPrompt`（auto-submit 初期プロンプト） | `developer_instructions` 末尾への付記（近似） | claude_to_codex | lossy | — | `initialPrompt` は `--agent` / `agent` 設定でメインセッション起動時のみ auto-submit される。Codex には auto-submit 挙動がないため `developer_instructions` 末尾に付記することで「モデルへの初期指示」として近似するが、auto-submit 挙動は dropped。warn:true |
| `subagents.nickname_candidates` | （対応なし） | `nickname_candidates`（`[agents.<name>]` / standalone TOML） | codex_to_claude | dropped | — | Codex 固有。UI 表示ニックネーム候補。Claude に受け皿なし |
| `subagents.config_file` | （対応なし） | `config_file`（ロールレイヤーへのパス） | codex_to_claude | dropped | — | Codex 固有のロールレイヤー機構。Claude に受け皿なし |
| `subagents.spawn-model` | 自動委譲（description マッチ） | `spawn_agent` ツールの明示呼び出し | — | dropped | — | 変換で再現できない根本的な設計差。Claude→Codex 変換時は必ず警告レポートに明記 |
| `subagents.plugin-restrictions` | `hooks`/`mcpServers`/`permissionMode` 有効（plugin 外エージェント） | （plugin エージェントとの概念差異） | — | — | — | Claude plugin エージェントではこの 3 フィールドが無視される点を変換時に注記すること |

---

## 4. 変換時の注意・既知の落とし穴

### 4.1 「自動委譲 vs 明示 spawn」：最大の落とし穴

Claude Code は description のセマンティックマッチによってエージェントへの委譲を**自動で**行う。Codex は `features.multi_agent = true`（既定 on）の状態でも、エージェントへの委譲は **`spawn_agent` ツールの明示呼び出しが必須**。

> "Codex only spawns a new agent when you explicitly ask it to do so."

Claude→Codex 変換では、「Claude が自動委譲していたワークフローが Codex で動かない」という実行時の落とし穴が最も頻繁に報告される。変換レポートには必ず「Codex ではエージェントの自動委譲は発生しない。AGENTS.md または developer_instructions にエージェントを明示的に spawn する指示を追加することを推奨する」と注記すること。

### 4.2 plugin エージェントにおける無視フィールド

Claude の **plugin エージェント**（plugin の `agents/` ディレクトリに配置）では、`hooks`・`mcpServers`・`permissionMode` の 3 フィールドがセキュリティ制約により**無視**される。変換時にこれらフィールドを持つエージェントを plugin として配布する場合は、無視される旨を警告すること。

回避策: エージェントファイルを `.claude/agents/` または `~/.claude/agents/` にコピーする。

### 4.3 `permissionMode` の部分対応

Claude の `permissionMode` 6 値のうち Codex で近似できるのは一部のみ:

| Claude `permissionMode` | Codex 近似 | 損失 |
|---|---|---|
| `default` | `sandbox_mode: workspace-write` + `approval_policy: on-request` | lossy |
| `bypassPermissions` | `sandbox_mode: danger-full-access` | lossy（回路遮断の差異あり） |
| `plan` | `sandbox_mode: read-only` + `approval_policy: never`（近似） | lossy |
| `acceptEdits` | 対応なし | dropped |
| `auto` | 対応なし | dropped |
| `dontAsk` | 対応なし | dropped |

### 4.4 `maxTurns` の粒度差

Claude の `maxTurns` はターン数カウントによる上限で、Codex の `agents.job_max_runtime_seconds` は時間ベースのタイムアウト。粒度が根本的に異なるため等価変換は不可能。Claude→Codex 変換では dropped として処理し、対応する時間制限の設定（`job_max_runtime_seconds`）を手動で検討するよう警告すること。

### 4.5 `model` エイリアスのマッピング

Claude の `model` フィールドはエイリアス（`sonnet`/`opus`/`haiku`）を受け付けるが、Codex standalone TOML の `model` はフル model ID 直書きが必要（例: `gpt-4o`）。変換時は可能な範囲でエイリアスをフル ID にマッピングし、unmapped のエイリアスは warn + 手動確認を促すこと。`inherit`（Claude デフォルト）は Codex では省略（親セッションから継承）に相当する。

### 4.6 `effort: max` → `xhigh` への丸め

Claude の `effort` には `max` 値があるが、Codex の `model_reasoning_effort` の最大値は `xhigh`。Claude→Codex 変換では `max` を `xhigh` に丸める。逆方向（Codex `minimal` → Claude）は `low` に丸める。いずれも warn:true。

### 4.7 `skills` フィールドの意味差

Claude の `skills` は**スキルのフル内容をコンテキストに inject**する。Codex の `skills.config` は**スキルの有効化オーバーライド**（`enabled`/`path`）のみで、内容 inject ではない。変換では近似として扱うが、動作に大きな差が生じる可能性があるため warn。

### 4.8 `hooks` のスコープ降格

Claude はエージェントの frontmatter に `hooks` を書くことでそのエージェント実行中だけ有効な hooks を定義できる。Codex では hooks の設定先が `config.toml` または `hooks.json`（session/project スコープ）に限られ、エージェントスコープに閉じることができない。変換後は全セッションに hooks が効くことを警告すること。

### 4.9 `memory` スコープ分離の喪失

Claude の `memory` は `user`（`~/.claude/agent-memory/<name>/`）・`project`（`.claude/agent-memory/<name>/`）・`local`（`.claude/agent-memory-local/<name>/`）の 3 スコープでディレクトリも独立している。Codex の `features.memories` はメモリ機能全体の on/off（`generate_memories`/`use_memories`）のみで、スコープ分離の概念がない。変換では lossy として扱い、スコープ意図を notes に残すこと。

### 4.10 `isolation: worktree` と `background: true` は dropped

Claude 固有の git worktree 分離実行（`isolation: worktree`）とバックグラウンド常時実行フラグ（`background: true`）は Codex に対応機構がない。変換時は dropped として処理し、ワークフローの変更を手動で検討するよう警告すること。

### 4.11 `initialPrompt` の auto-submit 挙動

Claude の `initialPrompt` は `--agent` / `agent` 設定でメインセッション起動時にのみ auto-submit される特殊なフィールドで、コマンド・スキルの処理も行われる。Codex には auto-submit の概念がなく、`developer_instructions` 末尾への付記で「モデルへの初期指示」として近似するが、auto-submit 挙動は再現できない。

### 4.12 Codex 固有フィールドの Claude への変換

- `nickname_candidates`（表示ニックネーム候補）: Claude に受け皿なし → dropped
- `config_file`（ロールレイヤーパス）: Claude のスコープ体系には対応概念なし → dropped
- `interrupt_message`（グローバル設定）: Claude に同等設定なし → dropped

---

## 5. 出典

- Claude Code Subagents: https://code.claude.com/docs/en/sub-agents
- Claude Code Plugins Reference (agents section): https://code.claude.com/docs/en/plugins-reference
- Codex Subagents: https://developers.openai.com/codex/subagents
- Codex Config Reference: https://developers.openai.com/codex/config-reference
- Codex Config Advanced: https://developers.openai.com/codex/config-advanced
- GitHub: openai/codex（codex-rs/config/src/config_toml.rs, codex-rs/core/src/config/agent_roles.rs）: https://github.com/openai/codex
