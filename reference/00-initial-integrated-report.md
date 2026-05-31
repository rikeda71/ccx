# Claude Code と Codex の SKILL / PLUGIN 設定値 比較レポート

> 目的: 将来的に「Claude Code ⇄ OpenAI Codex CLI」の設定（Skills / Plugins / 周辺機構）を相互変換する CLI を提供するための、設定値レベルの精密な対応関係・差分・情報損失ポイントの整理。
>
> 作成日: 2026-05-30 / 対象: Claude Code（`code.claude.com/docs`）, OpenAI Codex CLI（`developers.openai.com/codex`, `github.com/openai/codex`）
>
> 注意: Codex の Skills / Plugins / Hooks は 2025 年後半〜2026 年初頭に追加された比較的新しい機能で、仕様がまだ流動的です（後述 §7）。本レポートは公式ドキュメント・スキーマ・実リポジトリを根拠にしていますが、CLI 実装時は必ずバージョンごとに再検証してください。

---

## 0. エグゼクティブサマリー

| 観点 | 結論 |
|---|---|
| **概念の対応** | Skills・Plugins・Hooks・MCP・メモリファイル（CLAUDE.md / AGENTS.md）・marketplace の **6 大概念がほぼ 1:1 で両者に存在**する。Codex が Anthropic の設計を追随した形跡が濃く、相互変換は構造的には成立する。 |
| **最大の障壁** | **SKILL.md frontmatter の表現力差と「skill スコープ」概念の不在**。Claude は 16 フィールド、Codex skill は 2（`name`/`description`）のみ。不足分は Codex の別機構（`agents/openai.yaml`・`.rules`・subagent `config_file`・hooks）に分散しており、多くは **skill スコープを session/subagent へ「降格」すれば代替可能**（§2.6）。ただし `user-invocable`・`paths` 自動発火・組み込みツール禁止・引数機構は **確定損失**。 |
| **変換が容易な方向** | **Codex → Claude Code**。Codex の少ない frontmatter は Claude Code 側に全て受け皿があるため、ほぼ無損失。 |
| **変換が難しい方向** | **Claude Code → Codex**。`allowed-tools`/`model`/`effort`/`context:fork`/`hooks` は skill には書けないが、**`.rules`・subagent `config_file`・session hooks へ降格**して代替できる（§2.6）。`user-invocable`・`paths` 自動発火・引数機構・組み込みツール禁止は**破棄**。 |
| **設定ファイル形式** | Claude Code = **JSON**（`settings.json`, `plugin.json`, `.mcp.json`）、Codex = **TOML**（`config.toml`）+ JSON（`plugin.json`, `marketplace.json`）。フォーマット変換が常に挟まる。 |
| **追い風** | Codex の `marketplace.json` は **`.claude-plugin/marketplace.json` を互換パスとして読む**。MCP の STDIO キー（`command`/`args`/`env`）は両者共通。メモリファイルの階層マージ思想も同じ。 |

---

## 1. 全体マッピング（概念対応表）

| 概念 | Claude Code | OpenAI Codex CLI | 対応度 |
|---|---|---|---|
| 再利用可能な指示パッケージ | **Skills** (`SKILL.md`) | **Skills** (`SKILL.md`) | ◎ ファイル名まで同一 |
| ローカル Skill 配置 | `.claude/skills/<name>/` | `.agents/skills/<name>/` | ○ パスのみ差 |
| グローバル Skill 配置 | `~/.claude/skills/<name>/` | `~/.agents/skills/<name>/` | ○ パスのみ差 |
| 配布可能拡張バンドル | **Plugins** (`.claude-plugin/plugin.json`) | **Plugins** (`.codex-plugin/plugin.json`) | ◎ 構造ほぼ同一 |
| 配布カタログ | `marketplace.json` | `marketplace.json`（`.claude-plugin/` 互換読み込みあり） | ◎ |
| プロジェクト指示メモリ | `CLAUDE.md` | `AGENTS.md`（オープン標準） | ○ 思想同一・名前差 |
| 明示オーバーライド | （CLAUDE.md 階層優先） | `AGENTS.override.md` | △ Codex に専用ファイル |
| スラッシュコマンド（旧式） | `.claude/commands/*.md` | `~/.codex/prompts/*.md`（**deprecated**） | △ |
| ライフサイクル hooks | **Hooks**（デフォルト有効, 30+ イベント） | **Hooks**（`features.hooks=true` で opt-in, 10 イベント） | ○ Claude が広範 |
| MCP サーバー | `.mcp.json`（JSON） | `[mcp_servers.*]`（TOML） | ○ STDIO キー共通 |
| サブエージェント | **Agents** (`agents/*.md`) | `[agents.*]`（config.toml）+ `agents/openai.yaml` | △ 設計が異なる |
| 中核設定 | `settings.json`（JSON） | `config.toml`（TOML） | △ 形式・粒度差大 |
| 権限/サンドボックス | `permissions`（settings.json） | `approval_policy` + `sandbox_mode` + `[permissions.*]` | △ Codex が細粒度 |
| エンタープライズ強制 | managed settings | `requirements.toml` | ○ |

凡例: ◎ ほぼ無損失で変換可 / ○ 形式変換は要るが意味は保持 / △ 設計差が大きく部分的・要手動

---

## 2. Skills 詳細比較

### 2.1 ディレクトリ構造とスコープ

**Claude Code**

```
~/.claude/skills/<name>/SKILL.md         # personal（全プロジェクト）
.claude/skills/<name>/SKILL.md           # project
<plugin-root>/skills/<name>/SKILL.md     # plugin 内（名前空間 plugin:name）
<plugin-root>/SKILL.md                   # plugin root 単一 skill
.claude/commands/<name>.md               # legacy command（後方互換）
```
- 優先順位: Enterprise > Personal > Project
- 補助ファイル: `scripts/`, `references/`, 任意 `.md`, `assets/`（慣習。必須構造ではない）
- 起動ディレクトリからリポジトリルートまで遡って自動探索 + サブディレクトリも on-demand 探索

**Codex**

```
$CWD/.agents/skills/<name>/SKILL.md      # repo（作業ディレクトリ）
$REPO_ROOT/.agents/skills/<name>/SKILL.md# repo ルート共有
~/.agents/skills/<name>/SKILL.md         # user
/etc/codex/skills/<name>/SKILL.md        # admin
（Codex 同梱）                            # system / bundled
<plugin-root>/skills/<name>/SKILL.md     # plugin 内
```
- 優先順位（高い順）: REPO > USER > ADMIN > SYSTEM
- 補助ファイル: `scripts/`, `references/`, `assets/`, **`agents/openai.yaml`**（Codex 固有の UI/ポリシー設定）
- シンボリックリンク追跡あり

> **変換メモ**: ディレクトリ構造・補助ファイルの慣習はほぼ同じ。**変換は実質「`.claude/skills/` ⇄ `.agents/skills/` のパス付け替え」**で済む。ただし Codex 固有の `agents/openai.yaml` は Claude Code に受け皿がない（§5.1 参照）。

### 2.2 SKILL.md frontmatter フィールド対応（本レポートの核心）

両者の SKILL.md は YAML frontmatter + Markdown 本文という構造は共通。だが frontmatter の語彙が決定的に違う。

| Claude Code フィールド | 型 | Codex の対応 | 変換可否 | 備考 |
|---|---|---|---|---|
| `name` | string（≤64, lowercase 英数ハイフン, "claude"/"anthropic" 禁止） | `name`（必須） | ◎ 双方向可 | Codex 側の命名制約は緩いと見られる。Claude 向けは小文字化・予約語回避が必要 |
| `description` | string（≤1024） | `description`（必須） | ◎ 双方向可 | 両者とも自動発火トリガー。意味的に完全対応 |
| `when_to_use` | string | （`description` に内包） | ○ → Codex は description へ連結 | Codex に独立フィールドなし。description へマージ |
| `argument-hint` | string | （Custom Prompts の `argument-hint` のみ。Skills には無し） | △ | Skill 間では損失。prompt 変換なら保持可 |
| `arguments` | string/list（`$name` 名前付き位置引数） | （Skills に無し。Custom Prompts は `$1`〜`$9`/`$ARGUMENTS`） | △ | Skill→Skill では損失。本文の `$name` 置換も Codex 側で не解決 |
| `disable-model-invocation` | boolean | `agents/openai.yaml` の `policy.allow_implicit_invocation`（反転） | **△ ほぼ等価**（§2.5a） | 暗黙発火禁止＋description除外まで等価。明示呼び出し時の挙動に差（Codex の方が clean） |
| `user-invocable` | boolean | **対応フィールドなし**（`policy` は `allow_implicit_invocation`/`products` のみ・ソース確定） | **✕ 再現不可**（§2.5b） | Codex に「モデル専用・ユーザー非表示」概念が存在しない |
| `allowed-tools` | string/list（pre-approve, ワイルドカード可） | **skill スコープの対応なし**。MCP ツールのみ `approval_mode=auto` へ降格可 | **✕ skill 単位不可 / △ セッション降格**（§2.5c） | `SkillConfig` は `enabled`/`name`/`path` のみ。降格はスコープが全セッションに拡大（要警告） |
| `disallowed-tools` | string/list | **skill スコープの対応なし**。MCP のみ `disabled_tools` へ降格可 | **✕ skill 単位不可 / △ セッション降格**（§2.5c） | 組み込みツール（`AskUserQuestion` 等）への禁止は完全損失 |
| `model` | string（`/model` 値, `inherit`） | **subagent(agent TOML) / profile の `model`** | △ subagent/profile 降格（§2.6B） | skill 単位は不可だが、subagent 単位なら完全代替 |
| `effort` | enum(low/medium/high/xhigh/max) | **`model_reasoning_effort`（subagent/profile）** | △ 降格（§2.6B） | `max` は Codex 最大 `xhigh` に丸め（Codex に `max` なし） |
| `context: fork` | enum(fork) | **standalone agent TOML + `spawn_agent`** | △ 部分（§2.6B） | 自動 fork せず明示 spawn。`features.multi_agent` 要、`max_depth` 既定 1 |
| `agent` | string（fork 時の subagent type） | **subagent 名（agent TOML / `[agents.*]`）** | △（§2.6B） | subagent へマップ |
| `hooks` | object（skill スコープ hooks） | **Codex hooks（session/project スコープ）** | △ スコープ降格（§2.6C） | Codex hooks は skill スコープに閉じない。要 `features.hooks=true` |
| `paths` | string/list（glob で自動発火条件） | **等価なし**（`AGENTS.md` の dirname 階層配置が最も近い） | ✕ 等価なし（§2.6C） | ファイル操作イベント駆動の自動発火は Codex に無い |
| `shell` | enum(bash/powershell) | hooks の `commandWindows`（Windows 用上書き） | △ 部分（§2.6C） | shell 選択そのものではない |
| `${CLAUDE_SKILL_DIR}` 等の変数 | — | Codex 側に同等変数あるが名前が違う | △ | 本文中の変数置換は要マッピング |

| Codex 固有（SKILL.md / 補助ファイル） | 位置 | Claude Code の対応 | 変換可否 |
|---|---|---|---|
| `interface.display_name` | `agents/openai.yaml` | （Skill に表示名概念は弱い。plugin の `displayName` に近い） | △ |
| `interface.short_description` | `agents/openai.yaml` | `description`（部分） | △ |
| `interface.icon_small/large` | `agents/openai.yaml` | （Skill レベルのアイコン無し。plugin レベルにあり） | ✕ |
| `interface.brand_color` | `agents/openai.yaml` | （無し） | ✕ |
| `interface.default_prompt` | `agents/openai.yaml` | （`/skill` 実行時の前置プロンプト。Claude は本文先頭で代替） | △ |
| `policy.allow_implicit_invocation` | `agents/openai.yaml` | `disable-model-invocation`（反転） | ○ |
| `dependencies.tools`（MCP 依存） | `agents/openai.yaml` | （Skill 単位の MCP 依存宣言は弱い。plugin の `mcpServers`） | △ |

> **結論（Skills 変換）**:
> - **Codex → Claude Code**: `name` / `description` はそのまま、`agents/openai.yaml` の `policy`・`interface` を Claude の frontmatter（`disable-model-invocation` 等）と本文へ取り込めば **ほぼ無損失**。
> - **Claude Code → Codex**: `name`/`description`/`when_to_use`(連結)/`disable-model-invocation`(→openai.yaml) までは安全。**`allowed-tools`, `model`, `effort`, `context:fork`, `agent`, `paths`, `arguments`, `hooks` は Codex の Skill 機構に等価な置き場がなく、損失または本文へのテキスト埋め込みでの近似が必要**。

### 2.3 本文・補助ファイル

| 項目 | Claude Code | Codex |
|---|---|---|
| 本文推奨上限 | 500 行 | （明示上限未確認。コンテキスト予算で制御） |
| 補助ファイル参照深さ | 1 レベル推奨 | （同様の慣習と推測） |
| スクリプト同梱 | `scripts/`（コンテキスト非搭載・出力のみ） | `scripts/`（同様） |
| 動的注入 | `` !`cmd` `` / ` ```! ` ブロックでシェル実行結果を埋込 | （同等機構は未確認） |
| 変数置換 | `$ARGUMENTS`, `$N`, `$name`, `${CLAUDE_SESSION_ID}`, `${CLAUDE_EFFORT}`, `${CLAUDE_SKILL_DIR}` | （Custom Prompts は `$1`〜`$9`/`$ARGUMENTS`/`$UPPER`/`$$`。Skills の本文変数は要検証） |
| スキルリスト予算 | コンテキストの ~1%（設定可） | コンテキストの ~2% または ~8000 文字 |

> **変換メモ**: 本文 Markdown はほぼそのまま移植可能。ただし **Claude 固有の動的注入 `` !`cmd` `` と `${CLAUDE_*}` 変数は Codex 側で展開されない**ため、変換時に検出して警告すべき（リテラル文字列として残ると誤動作の恐れ）。

### 2.4 発火・呼び出しメカニズム

| 項目 | Claude Code | Codex |
|---|---|---|
| 自動発火 | `description`（+`when_to_use`）の意味マッチ | `description` の意味マッチ |
| 明示呼び出し | **`/skill-name`**（スラッシュ） | **`$skill-name`**（ドル）または `/skills` メニュー |
| 自動発火の抑止 | `disable-model-invocation: true` | `policy.allow_implicit_invocation: false`（openai.yaml） |
| 無効化（設定側） | `skillOverrides`（settings.json） | `[[skills.config]]` の `enabled=false`（config.toml） |
| プラグイン名前空間 | `plugin:skill` | `plugin:skill`（同様） |

> **変換メモ**: ユーザーが本文や README に「`/foo` を実行」と書いている場合、**Codex では `$foo`** に書き換える必要がある（呼び出し記号がスラッシュ ⇄ ドルで異なる）。本文テキスト内の呼び出し記法も変換対象。

### 2.5 【深掘り検証】skill 挙動制御 3 フィールドの Codex 再現可能性（ソースレベル確定）

`user-invocable` / `disable-model-invocation` / `allowed-tools`(+`disallowed-tools`) は skill 設計の核心であり、相互変換 CLI の成否を左右する。そこで Codex の**実装ソース**（`codex-rs/core-skills/` の Rust）・**JSON スキーマ**（`codex-rs/core/config.schema.json`）・**公式 skill 作成ガイド**（`openai/codex` および `openai/skills` の `references/openai_yaml.md`・`validate_plugin.py`）まで降りて再現可否を確定した。2 つの独立した検証が同一の一次ソースに到達し、結論も一致している。

#### 結論サマリー

| Claude Code フィールド | Codex での再現 | 一言結論 |
|---|---|---|
| `disable-model-invocation: true` | **△ ほぼ等価**（挙動差 1 点） | `agents/openai.yaml` の `policy.allow_implicit_invocation: false` で再現可 |
| `user-invocable: false` | **✕ 完全に存在しない** | Codex に「モデル専用・ユーザー非表示」概念がない（ソース確定） |
| `allowed-tools` | **✕ skill 単位不可 / △ 降格代替可**（§2.6A） | skill 単位フィールドはないが、user/project 層 `.rules` の `allow` でセッション単位の pre-approve は可能 |
| `disallowed-tools` | **✕ skill 単位不可 / △ 降格代替可**（§2.6A） | execpolicy `forbidden` / MCP `disabled_tools` でセッション単位の禁止は可能。組み込みツール禁止は不可 |

#### (a) `disable-model-invocation` → `policy.allow_implicit_invocation: false`【△ ほぼ等価・挙動差 1 点】

- **再現できる**。Codex の skill 補助ファイル `agents/openai.yaml` に `policy.allow_implicit_invocation: false` を書くと、モデルの暗黙（自動）発火が禁止され、ユーザーの `$skill-name` 明示呼び出しのみ可能になる。
- 根拠（実装）: `codex-rs/core-skills/src/render.rs` の `build_available_skills()` が `allowed_skills_for_implicit_invocation()` でフィルタし、`false` の skill を**モデルに見せる "## Skills" ブロックから description ごと除外**する。明示呼び出しは `codex-rs/core/src/session/turn.rs` の `collect_explicit_skill_mentions()` が**全 skill**（`skills_outcome.skills`）を対象にするため引き続き機能する。
- 根拠（公式記述）: `openai_yaml.md`「When false, the skill is not injected into the model context by default, but can still be invoked explicitly via `$skill`. Defaults to true.」
- **等価な点**: 暗黙発火の禁止＋description のモデルコンテキストからの除外。
- **非等価な点（重要）**: Claude Code では `disable-model-invocation: true` にすると description が完全に消え、ユーザーが `/skill-name` と打っても**モデルが skill の存在を認識できずルーティングに失敗**しうる（既知問題: `openai/codex-plugin-cc#211`。1 フラグが「暗黙発火禁止」と「明示ルーティング不能」の 2 関心事を混同）。Codex は両者を分離実装しており、明示呼び出しは正常動作する。**この点では Codex の設計の方が clean**。
- 変換方針: Claude→Codex は変換可。ただし「Codex 側では明示呼び出しがより確実に動く」差異を注記すべき。

#### (b) `user-invocable` → 対応フィールドなし【✕ 完全に存在しない】

- **再現できない**。`agents/openai.yaml` の `policy` セクションで公式に認識される既知フィールドは **`allow_implicit_invocation` と `products` の 2 つのみ**。これは `validate_plugin.py` の `reject_skill_agent_unknown_fields(policy, {"allow_implicit_invocation"}, ...)` というバリデーション実装でソースレベルに確定している（未知フィールドは拒否される）。
- 「ユーザーからは隠す／呼べないが、モデルは自動発火できる」という**片側制御の概念自体が Codex skill 設計に存在しない**。`config.toml` の `skills.config[].enabled = false` は skill を**完全無効化**するだけで、片側だけ塞ぐことはできない。
- 変換方針: Claude→Codex は**変換不可**。CLI はこのフィールドを破棄し、「**Codex では当該 skill をユーザーが `$name` で呼べてしまう**（モデル専用にできない）」と明示警告するしかない。

#### (c) `allowed-tools` / `disallowed-tools` → skill スコープでは再現不可【✕（セッション降格のみ △）】

- **skill スコープでは再現できない**。Codex の `SkillConfig`（config.toml の `[[skills.config]]`）は `enabled` / `name` / `path` の **3 フィールドのみ**で `additionalProperties: false`（`config.schema.json` で確定）。`SKILL.md` frontmatter も `name` / `description` のみで、公式ガイドは **"Do not include any other fields in YAML frontmatter."** と明記。**skill 単位でツールを許可/禁止/事前承認するフィールドは一切存在しない**。
- `agents/openai.yaml` の `dependencies.tools` は **MCP 依存の宣言**（不足依存の検出・自動インストール、`features.skill_mcp_dependency_install` が制御）であって、**ツール呼び出しの承認制御ではない**。
- 紛らわしい近傍: `approval_policy.granular.skill_approval`（config.toml）は「skill スクリプト実行時に承認プロンプトを出すか」の**グローバル**設定。①どの skill か区別しない ②`allowed-tools` とは逆方向（承認スキップではなく承認要求）。`allowed-tools` の代替にはならない。
- **近似（降格）の限界**: 対象が MCP ツールなら `mcp_servers.<id>.tools.<name>.approval_mode = "auto"`（事前承認）/ `disabled_tools`（禁止）で機能的に近づけられるが、**いずれも全セッション固定スコープ**になり「skill 実行中だけ」という本質が失われる（＝セキュリティ性質が変わる）。`AskUserQuestion` 等の **Codex 組み込みツールへの禁止は対応機構がなく完全損失**。
- 注意（バグ）: skill 単位の有効/無効を司る `[[skills.config]]` オーバーライド自体が「両方向とも効かない」既知バグ（`openai/codex#14161`、2026-03 時点オープン）。skill 単位制御は現状そもそも不安定。
- 変換方針: Claude→Codex は**情報損失**。MCP ツールのみセッション設定へ「降格」可能（スコープ拡大の**警告フラグ必須**）。それ以外（組み込みツール、引数パターン `Bash(git add *)` の動的スコープ）は破棄。

> **3 フィールドの総括**: skill の「誰が呼べるか（user/model）」「実行中だけ何を許すか」という Claude Code の**動的・skill スコープな制御**のうち、Codex で skill レベルで等価なのは (a) のみ。(b) `user-invocable` は仕様として存在せず代替も困難（確定損失）。(c) `allowed-tools`/`disallowed-tools` は **skill スコープでは存在しないが、session / subagent スコープへ「降格」すれば周辺機構（execpolicy `rules`・subagent `config_file`）でかなりの部分を機能的に代替できる**（次の §2.6 で詳述）。相互変換 CLI では (b) を確定損失、(c) を「スコープ降格つき部分代替」として扱うのが妥当。

### 2.6 【深掘り検証 2】`permissions` / `rules` / subagent による「降格代替」

§2.5 は「skill スコープでは不可」を確定した。だが **skill スコープを諦め、session / project / subagent スコープへ降格**すれば、Codex の周辺機構で `allowed-tools` 等をかなり代替できる。これは相互変換 CLI の現実的な変換戦略そのものなので、ソースレベルで確定した代替経路・質・限界を以下にまとめる。

> **重要な補正**: §2.5(c) で「Codex の rules は `prompt`/`forbidden` のみ」と述べたが、これは **managed 層（`requirements.toml`）限定**の制約だった。**user/project 層の `.rules` ファイルでは execpolicy の `allow` decision（＝承認スキップ＝pre-approve）が使える**ことが `codex-rs/execpolicy/src/decision.rs`（"Command may run without further approval."）で確認された。これにより `allowed-tools` のセッション単位での再現が可能になる。

#### (A) `allowed-tools` / `disallowed-tools` → execpolicy `rules` + MCP tool 制御【session/project スコープで部分代替】

Codex のコマンド承認は「execpolicy（`allow`/`prompt`/`forbidden`）→ `approval_policy`（never/on-request/untrusted/granular）→ `sandbox_mode` → `permissions`(filesystem/network)」の多段で決まる。`allow` が pre-approve に相当する。

| Claude の指定 | Codex の代替 | スコープ | 代替の質 |
|---|---|---|---|
| `allowed-tools: Bash(git add *)` | `prefix_rule(pattern=["git","add"], decision="allow")` を `~/.codex/rules/default.rules`(user) or `.codex/rules/*.rules`(project, 要 `trust_level="trusted"`) | session 全体 | **中**（skill スコープ喪失） |
| `allowed-tools: Bash(git *)`（ワイルドカード） | `prefix_rule(["git"], "allow")`（prefix マッチで「git 以降全部 allow」） | session | **中**（Codex はプレフィックスマッチのみ） |
| `allowed-tools: Bash(*)`（全許可） | `approval_policy="never"` or `sandbox_mode="danger-full-access"` | session | **低**（無差別許可） |
| `allowed-tools: <MCP tool>` | `[mcp_servers.X] enabled_tools=[...]` | user/project | **高** |
| `disallowed-tools: Bash(rm -rf *)` | `prefix_rule(["rm","-rf"], "forbidden")` | user/project/managed | **高**（最も忠実） |
| `disallowed-tools: <MCP tool>` | `[mcp_servers.X] disabled_tools=[...]` | user/project | **高** |
| `disallowed-tools: AskUserQuestion`（組み込み） | **代替なし**（公式 API 未文書） | — | **不可** |

- **`permissions` プロファイルの軸ズレ**: Codex `[permissions.<name>]` は **リソース軸**（filesystem path → read/write/deny、network domain → allow/deny）。Claude `allowed-tools` は **ツール軸**（コマンド＋引数）。`network.domains["evil.com"]="deny"` や `filesystem["~/.ssh"]="deny"` で `disallowed-tools` の一部は補完できるが、「ツールの実行可否」とは一致しない。なお `permissions` と `sandbox_mode` は**排他**（どちらか一方）。
- **結論**: コマンド系（Bash）とMCP系は session/project スコープで実用的に代替可。**組み込みツール（`AskUserQuestion` 等）の禁止だけは代替不可**。

#### (B) `model` / `effort` / `context:fork` → subagent + `config_file`【subagent スコープで代替・ただし自動発火しない】

Codex には per-subagent 設定が **2 系統**あり、いずれも実装済み（Issue #11701 completed）:
- **系統A**: `config.toml` の `[agents.<name>]` に `config_file`（role-specific config layer へのパス）+ `description` + `nickname_candidates`。
- **系統B**: `~/.codex/agents/<name>.toml`（standalone）。`name`/`description`/`developer_instructions` + **任意の `config.toml` 互換キー**（`model`, `model_reasoning_effort`, `sandbox_mode`, `approval_policy`, `mcp_servers`, `skills.config` …）。

→ **Claude の 1 skill を Codex の 1 subagent にマップ**すれば、以下を束ねて代替できる:

| Claude フィールド | Codex 代替 | 質 |
|---|---|---|
| `model` | agent TOML の `model` | **完全** |
| `effort`（low〜xhigh） | `model_reasoning_effort` | **完全** |
| `effort: max` | `model_reasoning_effort="xhigh"` | **近似**（Codex に `max` なし。`xhigh` が最大） |
| `context:fork` + `agent` | standalone agent TOML + モデルによる `spawn_agent` 呼び出し（`features.multi_agent=true`、`max_depth` 既定 1） | **部分** |
| `allowed-tools`（B経由） | agent TOML の `sandbox_mode`/`approval_policy`/`mcp_servers` | **部分**（subagent 実行中に限定できる） |
| `when_to_use` | agent TOML の `description` | **完全** |

- **本質的限界**: Codex の subagent は **自動 fork しない**。公式記述「Codex only spawns a new agent when you explicitly ask it to do so.」のとおり、モデルが `spawn_agent` ツールを明示的に呼んで初めて起動する。Claude の「skill 発火 → 自動 fork」とは起動契機の質が異なる。
- ただし subagent 経路の利点は、(A) のセッション全体降格と違い **「その subagent の実行中だけ」ツール権限/モデルを限定できる**点。skill スコープに最も近い疑似スコープを作れる。

#### (C) `paths` / `arguments` / `hooks` / `shell` の代替

| Claude フィールド | Codex 代替 | スコープ | 質 |
|---|---|---|---|
| `paths`（glob で自動発火） | **等価なし**。最も近いのは `AGENTS.md` のディレクトリ階層スコープ（glob の dirname に `AGENTS.md` 配置）。`child_agents_md` は階層ガイダンスで代替にならない | directory（cwd 依存） | **不可**（ファイル操作イベント駆動でない） |
| `arguments` / `argument-hint` | Custom Prompts の `$1`-`$9`/`$ARGUMENTS`/`argument-hint`（**deprecated**）。skill 本体に引数機構なし | prompt | **形だけ / 破棄** |
| `hooks`（skill スコープ） | Codex hooks（`features.hooks=true` で opt-in、`[[hooks.*]]`）。**session/project スコープで skill スコープに閉じない** | session/project | **部分**（スコープ降格・要警告） |
| `shell`（bash/powershell） | hooks の `commandWindows`（Windows 用コマンド上書き。shell 選択そのものではない） | hook handler | **部分** |

#### スコープ降格の総括

Claude の skill スコープは Codex に存在しないため、すべての代替は **より広い/別のスコープへ降格**する:
- ツール権限 → **session/project**（`.rules`）または **subagent**（`config_file` の sandbox/approval）
- model/effort → **subagent**（agent TOML）または **profile**
- hooks → **session/project**

降格の本質的コストは「**skill 実行中だけ**」という動的・自動の限定が失われること。session 降格はセッション全体に効き、subagent 降格は `spawn_agent` の明示起動を要する。**CLI はどのフィールドをどのスコープへ降格したかを必ず損失レポートに明記すべき**（§5・§6）。

---

## 3. Plugins 詳細比較

### 3.1 ディレクトリ構造と manifest

**Claude Code**

```
my-plugin/
├── .claude-plugin/plugin.json   # manifest（ここに置くのは plugin.json のみ）
├── skills/<name>/SKILL.md
├── commands/<name>.md           # legacy
├── agents/<name>.md
├── hooks/hooks.json
├── .mcp.json
├── .lsp.json
├── output-styles/ themes/ monitors/ bin/ scripts/
└── settings.json
```

**Codex**

```
my-plugin/
├── .codex-plugin/plugin.json    # manifest
├── skills/<name>/SKILL.md
├── .mcp.json                    # MCP 同梱
├── .app.json                    # アプリ/コネクタ（GitHub/Slack 等）
├── hooks/hooks.json
└── assets/                      # アイコン・ロゴ
```

> **変換メモ**: manifest ディレクトリ名が **`.claude-plugin/` ⇄ `.codex-plugin/`** で異なるだけで、内部の `skills/`・`hooks/`・`.mcp.json` の配置思想は共通。Codex の `.app.json`（コネクタ）は Claude Code に直接の対応がない。Claude の `output-styles/`・`themes/`・`monitors/`・`lspServers`・`bin/` は Codex plugin に対応物が未確認。

### 3.2 plugin.json フィールド対応

| Claude Code | Codex | 変換可否 | 備考 |
|---|---|---|---|
| `name`（必須, kebab-case） | `name` | ◎ | |
| `version`（semver, 省略時 git SHA） | `version` | ◎ | |
| `description` | `description` | ◎ | |
| `author`（object） | `author`（object: name/email/url） | ◎ | |
| `homepage` / `repository` / `license` / `keywords` | （一部 `interface` 配下に類似） | ○/△ | Codex は `interface.category` 等に集約傾向 |
| `displayName` | `interface.displayName` | ○ | 位置が異なる |
| `skills`（path, デフォルト skills/ に追加） | `skills`（`"./skills/"`） | ◎ | |
| `commands`（path, 置換） | （Codex は commands 概念薄い／prompts は別） | △ | |
| `agents`（path, 置換） | （config.toml の `[agents.*]` 側） | △ | |
| `hooks`（path/inline） | `hooks`（`"./hooks/hooks.json"`） | ○ | |
| `mcpServers`（path/inline） | `mcpServers`（`"./.mcp.json"`） | ○ | |
| `lspServers` | （未確認） | ✕ | |
| `outputStyles` / `experimental.themes` / `experimental.monitors` | （未確認） | ✕ | Claude 固有 |
| `userConfig`（型付き設定入力 UI） | （`interface.capabilities` 等。等価な型付き入力は未確認） | △ | Claude の方が宣言的設定入力が強力 |
| `defaultEnabled` | `policy.installation`（marketplace 側） | △ | |
| `dependencies`（plugin 依存, semver） | （未確認） | △ | |
| `channels`（メッセージ注入） | `.app.json`（コネクタ）に近い思想 | △ | |
| （Claude に無し） | `interface.brandColor` / `composerIcon` / `logo` / `capabilities` | — | Codex 固有 UI メタデータ |

### 3.3 marketplace.json 対応

| Claude Code | Codex | 変換可否 |
|---|---|---|
| 配置 `.claude-plugin/marketplace.json` | `.agents/plugins/marketplace.json`（**`.claude-plugin/marketplace.json` も互換読み込み**） | ◎ Codex が Claude 形式を吸収 |
| `name`（必須, kebab-case, 予約名禁止） | `name`（必須） | ◎ |
| `owner`（必須, name/email） | （未確認だが類似メタ） | ○ |
| `plugins[]`（必須） | `plugins[]`（必須） | ◎ |
| `plugins[].name` | `plugins[].name` | ◎ |
| `plugins[].source`（string / github / url / git-subdir / npm） | `plugins[].source`（`{source:"local"/"github"/..., path/repo}`） | ○ source 種別の書式差 |
| （Claude に無し） | `plugins[].policy`（`installation: AVAILABLE`, `authentication: ON_INSTALL`） | — | Codex 固有 |
| `metadata.pluginRoot` / `version` / `description` | （類似） | ○ |
| `allowCrossMarketplaceDependenciesOn` | （未確認） | △ |
| `plugins[]` の `category`/`tags`/`strict`/`defaultEnabled` 等 | （一部のみ） | △ |

> **変換メモ**: marketplace は **Codex が `.claude-plugin/marketplace.json` を互換パスで読む**ため、Claude → Codex 方向は配置だけなら無変換で動く可能性が高い。ただし `source` の記述形式（Claude は string or typed object、Codex は `{source, path/repo}`）が違うのでスキーマ変換は必要。

### 3.4 内包コンポーネントの仕様差（commands / agents / hooks / mcp）

#### slash commands / prompts
| | Claude Code | Codex |
|---|---|---|
| 形式 | `commands/<name>.md`（skill の legacy）/ skills 推奨 | `~/.codex/prompts/<name>.md`（**deprecated**, skills 推奨） |
| 引数 | `$ARGUMENTS`, `$N`, `$name`, `argument-hint`, `arguments` | `$1`〜`$9`, `$ARGUMENTS`, `$UPPER`, `$$`, `argument-hint`, `description` |
| 呼び出し | `/name` | `/prompts:name [args]` |
| 双方とも | skills への移行を推奨している点が一致 | |

> 引数テンプレートは **Codex の Custom Prompts の方が表現が近い**（`$1`〜`$9`）。Claude の `arguments`(named) ⇄ Codex prompts の `$UPPER` は変換可能。ただし両者とも skills 移行を推奨しており、skills には引数機能が弱いため、過渡的な扱いが必要。

#### agents（サブエージェント）
| | Claude Code | Codex |
|---|---|---|
| 定義 | `agents/<name>.md`（frontmatter: name, description, tools, model, permissionMode, maxTurns, skills, isolation, color, …約 18 フィールド） | `config.toml` の `[agents.<name>]`（config_file, description）+ skill 内 `agents/openai.yaml` |
| 並列度制御 | （セッション側） | `agents.max_threads`, `max_depth`, `job_max_runtime_seconds` |
| 変換 | △ 設計思想が異なる。Claude は markdown ファイル単位、Codex は config テーブル + ロール別 config ファイル | |

> agents の相互変換は **最も設計差が大きい**。Claude の `agents/*.md`（自己完結 markdown）と Codex の `[agents.*]`（config.toml の参照テーブル）は構造が違う。frontmatter の `tools`/`model`/`maxTurns` 等の一部は Codex 側に直接の置き場がなく、要手動。

#### hooks
| イベント | Claude Code | Codex |
|---|---|---|
| 共通 | PreToolUse, PostToolUse, Stop, SessionStart, SubagentStart, SubagentStop, UserPromptSubmit, PreCompact, PostCompact, PermissionRequest | 左記 10 種に対応 |
| Claude 固有 | Setup, UserPromptExpansion, PermissionDenied, PostToolUseFailure, PostToolBatch, Notification, MessageDisplay, TaskCreated, TaskCompleted, StopFailure, TeammateIdle, InstructionsLoaded, ConfigChange, CwdChanged, FileChanged, WorktreeCreate/Remove, Elicitation(Result), SessionEnd（計 30+） | 無し |
| 有効化 | デフォルト有効 | `features.hooks = true` で opt-in |
| 形式 | JSON（hooks.json）, matcher は文字列/正規表現 | TOML（`[[hooks.<Event>]]`）, matcher 正規表現, `command_windows` 上書き |
| hook タイプ | command / http / mcp_tool / prompt / agent | command（中心） |
| 出力制御 | exit code 0/2/その他, JSON で `permissionDecision` 等 | `continue`, `permissionDecision`, `updatedInput`, `decision:block` 等 |

> **変換メモ**: 両者に存在する 10 イベントは変換可能（JSON ⇄ TOML, matcher はほぼそのまま）。**Claude 固有の 20+ イベントと、command 以外の hook タイプ（http/mcp_tool/prompt/agent）は Codex に対応がなく損失**。逆に Codex → Claude は基本イベントが部分集合なので安全。

#### MCP servers
| キー | Claude Code（.mcp.json / JSON） | Codex（config.toml / TOML） | 変換 |
|---|---|---|---|
| 起動コマンド | `command` | `command` | ◎ |
| 引数 | `args` | `args` | ◎ |
| 環境変数 | `env`（object） | `env`（table）/ `env_vars`（転送名リスト） | ○ |
| 作業dir | `cwd` | `cwd` | ◎ |
| HTTP URL | `url` (+ `type:"http"`) | `url` | ◎ |
| HTTP 認証 | `headers` / `oauth` | `bearer_token_env_var` / `http_headers` / `env_http_headers` | ○ |
| 有効/無効 | `disabled: true` | `enabled: false` | ○（反転） |
| タイムアウト | `timeout`（ms） | `startup_timeout_sec` / `tool_timeout_sec`（秒） | △ 単位・粒度差 |
| ツール粒度制御 | （標準では無し） | `enabled_tools` / `disabled_tools` / per-tool `approval_mode` | ✕→ Codex 固有 |
| 常時ロード | `alwaysLoad` | （未確認） | △ |

> **変換メモ**: MCP の **STDIO 中核（command/args/env/cwd）は完全互換**。差は (1) JSON⇄TOML 形式、(2) timeout の単位（ms ⇄ 秒）、(3) 有効/無効フラグの極性（`disabled` ⇄ `enabled`）、(4) Codex のツール単位承認（Claude に無く損失）。

---

## 4. 周辺機構の比較

### 4.1 メモリ／指示ファイル（CLAUDE.md ⇄ AGENTS.md）

| 項目 | Claude Code | Codex |
|---|---|---|
| プロジェクト指示 | `CLAUDE.md` | `AGENTS.md`（Agentic AI Foundation のオープン標準） |
| グローバル指示 | `~/.claude/CLAUDE.md` | `~/.codex/AGENTS.md`（`$CODEX_HOME` 基準） |
| 明示オーバーライド | （階層の近さで優先） | `AGENTS.override.md`（専用ファイル） |
| 階層マージ | ルート→CWD、近い方が優先 | ルート→CWD で連結、後勝ち |
| サイズ上限 | （明示未確認） | `project_doc_max_bytes`（既定 32 KiB） |
| フォールバック名 | — | `project_doc_fallback_filenames` |

> **変換メモ**: ファイル名のリネーム（`CLAUDE.md` ⇄ `AGENTS.md`）と配置だけで意味は保持できる。**AGENTS.md はオープン標準**（Cursor / Jules / Amp 等も採用）なので、変換 CLI のハブ形式として AGENTS.md を採用する設計も検討に値する。

### 4.2 中核設定（settings.json ⇄ config.toml）

| 項目 | Claude Code | Codex |
|---|---|---|
| 形式 | JSON（`settings.json`） | TOML（`config.toml`） |
| スコープ | enterprise/user/project/local の 4 層 | system(`/etc/codex`)/user(`~/.codex`)/project(`.codex`, 要 trust)/profile/CLI の多層 |
| プロファイル | （無し、スコープで管理） | `[profiles.<name>]` / 別ファイル `~/.codex/<name>.config.toml` |
| 権限 | `permissions`（allow/deny ルール） | `approval_policy` + `sandbox_mode` + `[permissions.<name>]`（filesystem/network 粒度） |
| 強制設定 | managed settings | `requirements.toml` |
| 優先順位 | enterprise > project > local > user | CLI > `-c` > project > profile > user > system > 既定 |

> **変換メモ**: settings.json と config.toml は **粒度・思想が大きく異なり、機械的全変換は非現実的**。相互変換 CLI は当面 **Skills / Plugins / MCP / hooks / メモリファイルに対象を絞り、settings ⇄ config の全自動変換は「権限・MCP・hooks など対応の取れる部分集合のみ」に限定**するのが現実的。

### 4.3 / 4.4 MCP・Hooks は §3.4 に統合済み。

---

## 5. 相互変換マトリクス（CLI 設計の核心）

記号: ◎ 無損失 / ○ 形式変換のみ / △ 部分・近似・別ファイル分散 / ✕ 損失（破棄 or 手動）

### 5.1 Skills 変換損失表

| 要素 | Claude → Codex | Codex → Claude |
|---|---|---|
| `name` | ◎ | ◎（命名制約に注意） |
| `description` | ◎ | ◎ |
| `when_to_use` | ○（description へ連結） | —（Codex に無い） |
| 本文 Markdown | ○（`!`cmd`` と `${CLAUDE_*}` を要処理） | ◎ |
| `disable-model-invocation` | △（→ openai.yaml `allow_implicit_invocation`、明示呼び出し挙動に差／§2.5a） | ◎（← openai.yaml） |
| `user-invocable` | ✕（概念なし→破棄＋警告／§2.5b） | —（Codex に無い） |
| `argument-hint` / `arguments` | △（Skill では損失, prompt 経由なら可） | △ |
| `allowed-tools` / `disallowed-tools` | ✕ skill 単位 / △ 降格（`.rules` の `allow`/`forbidden`・MCP・subagent／§2.6A） | —（Codex に無い） |
| `model` / `effort` | ✕ skill 単位 / △ subagent・profile 降格（`max`→`xhigh`／§2.6B） | —（Codex に無い→既定） |
| `context: fork` / `agent` | ✕ skill 単位 / △ subagent 降格（自動 fork 不可・明示 spawn／§2.6B） | —（無い） |
| `paths`（glob 発火） | ✕（等価なし。`AGENTS.md` 配置で近似のみ／§2.6C） | —（無い） |
| `hooks`（skill スコープ） | ✕ skill 単位 / △ session/project 降格（§2.6C） | —（無い） |
| `shell` | △（hooks の `commandWindows` へ） | —（無い） |
| Codex `interface.*`（UI メタ） | —（Claude skill に枠弱い） | △（plugin displayName 等へ） |
| Codex `dependencies.tools`（MCP 依存） | —（無い） | △（plugin mcpServers へ） |

**要約**: Codex → Claude は **ほぼ無損失**。Claude → Codex は **frontmatter の 14 / 16 フィールドが要注意**で、特に `allowed-tools` / `model` / `effort` / `context:fork` / `paths` は等価変換不可。

### 5.2 Plugins 変換損失表

| 要素 | Claude → Codex | Codex → Claude |
|---|---|---|
| manifest 基本（name/version/description/author） | ◎ | ◎ |
| manifest ディレクトリ名 | ○（`.claude-plugin/` ⇄ `.codex-plugin/`） | ○ |
| `skills/` 同梱 | ◎ | ◎ |
| `hooks/` 同梱 | ○（基本 10 イベント） | ○ |
| `.mcp.json` 同梱 | ○（JSON⇄TOML 化が必要な場合あり） | ○ |
| `userConfig`（型付き入力） | △（Codex に等価薄い） | △ |
| `lspServers` / `outputStyles` / `themes` / `monitors` / `bin` | ✕（Codex に無し） | —（無い） |
| `dependencies`（plugin 依存） | △ | △ |
| Codex `.app.json`（コネクタ） | —（無い） | ✕（Claude に無し） |
| Codex `interface.*`（brandColor/logo/capabilities） | —（無い） | △（一部 displayName へ） |
| marketplace.json | ◎（Codex が `.claude-plugin/` 互換読み） | ○（source 形式変換） |

### 5.3 変換不可能・要手動の代表項目（CLI で「警告」を出すべき箇所）

- **Claude → Codex で『スコープ降格すれば部分代替』できるもの（§2.6）**: `allowed-tools`/`disallowed-tools`（→ user/project `.rules` の `allow`/`forbidden`、MCP `enabled_tools`/`disabled_tools`。ただし `AskUserQuestion` 等組み込みツール禁止は除く）, `model`/`effort`（→ subagent agent TOML / profile、`max`→`xhigh`）, `context:fork`+`agent`（→ subagent + 明示 `spawn_agent`）, skill スコープ `hooks`（→ session/project hooks）。いずれも skill→session/subagent へのスコープ降格を伴い、損失レポートに明記必須。
- **Claude → Codex で完全に破棄/手動になるもの**: `user-invocable`（概念なし）, `disallowed-tools` の組み込みツール（`AskUserQuestion` 等）, `paths` の glob 自動発火, `arguments`/`argument-hint`（skill に引数機構なし）, plugin の `lspServers`/`outputStyles`/`themes`/`monitors`/`bin`, `userConfig`, 本文中の `` !`cmd` `` 動的注入と `${CLAUDE_*}` 変数, command 以外の hook タイプ（http/mcp_tool/prompt/agent）, Claude 固有 hook イベント 20+。
- **Codex → Claude で破棄/手動になるもの**: `agents/openai.yaml` の `interface.*`（アイコン・brand_color 等）、`.app.json`（コネクタ）、MCP のツール単位 `approval_mode`/`enabled_tools`、`config.toml` の `[permissions.*]` 細粒度・`profiles`・`requirements.toml` の強制ルール。
- **呼び出し記法**: 本文・README 内の `/skill` ⇄ `$skill` の書き換え（自動置換は誤検出リスクあり、検出して提案する方式が安全）。

---

## 6. 相互変換 CLI への設計提言

1. **対象を段階的に絞る**: v1 は **Skills と MCP** に限定すると ROI が高い（対応度 ◎〜○ が多く、損失が少ない）。次に Plugins manifest、Hooks（基本 10 イベント）、メモリファイル。settings ⇄ config 全自動変換は最後（または非対応宣言）。skill の `model`/`effort`/ツール権限まで保ちたい場合は **「skill → subagent」変換モード（§2.6B）をオプション提供**する。
2. **中間表現（IR）を持つ**: Claude/Codex の両方を、上位の **正規化スキーマ（IR）** に写してから出力する設計にする。IR は「両者の和集合」をフィールドとして持ち、各フィールドに `origin`（claude/codex/both）と `lossiness`（lossless/lossy/dropped）を付与。
3. **損失レポートを必須出力にする**: 変換実行ごとに「破棄したフィールド」「近似したフィールド」「手動対応が必要な箇所」を一覧する **conversion report**（`--report` で詳細）を出す。§5 の表がそのまま検査ルールになる。**各フィールドをどのスコープへ降格したか（session/project/subagent）も明記**する（§2.6）。
4. **`agents/openai.yaml` と Claude frontmatter のブリッジ**を専用モジュール化: `disable-model-invocation ⇔ policy.allow_implicit_invocation`、`interface ⇔ displayName/description` のマッピングはここに閉じ込める。
5. **本文スキャナ**を用意: `` !`cmd` ``、`${CLAUDE_*}` / `$ARGUMENTS` / `$N` / `$name`、`/skill` 呼び出し記法を検出し、変換先で無効になるものを警告・置換提案する。
6. **形式変換層を分離**: JSON ⇄ TOML、timeout の ms ⇄ 秒、`disabled` ⇄ `enabled` の極性反転などは、意味変換とは別の薄い層に。
7. **marketplace は互換パスを活用**: Claude → Codex は `.claude-plugin/marketplace.json` をそのまま置ける可能性が高い。`source` スキーマだけ正規化。
8. **バージョン検出を入れる**: Codex 側機能は流動的（§7）。`codex --version` / `claude --version` を見て、未対応機能はスキップ＋警告。
9. **AGENTS.md をハブ候補に**: メモリファイルはオープン標準 AGENTS.md を中心に据えると、将来 Cursor 等へ展開しやすい。
10. **往復テスト（round-trip）**: `claude→codex→claude` で差分が「既知の損失項目」だけになることを検証するゴールデンテストを CI に組む。

### 降格マッピング・エンジンの具体ルール（§2.6 を実装に落とす）

Claude→Codex で skill スコープを失う代わりに機能を保つ「降格」の実装ルール:

- **ツール pre-approve**: `allowed-tools: Bash(<cmd> <args>)` → `prefix_rule(pattern=[<cmd>, <args>...], decision="allow")` を `.codex/rules/<skill>.rules`（project）または `~/.codex/rules/default.rules`（user）へ生成。`disallowed-tools: Bash(...)` → `decision="forbidden"`。MCP ツールは `[mcp_servers.X] enabled_tools`/`disabled_tools` へ。**組み込みツール（`AskUserQuestion` 等）の禁止は変換先なし → 警告して破棄**。
- **skill → subagent**: `model`/`effort`/`context:fork` を持つ skill は `.codex/agents/<skill>.toml`（`model` / `model_reasoning_effort`（`max`→`xhigh`） / `sandbox_mode` / `approval_policy` / `developer_instructions`=skill 本文）を生成し、`config.toml` の `[agents.<skill>]` に `config_file` で参照。`description`=`when_to_use`。`[features] multi_agent=true` を明記。
- **hooks**: skill スコープ hooks → `[[hooks.<Event>]]`（session/project）+「skill スコープではなくなる」警告。`command` タイプのみ移植可、他タイプは破棄。
- **必須の警告出力**: ① スコープ降格（skill→session/subagent） ② 自動 fork → 明示 `spawn_agent` への挙動変化 ③ 組み込みツール禁止・`paths` 自動発火・引数機構の喪失 ④ project 層の `.rules`/`.codex/agents` は `projects.<path>.trust_level="trusted"` が前提である旨。

---

## 7. 未確認事項・仕様の流動性に関する注意

- Codex の **Skills / Plugins / Hooks は新しく、ドキュメントとスキーマが先行**している部分がある。実バイナリでの挙動（特に skill 本文の変数置換、`user-invocable` 相当の有無、plugin の `lspServers` 等の対応）は **要実機検証**。
- Codex **Custom Prompts は deprecated**。引数テンプレートを当てにした変換は過渡的措置とすること。
- Claude Code 側も hook イベントや plugin フィールド（`defaultEnabled`, `experimental.*`）はバージョンで増減する。スキーマ URL（`json.schemastore.org/claude-code-plugin-manifest.json` 等）の追跡を推奨。
- `settings.json` ⇄ `config.toml` の完全対応表は本レポートでは **MCP/hooks/権限の部分集合のみ**確証。全フィールド対応は別途要調査。
- 本レポートの ✕/△ 判定は「現行ドキュメント上の等価フィールドの有無」に基づく。実際には本文への埋め込みで“機能的に”近似できるケースもある（例: `allowed-tools` を本文の指示文として記述）。

---

## 8. 参照 URL

**Claude Code**
- Skills: https://code.claude.com/docs/en/skills
- Agent Skills（API/overview）: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview
- Plugins: https://code.claude.com/docs/en/plugins
- Plugins Reference: https://code.claude.com/docs/en/plugins-reference
- Plugin Marketplaces: https://code.claude.com/docs/en/plugin-marketplaces
- Sub-agents: https://code.claude.com/docs/en/sub-agents
- Hooks: https://code.claude.com/docs/en/hooks
- MCP: https://code.claude.com/docs/en/mcp
- 公式 marketplace 例: https://github.com/anthropics/claude-plugins-official

**OpenAI Codex**
- Skills: https://developers.openai.com/codex/skills
- Custom Prompts（deprecated）: https://developers.openai.com/codex/custom-prompts
- AGENTS.md: https://developers.openai.com/codex/guides/agents-md
- Plugins: https://developers.openai.com/codex/plugins
- Build plugins: https://developers.openai.com/codex/plugins/build
- Hooks: https://developers.openai.com/codex/hooks
- Config Reference: https://developers.openai.com/codex/config-reference
- Config Sample: https://developers.openai.com/codex/config-sample
- MCP: https://developers.openai.com/codex/mcp
- Managed configuration: https://developers.openai.com/codex/enterprise/managed-configuration
- リポジトリ: https://github.com/openai/codex （config.schema.json, docs/skills.md, .codex/skills/）
- AGENTS.md 標準: https://agents.md/
