# 10. CLI Design — 相互変換 CLI の設計

`mappings/*.yaml` を消費して Claude Code ⇄ Codex の設定を相互変換する CLI の設計指針。実装言語は問わないが、ここでは概念とインターフェースを定義する。

## 1. パイプライン

```
入力ファイル群
   │  (1) detect & parse        — ファイル種別判定 + 形式パース(JSON/TOML/MD+frontmatter)
   ▼
   ソース別 AST
   │  (2) lift to IR            — 中間表現へ。フィールドを正規化キーに写像
   ▼
   IR (中間表現)
   │  (3) apply mappings        — mappings/*.yaml の direction/loss/degrade/transform を適用
   ▼
   ターゲット IR (+ 損失メタ)
   │  (4) lower & serialize     — 出力形式へ(TOML/JSON/MD)。本文は scanner で書き換え
   ▼
   出力ファイル群 + (5) conversion report
```

- **(1) detect & parse**: 拡張子・パス・先頭バイトで Claude/Codex どちらの何かを判定（`.claude/skills/*/SKILL.md`, `.mcp.json`, `config.toml`, `CLAUDE.md` …）。JSON/TOML/YAML-frontmatter をパース。
- **(2) lift to IR**: 各領域の `mappings/*.yaml` の `id` を正規化キーとして、ソースのフィールドを IR に載せる。
- **(3) apply mappings**: 後述の変換エンジン。`transform` を関数適用、`degrade` を解決、`direction`/`loss` で取捨。
- **(4) lower & serialize**: ターゲット形式に直列化。skill/command/prompt の**本文**は §4 のスキャナで変数・呼び出し記法を書き換え。
- **(5) report**: `dropped` と `degrade` を必ず列挙（§5）。

## 2. 中間表現（IR）

両ツールの**和集合**をフィールドに持つ正規化スキーマ。各フィールドにメタを付す。

```jsonc
{
  "kind": "skill",                       // skill|plugin|subagent|hooks|mcp|settings|memory
  "source_tool": "claude",               // claude|codex
  "fields": {
    "skills.name":  { "value": "deploy", "origin": "claude", "lossiness": "lossless" },
    "skills.allowed-tools": {
      "value": ["Bash(git add *)"], "origin": "claude",
      "lossiness": "lossy",
      "degrade": { "to": "session", "target": ".codex/rules/deploy.rules" }
    }
  },
  "body": "...skill 本文(Markdown)...",
  "warnings": [ /* 変換中に蓄積 */ ]
}
```

IR を経由する利点: (a) 双方向を1つのテーブルで扱える、(b) `lossiness`/`degrade` を機械的に集計して report 化できる、(c) 往復テスト（§7）で IR 差分を比較できる。

## 3. 変換エンジン：transform と degrade

### 3.1 transform（値・書式変換）
`mappings` の `transform` 文字列をパースして関数適用。`;` 区切りで複数。実装すべき基本関数:

| transform | 実装 |
|---|---|
| `unit:ms_to_sec` / `unit:sec_to_ms` | 数値 ÷1000 / ×1000（小数許容） |
| `polarity:invert` | bool 反転（`disabled:true` ⇔ `enabled:false`） |
| `enum_map:{a:b,...}` | enum 値の対応（`effort: max`→`xhigh`） |
| `index_shift:+1` / `-1` | 引数インデックスの 0⇔1 基点（`$ARGUMENTS[0]`→`$1`） |
| `str_to_list:space` / `list_to_str:space` | スペース区切り文字列 ⇔ 配列（OAuth scopes） |
| `rename` | キー名変更（`headers`⇔`http_headers`） |
| `format:json_to_toml` / `toml_to_json` | シリアライズ形式変換 |
| `extract:bearer_env` | `"Bearer ${VAR}"` から `VAR` 抽出（MCP Bearer） |
| `path:remap` | パス規約変換（`.claude/`⇔`.agents/`, `.claude-plugin/`⇔`.codex-plugin/`） |
| `inline_imports` | `@import` 参照を1ファイルに展開（CLAUDE.md→AGENTS.md） |

### 3.2 降格マッピングエンジン（Claude → Codex の核心）

skill スコープを失う代わりに機能を保つ「降格」を、専用モジュールで実装する。

- **ツール pre-approve**: `allowed-tools: Bash(<cmd> <args...>)` → `prefix_rule(pattern=[<cmd>, <args...>], decision="allow")` を `.codex/rules/<skill>.rules`（project）または `~/.codex/rules/default.rules`（user）へ生成。`disallowed-tools: Bash(...)` → `decision="forbidden"`。MCP ツールは `[mcp_servers.X] enabled_tools`/`disabled_tools`。**組み込みツール（`AskUserQuestion` 等）の禁止は変換先なし → 警告して破棄**。
- **skill → subagent**: `model`/`effort`/`context:fork` を持つ skill は `.codex/agents/<skill>.toml`（`model` / `model_reasoning_effort`(`max`→`xhigh`) / `sandbox_mode` / `approval_policy` / `developer_instructions`=skill 本文）を生成し、`config.toml` の `[agents.<skill>]` に `config_file` で参照。`description`=`when_to_use`。`[features] multi_agent=true` を明記。
- **hooks**: skill スコープ hooks → `[[hooks.<Event>]]`（session/project）+「skill スコープではなくなる」警告。`command` タイプのみ移植可、`http`/`mcp_tool`/`prompt`/`agent` は破棄。
- **必須の警告出力**: ① スコープ降格（skill→session/subagent） ② 自動 fork → 明示 `spawn_agent` への挙動変化 ③ 組み込みツール禁止・`paths` 自動発火・引数機構の喪失 ④ project 層の `.rules`/`.codex/agents` は `projects.<path>.trust_level="trusted"` が前提。

## 4. 本文スキャナ（skill/command/prompt の Markdown 本文）

本文には変数・引数・呼び出し記法・動的注入が埋まっており、形式変換だけでは壊れる。スキャナで検出して書き換え/警告する（パターンは [09](09-variables-and-templating.md) の正規表現一覧）。

- `$ARGUMENTS[N]`(0基点) / `$N` → Codex `$(N+1)`（`index_shift:+1`、**最重要の落とし穴**）。
- `$name`（named）→ Codex `$UPPERCASE`（呼び出しが `KEY=value` に変わる、警告）。
- `${CLAUDE_SESSION_ID}` / `${CLAUDE_EFFORT}` / `${CLAUDE_SKILL_DIR}` / `${CLAUDE_PROJECT_DIR}` / `${CLAUDE_PLUGIN_ROOT}` → Codex に同等なし、**破棄＋警告**。
- 動的注入 `` !`cmd` `` / ` ```! ` → Codex 非対応。**そのまま残すと literal 文字列化して誤動作するため、検出して警告**（自動除去は危険なので提案にとどめる）。
- 呼び出し記法 `/skill-name` ⇔ `$skill-name`、`/plugin:skill`（名前空間）は Codex 非対応。

> 自動置換は誤検出リスクがあるため、`--rewrite-body` を明示したときのみ実行し、既定は検出＋提案にとどめるのが安全。

## 5. conversion report

変換実行ごとに必ず出力する。`mappings` の不変条件（SCHEMA.md §不変条件）に従う:

```
$ claude2codex ./my-plugin --report
✔ skills/deploy/SKILL.md → .agents/skills/deploy/SKILL.md
  ◎ name, description, body           (lossless)
  △ allowed-tools → .codex/rules/deploy.rules   (lossy, degrade: skill→session)
  △ model: opus → gpt-5.x             (lossy, 要モデル名マップ)
  ✕ user-invocable                    (dropped: Codex に概念なし)
  ✕ paths                             (dropped: glob 自動発火は非対応)
  ⚠ 本文 L42: !`git diff` は Codex で実行されません（literal 化）

Summary: 3 lossless, 2 lossy(+2 degraded to session/subagent), 2 dropped, 1 body-warning
```

- `--report`（詳細）/ `--report=json`（機械可読、CI 用）/ `--dry-run`（書き込まず report のみ）。
- `dropped` と `degrade` は**必ず**列挙。silent な切り捨て厳禁。

## 6. CLI インターフェース（案）

```
claude2codex <path> [--out DIR] [--scope user|project] [--report[=json]] [--dry-run] [--rewrite-body]
codex2claude <path> [--out DIR] [--report[=json]] [--dry-run] [--rewrite-body]
ccx check <path>          # 変換可能性の事前診断（dropped 件数の見積り）
ccx model-map [--edit]    # モデル名対応表(claude-* ⇔ gpt-*)の管理（settings/subagents で必要）
```

- 領域単位の変換も可能に: `claude2codex skill ./skills/deploy`。
- `--scope` は降格先（user 層 `~/.codex/...` か project 層 `.codex/...`）を選ぶ。

## 7. 品質保証

- **往復テスト（round-trip）**: `claude → codex → claude` で IR 差分が「既知の損失項目（`dropped`/`lossy`）」だけになることをゴールデンテストで検証。`lossless` エントリは完全一致を要求。
- **mappings 検証**: 全 `id` の一意性、`loss:dropped` に `transform` が無いこと、`degrade` のあるエントリが `loss:lossy` であること、`source` URL の存在を CI でチェック。
- **スキーマ追従**: SchemaStore（Claude）と `config.schema.json`（Codex）の更新を監視し、新フィールドの差分を検出。

## 8. 領域の分類とロードマップ

変換の「実装労力」と「CLI としての価値」は領域で大きく異なる。損失分布（lossless/lossy/dropped）だけでは測れず、**新規ロジックの要否**と**他領域の統合点か**で分類する。

### 領域マップ（コア / 随伴 / 最難関）

| 領域 | total | lossless | 性質 | 区分 |
|---|---|---|---|---|
| **skills** | 22 | 5 | 降格エンジン（`.rules`/subagent 生成）+ 本文スキャナ。**新規ロジックの本体** | **コア** |
| **hooks** | 83 | 34 | JSON⇄TOML の構造変換（array-of-tables）+ 入出力 JSON のネスト変換 + 30↔10 イベント対応 | **コア** |
| **plugins** | 48 | 13 | skills/hooks/mcp/agents を内包する**統合点**（内部で各変換器を再帰呼び出し）+ marketplace | **コア（統合点）** |
| mcp | 30 | 10 | フィールド 1:1 マップ + 機械 transform（単位/極性/rename/Bearer 抽出）。**新規ロジック不要** | 軽量随伴（plugins の部品） |
| memory | 16 | 3 | ファイルリネーム + `@import` インライン展開 | 軽量随伴 |
| variables | 14 | 0 | 本文スキャナの一部（skills 変換に内包して実装） | 随伴（skills 内包） |
| subagents | 25 | 4 | 設計差大（自動 fork vs `spawn_agent`、tools 軸違い） | 難関・部分対応 |
| settings-config | 49 | 2 | 権限の軸違い、lossless 率 4%。全自動は非現実的 | 最難関・部分集合のみ |

**要点: CLI の価値と難所は skills / hooks / plugins に集中する。**
- **skills** = 降格エンジンと本文スキャナという新規ロジックの本体。
- **hooks** = 単純な key-value でない構造変換（array-of-tables、`hookSpecificOutput` のネスト、イベント対応）。
- **plugins** = 他コンポーネントを内包する**統合点**で、skills/hooks/mcp の変換器を内部で呼ぶ CLI の集大成。
- 対して **MCP / memory は機械的 transform でほぼ済み新規ロジックを要さない**（MCP の dropped は Codex 固有のツール粒度制御を捨てるだけ）。MCP は単体価値より「**plugins の部品**」として必要。variables は skills の本文スキャナに内包される。

### ロードマップ（依存順）

| フェーズ | 範囲 | 狙い |
|---|---|---|
| **v1** | **Skills**（降格エンジン + 本文スキャナ＝variables 内包）＋ MCP（部品として軽量同梱） | 単一 skill の往復変換が動く。CLI の中核ロジック（降格・本文書き換え）を確立 |
| **v2** | **Hooks**（JSON⇄TOML 構造変換）＋ Memory（CLAUDE.md⇄AGENTS.md, `@import` 展開） | 構造変換器を確立 |
| **v3** | **Plugins** ＋ marketplace | v1/v2 の変換器を内部で呼ぶ統合点。skills/hooks/mcp を束ねた plugin を丸ごと変換 |
| **v4** | Subagents（skill→subagent 降格と統合）＋ Settings 部分集合（権限/env/model）＋ report の CI 統合 | 最難関。全自動は狙わない |

> MCP を v1 に同梱するのは「ROI が高い」からではなく、**軽量で v3(plugins) の前提部品になる**から。CLI の価値の中心は **v1 の skills と v3 の plugins**、そして v2 の hooks。

## 9. バージョン依存性

Codex の Skills/Plugins/Hooks は流動的。CLI は `codex --version` / `claude --version` を読み、`mappings` の各エントリに（将来）付与する `min_version`/`max_version` で未対応機能をスキップ＋警告する。現状の既知の不安定要素は各 `mappings/*.yaml` の `notes` を参照（例: `openai/codex#14161` の `[[skills.config]]` バグ、`features.codex_hooks` の扱い、Windows hooks 未サポート）。
