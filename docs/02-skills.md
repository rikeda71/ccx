# Skills: Claude Code ⇄ Codex

> SKILL.md のファイル名・ディレクトリ構成は両者でほぼ同一だが、frontmatter フィールドに決定的な非対称性がある。Claude Code が 16 フィールドを持つのに対し Codex の実行時 skill loader が認識するのは `name`/`description`（と `metadata.short-description`）のみで、残りは補助ファイル `agents/openai.yaml`・session/subagent スコープへの「降格」で部分代替するしかない。なお Codex の実行時 loader は `deny_unknown_fields` を使っていないため、Claude 固有 frontmatter を含む SKILL.md を読んでもエラーにならず `name`/`description` だけ使って残りを黙って無視してロードに成功する（fail open）。Codex→Claude 変換はほぼ無損失、Claude→Codex 変換は 14/16 フィールドが要注意。

## 0. 概要

Skills は両者で**ファイル名まで同一**（`SKILL.md`）という珍しい概念一致を示す。YAML frontmatter + Markdown 本文という構造も共通。相互変換の難易度は非対称で、Codex から Claude へは受け皿が十分にありほぼ無損失だが、Claude から Codex へは frontmatter の表現力差が大きく、多くのフィールドが skill スコープでは再現不可能で session / subagent スコープへの降格が必要になる。`user-invocable`・`paths`（glob 自動発火）・`arguments`（引数機構）・組み込みツール禁止は確定損失。なお Claude 固有の frontmatter フィールドを含む SKILL.md を Codex が読み込んでもエラーにはならず、`name`/`description` だけ使って残りを黙って無視する（fail open）。

呼び出し記法が **`/skill-name`（Claude）⇔ `$skill-name`（Codex）** と異なるため、本文・README 中の記法も変換対象となる。

## 1. Claude Code 側の仕様

### 配置・ファイル・スコープ

| スコープ | パス | 優先順位 |
|---|---|---|
| enterprise | managed settings 経由 | 最高 |
| personal（全プロジェクト） | `~/.claude/skills/<name>/SKILL.md` | 高 |
| project | `.claude/skills/<name>/SKILL.md` | 中 |
| plugin 内 | `<plugin-root>/skills/<name>/SKILL.md` | plugin スコープ |
| legacy command（後方互換） | `.claude/commands/<name>.md` | — |

- 補助ファイル: `scripts/`（コンテキスト非搭載・出力のみ）、`references/`、任意 `.md`、`assets/`（慣習。必須構造ではない）
- 起動ディレクトリからリポジトリルートまで遡って自動探索 + サブディレクトリも on-demand 探索
- 本文推奨上限: 500 行
- スキルリスト予算: コンテキストの約 1%
- 呼び出し記法: `/skill-name`（明示）、`description` + `when_to_use` の意味マッチ（自動）
- 動的注入: `` !`cmd` `` / ` ```! ` ブロックでシェル実行結果を本文に埋込可

### 全フィールド表

| フィールド | 型 | 必須 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|---|
| `name` | string (≤64, lowercase 英数ハイフン, "claude"/"anthropic" 禁止) | 必須 | — | skill | Skill の識別名。`/name` で呼び出し |
| `description` | string (≤1024) | 必須 | — | skill | 自動発火トリガーにもなる説明文 |
| `when_to_use` | string | 任意 | — | skill | 自動発火をより細かく制御する発火条件の補足説明 |
| `argument-hint` | string | 任意 | — | skill | `/name` 呼び出し時に UI に表示される引数ヒント |
| `arguments` | string / list (`$name` 名前付き位置引数) | 任意 | — | skill | 本文内で `$name` 置換される名前付き引数の定義 |
| `disable-model-invocation` | boolean | 任意 | false | skill | `true` にすると暗黙（自動）発火を禁止。ただし明示呼び出しも阻害しうる既知問題あり |
| `user-invocable` | boolean | 任意 | true | skill | `false` にするとユーザーが `/name` で呼べなくなり、モデルのみ自動発火可能 |
| `allowed-tools` | string / list（pre-approve、ワイルドカード可） | 任意 | — | skill | skill 実行中だけ事前承認するツール指定 |
| `disallowed-tools` | string / list | 任意 | — | skill | skill 実行中だけ禁止するツール指定（組み込みツール `AskUserQuestion` 等も可） |
| `model` | string (`/model` の値 / `inherit`) | 任意 | inherit | skill | skill 実行に使うモデル |
| `effort` | enum (low / medium / high / xhigh / max) | 任意 | — | skill | 推論 effort レベル |
| `context` | enum (fork) | 任意 | — | skill | `fork` を指定するとサブエージェントとして分岐実行 |
| `agent` | string | 任意 | — | skill | `context: fork` 時に起動する subagent タイプ |
| `hooks` | object | 任意 | — | skill | skill スコープで有効な lifecycle hooks の定義 |
| `paths` | string / list（glob） | 任意 | — | skill | このパターンにマッチするファイルが操作された場合に自動発火 |
| `shell` | enum (bash / powershell) | 任意 | bash | skill | スクリプト実行に使用するシェル |

### 本文内変数

| 変数 | 説明 |
|---|---|
| `$ARGUMENTS` | 明示呼び出し時に渡された引数全体 |
| `$N`（`$1`, `$2`, …） | 位置引数 |
| `$name` | `arguments` で定義した名前付き引数 |
| `${CLAUDE_SESSION_ID}` | 現在のセッション ID |
| `${CLAUDE_EFFORT}` | 現在の effort 設定 |
| `${CLAUDE_SKILL_DIR}` | skill ディレクトリへの絶対パス |

---

## 2. Codex 側の仕様

### 配置・ファイル・スコープ

| スコープ | パス | 優先順位 |
|---|---|---|
| repo (CWD) | `$CWD/.agents/skills/<name>/SKILL.md` | 最高（REPO） |
| repo root | `$REPO_ROOT/.agents/skills/<name>/SKILL.md` | REPO |
| user | `~/.agents/skills/<name>/SKILL.md` | USER |
| admin | `/etc/codex/skills/<name>/SKILL.md` | ADMIN |
| system / bundled | Codex 同梱 | SYSTEM |
| plugin 内 | `<plugin-root>/skills/<name>/SKILL.md` | plugin スコープ |

- 優先順位（高い順）: REPO > USER > ADMIN > SYSTEM
- 補助ファイル: `scripts/`、`references/`、`assets/`、**`agents/openai.yaml`**（Codex 固有の UI/ポリシー設定）
- シンボリックリンク追跡あり
- スキルリスト予算: コンテキストの約 2% または約 8000 文字
- 呼び出し記法: `$skill-name`（明示）または `/skills` メニュー、`description` の意味マッチ（自動）

### 全フィールド表（SKILL.md frontmatter）

| フィールド | 型 | 必須 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|---|
| `name` | string | 必須 | — | skill | Skill の識別名。`$name` で呼び出し |
| `description` | string | 必須 | — | skill | 自動発火トリガーにもなる説明文。公式ガイドは「**他のフィールドを frontmatter に含めるな**」と明記。ただし実行時 skill loader（`SkillFrontmatter` 構造体）は `deny_unknown_fields` を使っておらず、Claude 固有フィールド（`allowed-tools` 等）を含む SKILL.md を読んでもエラーにならず `name`/`description` だけ使って残りを**黙って無視して**ロードに成功する（fail open）。reject ではない |

### 全フィールド表（agents/openai.yaml）

`agents/openai.yaml` は SKILL.md ディレクトリ配下に置く補助ファイルで、Codex 固有の UI・ポリシー設定を担う。**このファイルが存在しない場合は `policy = None` となり `allow_implicit_invocation` はデフォルト `true`**（loader の "Fail open: optional metadata should not block loading SKILL.md" 設計による）。

| フィールド | 型 | 必須 | デフォルト | スコープ | 説明 |
|---|---|---|---|---|---|
| `policy.allow_implicit_invocation` | boolean | 任意 | true | skill | `false` にするとモデルによる暗黙発火を禁止。明示呼び出し（`$name`）は引き続き機能する |
| `policy.products` | list | 任意 | — | skill | この skill を利用可能な product リスト |
| `interface.display_name` | string | 任意 | — | skill | UI 表示名 |
| `interface.short_description` | string | 任意 | — | skill | UI 向け短縮説明 |
| `interface.icon_small` | string (path) | 任意 | — | skill | 小アイコンのパス |
| `interface.icon_large` | string (path) | 任意 | — | skill | 大アイコンのパス |
| `interface.brand_color` | string (hex) | 任意 | — | skill | ブランドカラー |
| `interface.default_prompt` | string | 任意 | — | skill | `$skill-name` 実行時の前置プロンプト |
| `dependencies.tools` | list | 任意 | — | skill | この skill が依存する MCP ツールの宣言（`features.skill_mcp_dependency_install` が制御） |

### config.toml の SkillConfig（スキル有効化制御）

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `name` | string | 必須 | skill 名 |
| `enabled` | boolean | 任意 | `false` で無効化。additionalProperties: false により他フィールド不可 |
| `path` | string | 任意 | skill パスの明示指定 |

---

## 3. 変換テーブル

`mappings/skills.yaml` の人間可読版。

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `skills.path` | `.claude/skills/<name>/SKILL.md` | `.agents/skills/<name>/SKILL.md` | both | lossless | — | `path:remap`（`.claude/` ⇄ `.agents/`）のみ |
| `skills.name` | `name`（frontmatter） | `name`（frontmatter） | both | lossless | — | Codex 側の命名制約は緩い。Claude 向けは小文字化・予約語回避が必要 |
| `skills.description` | `description` | `description` | both | lossless | — | 両者とも自動発火トリガー |
| `skills.when_to_use` | `when_to_use` | `description`（連結） | claude_to_codex | lossy | — | `when_to_use` の内容を `description` 末尾にマージ。独立フィールドが失われる |
| `skills.disable-model-invocation` | `disable-model-invocation` | `agents/openai.yaml` の `policy.allow_implicit_invocation` | both | lossy | — | `polarity:invert`。Claude→Codex 変換時は明示呼び出し挙動の差を注記（Codex の方が clean） |
| `skills.user-invocable` | `user-invocable` | （対応なし） | claude_to_codex | dropped | — | Codex に「モデル専用・ユーザー非表示」概念が存在しない（ソース確定）。warn:true |
| `skills.allowed-tools` | `allowed-tools` | `.codex/rules/<skill>.rules`（execpolicy allow）、MCP は `mcp_servers.enabled_tools` | claude_to_codex | lossy | skill → session/project | skill スコープ喪失。スコープ降格で全セッションに効く。組み込みツール（AskUserQuestion 等）は dropped。warn:true |
| `skills.disallowed-tools` | `disallowed-tools` | `.codex/rules/<skill>.rules`（execpolicy forbidden）、MCP は `mcp_servers.disabled_tools` | claude_to_codex | lossy | skill → session/project | 組み込みツール（AskUserQuestion 等）禁止は代替なし→dropped。warn:true |
| `skills.model` | `model` | `.codex/agents/<skill>.toml` の `model`（subagent） | claude_to_codex | lossy | skill → subagent | 自動降格。subagent は明示 `spawn_agent` が必要で自動 fork しない。warn:true |
| `skills.effort` | `effort` | `.codex/agents/<skill>.toml` の `model_reasoning_effort` | claude_to_codex | lossy | skill → subagent | `enum_map:{max:xhigh}`。Codex に `max` 値が存在しないため `xhigh` に丸め。warn:true |
| `skills.context-fork` | `context: fork` + `agent` | standalone agent TOML + モデルによる `spawn_agent` 呼び出し | claude_to_codex | lossy | skill → subagent | 自動 fork 不可。`features.multi_agent=true` 必須、`max_depth` 既定 1。warn:true |
| `skills.hooks` | `hooks`（skill スコープ） | `[[hooks.<Event>]]`（session/project スコープ） | claude_to_codex | lossy | skill → session/project | `features.hooks=true` 必須。skill スコープに閉じない。warn:true |
| `skills.paths` | `paths`（glob 自動発火） | （等価なし） | claude_to_codex | dropped | — | ファイル操作イベント駆動の自動発火は Codex に存在しない。AGENTS.md の階層配置が最も近い近似だが等価ではない。warn:true |
| `skills.argument-hint` | `argument-hint` | （Skill には無し） | claude_to_codex | dropped | — | Custom Prompts（deprecated）にのみ存在。skill 間変換では損失。warn:true |
| `skills.arguments` | `arguments` | （Skill には無し） | claude_to_codex | dropped | — | skill 本体に引数機構なし。本文の `$name` 置換も Codex 側で解決されない。warn:true |
| `skills.shell` | `shell` | hooks の `commandWindows`（Windows 用上書き） | claude_to_codex | lossy | — | shell 選択そのものではなく Windows 限定の hook コマンド上書き |
| `skills.invocation-syntax` | `/skill-name`（本文内記法） | `$skill-name`（本文内記法） | both | lossless | — | 本文・README 中の呼び出し記法を書き換え。誤検出リスクあり、検出して提案方式が安全 |
| `skills.openai-yaml.allow_implicit_invocation` | `disable-model-invocation` | `policy.allow_implicit_invocation` | codex_to_claude | lossless | — | `polarity:invert` |
| `skills.openai-yaml.interface` | （Skill レベルのアイコン・ブランド設定なし） | `interface.display_name` / `icon_small` / `icon_large` / `brand_color` | codex_to_claude | lossy | — | Claude の skill に受け皿が弱い（plugin の `displayName` に近い）。warn:true |
| `skills.openai-yaml.default_prompt` | （本文先頭で代替） | `interface.default_prompt` | codex_to_claude | lossy | — | Claude は本文先頭への追記で近似 |
| `skills.openai-yaml.dependencies-tools` | （Skill レベルの MCP 依存宣言は弱い） | `dependencies.tools` | codex_to_claude | lossy | — | plugin の `mcpServers` に近い。warn:true |
| `skills.body-dynamic-injection` | `` !`cmd` `` / `${CLAUDE_*}` 変数 | （等価機構は未確認） | claude_to_codex | dropped | — | 変換時に検出して警告すべき。リテラル文字列として残ると誤動作の恐れ |

---

## 4. 変換時の注意・既知の落とし穴

### 4.1 パス変換は単純だが補助ファイルに注意

SKILL.md のパス変換自体は `.claude/` ⇄ `.agents/` の付け替えだけで済む。ただし Codex 固有の補助ファイル `agents/openai.yaml` は Claude Code 側に直接の受け皿がなく、変換時に frontmatter への展開・分解が必要。

### 4.2 `disable-model-invocation` と `policy.allow_implicit_invocation` の挙動差

`polarity:invert` で値変換はできる。しかし挙動に重要な差がある。Claude Code では `disable-model-invocation: true` にすると description がモデルコンテキストから消え、ユーザーが `/skill-name` と打っても**モデルが skill の存在を認識できずルーティングに失敗しうる**（既知問題: `openai/codex-plugin-cc#211`）。Codex の `policy.allow_implicit_invocation: false` は暗黙発火禁止と明示呼び出しを分離実装しており、明示呼び出し（`$name`）は正常動作する。Claude→Codex 変換時はこの差をレポートに注記すること。

### 4.3 `user-invocable: false` は確定損失

Codex の `agents/openai.yaml` の `policy` セクションで意味を持つフィールドは `allow_implicit_invocation` と `products` の 2 つのみ。`user-invocable` 等を `agents/openai.yaml` に書いても、実行時 skill loader（`codex-rs/core-skills/src/loader.rs`）は未知フィールドを fail open で黙って無視するため設定は効かない。なお `validate_plugin.py` の `reject_skill_agent_unknown_fields` は plugin 作成サンプルの lint ツールであって実行時 loader とは別物（実行時は寛容）。「ユーザーからは隠す・呼べないが、モデルは自動発火できる」という片側制御の概念が Codex skill 設計に存在しない。変換時は必ず warn を出し、破棄した旨をレポートに明記すること。

### 4.4 `allowed-tools` / `disallowed-tools` のスコープ降格

skill スコープでの per-tool 承認制御は Codex に存在しない（`SkillConfig` は `enabled`/`name`/`path` の 3 フィールドのみ、`additionalProperties: false` で確定）。降格代替経路:

- **コマンド系（Bash）**: `prefix_rule(pattern=[...], decision="allow"/"forbidden")` を `.codex/rules/<skill>.rules`（project、`trust_level="trusted"` 要）または `~/.codex/rules/default.rules`（user）へ生成。スコープは全セッションに拡大する点を必ず警告。
- **MCP ツール**: `[mcp_servers.X] enabled_tools`（allow）/ `disabled_tools`（disallow）へ。
- **組み込みツール（`AskUserQuestion` 等）の禁止**: 代替機構なし。dropped + warn。

また、`[[skills.config]]` オーバーライド自体が両方向とも効かない既知バグ（`openai/codex#14161`、2026-03 時点オープン）があり、skill 単位制御は実装上も不安定な状況。

### 4.5 `model` / `effort` / `context:fork` の subagent 降格

これらを持つ skill を変換する際は `.codex/agents/<skill>.toml`（standalone subagent TOML）を生成し、`config.toml` の `[agents.<skill>]` に `config_file` で参照させる設計が現実的。`effort: max` は Codex の最大値 `xhigh` に丸める（Codex に `max` 値がない）。ただし **Codex の subagent は自動 fork しない**（「Codex only spawns a new agent when you explicitly ask it to do so.」）。`context:fork` の自動分岐挙動は再現不可で、モデルが `spawn_agent` を明示的に呼ぶことに依存する。`features.multi_agent=true` の明示設定も必要。

### 4.6 `paths`（glob 自動発火）は等価機構なし

ファイル操作イベント駆動の自動発火は Codex に存在しない。最も近い近似は `AGENTS.md` をディレクトリ階層に配置することだが、これは「そのディレクトリでの作業への guidance」であってファイル操作イベント駆動の発火ではない。変換では dropped + warn が唯一の誠実な処理。

### 4.7 `arguments` / `argument-hint` は Skill 間では損失

Codex の Skill（`SKILL.md`）には引数機構がない。Custom Prompts（deprecated）にのみ `$1`〜`$9`/`argument-hint` が存在するが、skills 移行を前提とすると過渡的な代替に過ぎない。変換では dropped + warn。本文内の `$name` 引数置換も Codex 側で展開されないため、検出して警告すること。

### 4.8 本文内の動的注入・変数は要検出

Claude の `` !`cmd` `` 動的注入と `${CLAUDE_SESSION_ID}` 等の変数は、Codex 側での同等機構が未確認。変換後にリテラル文字列として残ると誤動作の恐れがあるため、本文スキャナで検出し警告・要手動確認の旨をレポートに含めること。

### 4.9 呼び出し記法の書き換えは慎重に

本文・README 中の `/skill-name` ⇄ `$skill-name` の置換は自動変換の誤検出リスクがある（`$` は shell 変数とも衝突、`/` はパス区切りとも衝突）。検出して置換提案を出す方式が安全。

### 4.10 `agents/openai.yaml` の `interface.*` は Claude への受け皿が弱い

Codex→Claude 変換では `interface.display_name` / `icon_small` / `icon_large` / `brand_color` の受け皿が Claude の skill レベルに存在しない。plugin の `displayName` に近い概念はあるが、skill 個別の UI メタデータとしては未対応。warn を出し、手動確認を促すこと。

---

## 5. 出典

- Claude Code Skills: https://code.claude.com/docs/en/skills
- Claude Code Agent Skills overview: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview
- Claude Code Plugins: https://code.claude.com/docs/en/plugins
- Claude Code Hooks: https://code.claude.com/docs/en/hooks
- OpenAI Codex Skills: https://developers.openai.com/codex/skills
- OpenAI Codex Config Reference: https://developers.openai.com/codex/config-reference
- OpenAI Codex Hooks: https://developers.openai.com/codex/hooks
- GitHub: openai/codex（config.schema.json, codex-rs/core-skills/, docs/skills.md, references/openai_yaml.md, validate_plugin.py）: https://github.com/openai/codex
- AGENTS.md オープン標準: https://agents.md/
