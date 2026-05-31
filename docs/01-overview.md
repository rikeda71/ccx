# 01. Overview — 概念対応マップと設計思想

Claude Code と OpenAI Codex CLI は、エージェント型コーディングツールとして**同じ構成要素**（再利用可能な指示、配布バンドル、サブエージェント、ライフサイクルフック、外部ツール接続、指示メモリ）を持つ。Codex は Anthropic の設計を追随した形跡が濃く、相互変換は構造的に成立する。だが細部の表現力・スコープ・ファイル形式が異なり、特に **Claude → Codex 方向で情報損失**が生じる。本ドキュメントは、その全体像と、変換 CLI を貫く設計思想を述べる。

## 1. 6 大概念の対応マップ

| レイヤ | Claude Code | Codex | ファイル形式 | 詳細 |
|---|---|---|---|---|
| 指示パッケージ | `.claude/skills/<n>/SKILL.md` | `.agents/skills/<n>/SKILL.md` | MD + YAML frontmatter | [02](02-skills.md) |
| 配布バンドル | `.claude-plugin/plugin.json` | `.codex-plugin/plugin.json` | JSON | [03](03-plugins.md) |
| 配布カタログ | `.claude-plugin/marketplace.json` | `.agents/plugins/marketplace.json`（`.claude-plugin/` 互換） | JSON | [03](03-plugins.md) |
| サブエージェント | `.claude/agents/<n>.md` | `[agents.<n>]`(config.toml) + `~/.codex/agents/<n>.toml` | MD / TOML | [04](04-subagents.md) |
| フック | `settings.json`/`hooks.json` の `hooks` | `config.toml` の `[hooks.*]` | JSON / TOML | [05](05-hooks.md) |
| MCP | `.mcp.json` | `[mcp_servers.*]`(config.toml) | JSON / TOML | [06](06-mcp.md) |
| 中核設定 | `settings.json` | `config.toml` | JSON / TOML | [07](07-settings-and-config.md) |
| 指示メモリ | `CLAUDE.md` | `AGENTS.md` | Markdown | [08](08-memory-files.md) |

加えて、本文中の**変数・引数・呼び出し記法・動的注入**（[09](09-variables-and-templating.md)）が、skill/command/prompt を変換するときの書き換え対象になる。

## 2. 根本差①：「skill スコープ」概念の不在

Claude Code の最大の特徴は、多くの制御が **「その skill（または agent）が実行されている間だけ」効く動的スコープ**を持つこと。例:
- `allowed-tools` / `disallowed-tools` — その skill 実行中だけのツール事前承認/禁止
- `model` / `effort` — その skill 実行中だけのモデル/推論強度
- skill スコープ `hooks` — その skill のライフサイクルにだけ効くフック

Codex には**この「コンポーネント実行中だけ」という動的スコープが存在しない**。Codex skill の `SKILL.md` frontmatter は `name`/`description` の 2 つのみ（補助 `agents/openai.yaml` の `policy.allow_implicit_invocation`/`products` を除く）。これは「Codex に該当フィールドが無い」という単純な欠落ではなく、**Codex skill が最小設計（name + description + 本文）を採っている**ことに起因する構造的な差。

→ 変換 CLI は、これらを **より広い/別のスコープへ「降格」**することで機能的に近似する（§5）。

## 3. 根本差②：権限モデルの軸違い（ツール軸 vs リソース軸）

- **Claude Code = ツール軸**: `permissions.allow/ask/deny` に `Bash(npm run *)` / `Read(~/.env)` / `WebFetch(domain:x)` のように**ツール＋引数パターン**を書く。評価は deny → ask → allow。
- **Codex = リソース軸 + フェーズ分離**: `approval_policy`（いつ確認するか）+ `sandbox_mode`（技術的境界）+ `[permissions.<name>]`（filesystem パス→read/write/deny, network ドメイン→allow/deny）+ `.rules`（execpolicy が `allow`/`prompt`/`forbidden`）。

この軸の違いにより、権限の相互変換は多くが `lossy`。詳細とマトリクスは [07](07-settings-and-config.md)。

## 4. 根本差③：ファイル形式と配置

| 観点 | Claude Code | Codex |
|---|---|---|
| 中核設定形式 | **JSON**（settings.json, plugin.json, .mcp.json） | **TOML**（config.toml）+ JSON（plugin.json, marketplace.json, .app.json） |
| skill 配置 | `.claude/` | `.agents/`（AGENTS.md オープン標準に沿う） |
| plugin manifest dir | `.claude-plugin/` | `.codex-plugin/`（ただし `.claude-plugin/marketplace.json` を互換読み込み） |
| 真偽の極性 | MCP は `disabled: true` | MCP は `enabled: false` |
| タイムアウト単位 | ms | 秒 |
| 引数インデックス | 0 基点（`$ARGUMENTS[0]`） | 1 基点（`$1`） |

これらは機械的な変換規則（`format:json_to_toml`, `path:remap`, `polarity:invert`, `unit:ms_to_sec`, `index_shift:+1`）で吸収する。

## 5. 設計思想：スコープ降格（degrade）

skill スコープが Codex に無い以上、変換は「**より広い/別のスコープへ降格**」する。降格には必ずコスト（動的・自動の限定が失われる）が伴うため、**conversion report に明記**する。

| Claude のスコープ | 降格先（Codex） | 例 |
|---|---|---|
| skill（ツール権限） | **session/project**（`.rules` の execpolicy） | `allowed-tools` → `prefix_rule(decision="allow")` |
| skill（model/effort/fork） | **subagent**（`.codex/agents/<n>.toml`） | `model`/`effort`/`context:fork` を subagent に束ねる |
| skill（hooks） | **session/project**（`[hooks.*]`） | skill スコープ hooks → グローバル hooks |

降格の本質的コスト: session 降格は「セッション全体に効く」、subagent 降格は「自動 fork せず `spawn_agent` の明示起動が要る」。

## 6. 損失レベルと方向の定義

全 `mappings/*.yaml` で共通の語彙（[`../mappings/SCHEMA.md`](../mappings/SCHEMA.md)）:

- **loss**: `lossless`（完全等価、値/書式変換のみ）/ `lossy`（意味は近いが情報・スコープが変わる/値が丸まる）/ `dropped`（対応なし、破棄）。
- **direction**: `both`（双方向）/ `claude_to_codex`（Claude→Codex のみ）/ `codex_to_claude`（Codex→Codex のみ）。
- **degrade**: skill→別スコープへ移る場合に降格先（`to`）と具体的書き込み先（`target`）を記載。
- **transform**: 値/書式変換規則（`unit:ms_to_sec` 等）。

## 7. 変換の難易度

- **容易（Codex → Claude）**: Codex の語彙が少なく、Claude 側に受け皿が多いため、ほぼ無損失。Codex 固有（`nickname_candidates`, `config_file`, `[permissions.*]` プロファイル, granular approval, `.app.json` 等）だけが落ちる。
- **困難（Claude → Codex）**: §2 の skill スコープ制御と、表現力の高い frontmatter・plugin コンポーネント（`lspServers`/`outputStyles`/`themes`/`monitors`/`userConfig`/`channels`）が落ちるか降格する。

### 領域の重要度（方向の難易度とは別軸）

「どちら向きが難しいか」とは別に、**変換ロジックの新規性と統合点か**で領域を見ると、CLI の価値と実装の難所は **Skills / Hooks / Plugins に集中する**:

- **Skills** — 降格エンジン（`.rules`/subagent 生成）と本文スキャナという**新規ロジックの本体**。
- **Hooks** — 単純な key-value でない構造変換（JSON⇄TOML の array-of-tables、`hookSpecificOutput` のネスト、30↔10 イベント対応）。
- **Plugins** — skills/hooks/mcp/agents を**内包する統合点**で、各変換器を内部で呼ぶ集大成。

一方 **MCP・Memory は機械的 transform でほぼ済む軽量領域**（MCP の dropped は Codex 固有のツール粒度制御を捨てるだけ）。特に MCP は単体の価値より **plugins の部品**として要る。Variables は skills の本文スキャナに内包。Settings⇄Config は権限の軸違いで最難関（lossless 率 4%）だが全自動は狙わない。優先度の詳細は [10 CLI Design](10-cli-design.md) §8。

## 8. 次に読む

- 領域別の全フィールド表と変換テーブル: [02 Skills](02-skills.md) / [03 Plugins](03-plugins.md) / [04 Subagents](04-subagents.md) / [05 Hooks](05-hooks.md) / [06 MCP](06-mcp.md) / [07 Settings⇄Config](07-settings-and-config.md) / [08 Memory](08-memory-files.md) / [09 Variables](09-variables-and-templating.md)
- 変換 CLI の設計（IR・降格エンジン・損失レポート・ロードマップ）: [10 CLI Design](10-cli-design.md)
- 初期の深掘り検証（特に Skills の `user-invocable`/`disable-model-invocation`/`allowed-tools` を Codex ソースレベルで検証）: [reference/00-initial-integrated-report.md](../reference/00-initial-integrated-report.md)
