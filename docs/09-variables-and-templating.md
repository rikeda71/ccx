# Variables & Templating: Claude Code ⇄ Codex

> 変数・プレースホルダ・呼び出し記法・動的シェル注入の領域。Claude Code は豊富なテンプレート機能（0-indexed 位置引数・名前付き引数・環境変数・動的シェル注入）を持つが、Codex は Skill 本体に前処理置換ロジックを持たず、Custom Prompts（deprecated）に限定的な 1-indexed 引数展開があるのみ。この非対称性が変換の最大の難所。

## 0. 概要

| 機能カテゴリ | Claude Code | Codex |
|---|---|---|
| 引数全体 | `$ARGUMENTS` | `$ARGUMENTS`（Custom Prompts のみ。Skill 本体は非対応） |
| 位置引数（インデックス） | `$ARGUMENTS[N]`（0-indexed）、`$N`（0-indexed shorthand） | `$1`〜`$9`（1-indexed、Custom Prompts のみ） |
| 名前付き引数 | `$name`（frontmatter `arguments:` 宣言） | `$UPPERCASE_NAME`（`KEY=value` 呼び出し形式、Custom Prompts のみ） |
| 環境変数・セッション変数 | `${CLAUDE_SESSION_ID}` 等 5 種 | なし（Skill 本体に同等の変数なし） |
| 動的シェル注入 | `` !`cmd` ``（インライン）/ ` ```! ` ブロック | なし（Issue #5019 "not planned"） |
| 呼び出し記法 | `/skill-name [args]` | `$skill-name`（引数は Skill 本体では非対応） |
| 名前空間付き呼び出し | `/plugin:skill` | なし |
| リテラル `$` エスケープ | なし | `$$` |

相互変換の難所は3点。
1. **インデックスのずれ**: Claude は 0-indexed（`$0`=最初）、Codex は 1-indexed（`$1`=最初）。機械変換では必ず ±1 ずれる。
2. **動的注入の完全非対応**: Claude の `` !`cmd` `` / ` ```! ` を Codex に変換する手段がなく、リテラルとして残ると誤動作する高リスク。
3. **Codex Skill 本体に前処理置換なし**: `$ARGUMENTS` を含む変数展開はすべて Custom Prompts（deprecated）にのみ有効で、SKILL.md 本文では展開されない。

---

## 1. Claude Code 側の仕様

### 配置・ファイル・スコープ

変数展開は skill 本体（`SKILL.md`）の実行前処理として Claude ランタイムが行う。

| スコープ | 設定の場所 | 説明 |
|---|---|---|
| skill 本体 | `.claude/skills/<name>/SKILL.md` | 全変数・注入が使用可能 |
| legacy command | `.claude/commands/<name>.md` | 同上（後方互換） |
| `disableSkillShellExecution` | `settings.json` | `true` にすると動的注入（`` !`cmd` `` / ` ```! ` ）を全 skill で無効化 |
| `shell` frontmatter | SKILL.md frontmatter | `bash`（既定）/ `powershell`。動的注入の実行シェルを切替 |

### 全フィールド表（変数・プレースホルダ）

| 変数 / 記法 | 型 | 展開タイミング | スコープ | 説明 |
|---|---|---|---|---|
| `$ARGUMENTS` | string | skill 呼び出し時 | skill | `/skill-name` に続けて渡されたテキスト引数全体（空白区切りの複数引数を含む全文字列） |
| `$ARGUMENTS[N]` | string | skill 呼び出し時 | skill | 0-indexed の個別位置引数。`$ARGUMENTS[0]` が最初の引数 |
| `$N`（`$0`, `$1`, `$2`, …） | string | skill 呼び出し時 | skill | `$ARGUMENTS[N]` の shorthand 記法。0-indexed |
| `$name` | string | skill 呼び出し時 | skill | frontmatter の `arguments:` で宣言した名前付き引数。`/skill foo bar` で `$name` が `foo` に置換される（宣言順） |
| `${CLAUDE_SESSION_ID}` | string | skill ロード時 | session | 現在のセッションを識別する UUID |
| `${CLAUDE_EFFORT}` | string | skill ロード時 | session | 現在の effort 設定（`low`/`medium`/`high`/`xhigh`/`max`） |
| `${CLAUDE_SKILL_DIR}` | string | skill ロード時 | skill | この SKILL.md が置かれたディレクトリの絶対パス |
| `${CLAUDE_PROJECT_DIR}` | string | skill ロード時 | project | プロジェクトルート（リポジトリルート）の絶対パス |
| `${CLAUDE_PLUGIN_ROOT}` | string | skill ロード時 | plugin | plugin 内 skill の場合はその plugin ルートの絶対パス。plugin 外は空文字列 |

### 動的シェル注入

skill 本体が Claude に渡される**前**にシェルを実行し、その標準出力で置換する。

| 記法 | 展開形式 | 説明 |
|---|---|---|
| `` !`cmd` `` | インライン置換 | 行頭、または空白・タブ直後に出現。`` !`git log --oneline -5` `` のように使用。コマンドの stdout が文字列として埋め込まれる |
| ` ```! ` ブロック | ブロック置換 | ` ```! ` で始まる fenced コードブロック。ブロック全体がコマンドとして実行され stdout で置換される |
| `disableSkillShellExecution` | 無効化設定 | `settings.json` に `"disableSkillShellExecution": true` で全 skill の注入を無効化 |
| `shell: powershell` | シェル切替 | frontmatter で `shell: powershell` を指定すると注入コマンドを PowerShell で実行 |

### 本文スキャナ用正規表現

CLI が変換前に skill 本体をスキャンして変数・注入を検出する際の正規表現一覧。

| パターン | 検出対象 |
|---|---|
| `\$ARGUMENTS(?:\[(\d+)\])?` | `$ARGUMENTS` および `$ARGUMENTS[N]`（グループ1にインデックス） |
| `\$(\d+)` | `$0`〜`$9` などの位置引数 shorthand（0-indexed） |
| `\$([a-z][a-z0-9_]*)` | `$name` 形式の小文字名前付き引数（frontmatter `arguments:` 宣言との照合が必要） |
| `\$\{CLAUDE_[A-Z_]+\}` | `${CLAUDE_SESSION_ID}` 等の組み込みセッション変数 |
| `^!\`[^\`]+\`` | 行頭インライン動的注入 |
| `^[ \t]+!\`[^\`]+\`` | 空白/タブ直後のインライン動的注入 |
| `^\`\`\`!$` | ブロック動的注入の開始行 |
| `/[\w-]+:[\w-]+` | 名前空間付き呼び出し記法 `/plugin:skill` |
| `\$[\w-]+` | Codex 形式の呼び出し記法 `$skill-name` |

---

## 2. Codex 側の仕様

### 配置・ファイル・スコープ

Codex は Skill 本体（SKILL.md）に対して前処理の変数置換を行わない。`render.rs`/`injection.rs` に展開ロジックが存在せず、モデルが本文を読んで解釈する設計。

| スコープ | 設定の場所 | 説明 |
|---|---|---|
| Custom Prompts のみ | `/prompts:name` 経由 | 変数展開は Custom Prompts（deprecated）にのみ存在 |
| Skill 本体 | `.agents/skills/<name>/SKILL.md` | 変数展開・動的注入なし。`$ARGUMENTS` 等はリテラルとして渡される |
| 動的シェル注入 | — | Issue #5019 "not planned"。実装予定なし |

### 全フィールド表（Custom Prompts の変数・deprecated）

Custom Prompts は `/prompts:name` で呼び出す別概念であり、Skill への移行が推奨されている。

| 変数 / 記法 | 型 | 展開タイミング | スコープ | 説明 |
|---|---|---|---|---|
| `$1`〜`$9` | string | 呼び出し時 | Custom Prompts | **1-indexed** の位置引数。`/prompts:name foo bar` で `$1`=`foo`、`$2`=`bar` |
| `$ARGUMENTS` | string | 呼び出し時 | Custom Prompts | 引数全体の文字列。Skill 本体では**非対応**（公式未記載・ソースにロジックなし） |
| `$UPPERCASE_NAME` | string | 呼び出し時 | Custom Prompts | `KEY=value` 形式で渡すと `$KEY` が `value` に展開される |
| `$$` | string | 呼び出し時 | Custom Prompts | リテラルの `$` 文字に展開されるエスケープシーケンス |

### 動的シェル注入（非対応）

Codex には動的シェル注入機能が存在しない。Issue #5019 で "not planned" として明示的に却下されている。

### 呼び出し記法

| 記法 | 対象 | 説明 |
|---|---|---|
| `$skill-name` | Skill | Codex の Skill 明示呼び出し。引数は Skill 本体では展開されない |
| `/prompts:name KEY=v` | Custom Prompts | Custom Prompts の呼び出し。`KEY=value` 形式で名前付き引数を渡す |
| `/skills` | Skill 一覧 | メニュー形式で Skill を選択 |

---

## 3. 変換テーブル

`mappings/variables.yaml` の人間可読版。

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `variables.arguments-all` | `$ARGUMENTS` | `$ARGUMENTS`（Custom Prompts のみ） | both | lossy | — | 記法同一。Codex では Skill 本体での展開は公式未記載・ソースにロジックなし。Custom Prompts のみ有効。warn:true |
| `variables.arguments-indexed` | `$ARGUMENTS[N]`（0-indexed） | `$(N+1)`（1-indexed） | both | lossy | — | `index_shift:+1`（Claude→Codex）/ `index_shift:-1`（Codex→Claude）。**インデックスずれは最重要の落とし穴**。warn:true |
| `variables.positional-shorthand` | `$N`（0-indexed、`$0`=最初） | `$(N+1)`（1-indexed、`$1`=最初） | both | lossy | — | `index_shift:+1`。`$0`→`$1`、`$1`→`$2`。warn:true |
| `variables.named` | `$name`（frontmatter `arguments:` 宣言） | `$UPPERCASE_NAME`（`KEY=value` 呼び出し） | both | lossy | — | `rename`（小文字→大文字）＋呼び出し記法変更。Codex では Custom Prompts のみ有効。warn:true |
| `variables.session-id` | `${CLAUDE_SESSION_ID}` | （なし） | claude_to_codex | dropped | — | Codex に同等変数なし。変換後にリテラルとして残ると誤動作。warn:true |
| `variables.effort-var` | `${CLAUDE_EFFORT}` | （なし） | claude_to_codex | dropped | — | Codex に同等変数なし。warn:true |
| `variables.skill-dir` | `${CLAUDE_SKILL_DIR}` | （なし） | claude_to_codex | dropped | — | Codex に同等変数なし。warn:true |
| `variables.project-dir` | `${CLAUDE_PROJECT_DIR}` | （なし） | claude_to_codex | dropped | — | Codex に同等変数なし。warn:true |
| `variables.plugin-root` | `${CLAUDE_PLUGIN_ROOT}` | （なし） | claude_to_codex | dropped | — | Codex に同等変数なし。warn:true |
| `variables.inline-injection` | `` !`cmd` ``（行頭/空白直後） | （なし） | claude_to_codex | dropped | — | 動的シェル注入。Issue #5019 "not planned"。変換せず残すとリテラル化して**誤動作の高リスク**。検出して必ず warn:true |
| `variables.block-injection` | ` ```! ` ブロック | （なし） | claude_to_codex | dropped | — | ブロック動的注入。同上・高リスク。warn:true |
| `variables.dollar-escape` | （なし） | `$$`（リテラル `$`） | codex_to_claude | dropped | — | Claude 側に同等エスケープなし。Codex→Claude 変換時は `$` リテラルとして残す必要があるが、Claude 側変数と衝突する恐れあり。warn:true |
| `variables.invocation-slash` | `/skill-name [args]` | `$skill-name`（Skill）/ `/prompts:name KEY=v`（Custom Prompts） | both | lossy | — | 本文・README 中の呼び出し記法を書き換え。誤検出リスクあり（`$` は shell 変数、`/` はパス区切りと衝突）。検出して置換提案方式が安全。warn:true |
| `variables.invocation-namespaced` | `/plugin:skill`（名前空間付き） | （なし） | claude_to_codex | dropped | — | Codex に名前空間概念なし。warn:true |

---

## 4. 変換時の注意・既知の落とし穴

### 4.1 インデックスずれは最重要の落とし穴

Claude の位置引数は **0-indexed**（`$0` が最初の引数）、Codex（Custom Prompts）は **1-indexed**（`$1` が最初の引数）。数値部分を単純にコピーすると常に 1 ずれた引数を参照する。変換 CLI は `$0`→`$1`、`$1`→`$2`、`$ARGUMENTS[0]`→`$1`、`$ARGUMENTS[2]`→`$3` のように `index_shift:+1` を必ず適用し、変換レポートに変換前後のインデックスを明記すること。

### 4.2 動的シェル注入は変換せず必ず警告

`` !`cmd` `` とバックスラッシュフェンスブロック ` ```! ` は Claude 固有機能で、Codex は Issue #5019 で "not planned" として実装拒否している。**変換せず残すとリテラルテキストとして扱われ、挙動が無音で壊れる**（高リスク）。変換 CLI は本文スキャナで必ず検出し、変換レポートに行番号とコマンド内容を列挙して手動対応を促すこと。自動削除は情報損失が大きいため行わず、削除要否をユーザーに委ねること。

### 4.3 `${CLAUDE_*}` 変数は全て削除対象だが手動確認を促す

`${CLAUDE_SESSION_ID}`・`${CLAUDE_EFFORT}`・`${CLAUDE_SKILL_DIR}`・`${CLAUDE_PROJECT_DIR}`・`${CLAUDE_PLUGIN_ROOT}` はすべて Codex 側に同等物がない。自動削除するとロジックが壊れる可能性が高いため、本文スキャナが検出した箇所を変換レポートに列挙し、代替手段（ハードコード・環境変数・別の呼び出しパターン）の検討をユーザーに促す。

### 4.4 `$ARGUMENTS` は Skill 本体では非対応（Codex）

Codex の Custom Prompts（deprecated）では `$ARGUMENTS` が機能するが、SKILL.md 本文での `$ARGUMENTS` 展開は公式ドキュメントに記載がなく、`render.rs`/`injection.rs` にも展開ロジックが存在しないことが確認されている。非対応の可能性が高い。Claude→Codex 変換では `$ARGUMENTS` が含まれる箇所を検出し、Custom Prompts ではなく Skill に変換する場合は warn を出し「Codex Skill 本体では `$ARGUMENTS` が展開されない可能性が高い」と明記すること。

### 4.5 名前付き引数は呼び出し記法ごと変わる

Claude の `$name`（frontmatter `arguments:` 宣言の名前付き引数）は `/skill-name foo` のような位置渡しで機能するが、Codex Custom Prompts の `$UPPERCASE_NAME` は `KEY=value` 形式の明示的な名前渡しでのみ機能する。変換後は呼び出しドキュメントも合わせて更新する必要がある。

### 4.6 `$$` エスケープは Claude に同等物がない

Codex Custom Prompts の `$$`（リテラル `$`）を Claude に変換する場合、Claude 側には同等のエスケープが存在しない。変換後の文字列が Claude の変数（`$name` 等）と衝突しないかを確認する必要がある。衝突する場合は手動での記法変更が必要。

### 4.7 `/plugin:skill` 名前空間付き呼び出しは Codex に相当物なし

Claude の plugin 名前空間記法 `/plugin:skill` は Codex に対応する機能がない。変換時は plugin 名前空間を除去して `$skill` 記法へ変換するか、dropped として手動対応を要求するか選択が必要。名前空間を除去すると同名 skill が複数ある場合に衝突する恐れがある点を警告すること。

### 4.8 正規表現の誤検出リスク

本文スキャナ用正規表現は誤検出を起こしやすい。

- `\$([a-z][a-z0-9_]*)` は shell スクリプト内の変数（`$HOME`、`$PATH` 等）と衝突する。コードブロック内は除外するか検出後に確認プロンプトを出すこと。
- `\$[\w-]+` は Codex 呼び出し記法の検出だが、shell スクリプトや URL と混同しやすい。文脈（本文の prose 部分か code 部分か）を区別すること。
- `^!\`[^\`]+\`` のパターンは Markdown のコードブロック内にある場合も検出される。ネストを考慮した実装が望ましい。

### 4.9 Agent Skills 標準との関係

agentskills.io の Agent Skills 仕様には変数・動的注入の規定が含まれない。これらは Claude Code・Codex それぞれの独自拡張であり、オープン標準への変換では両者とも dropped となる点に注意。

---

## 5. 出典

- Claude Code Skills（変数・動的注入の記法）: https://code.claude.com/docs/en/skills
- Claude Code Slash Commands（`$ARGUMENTS`・引数記法）: https://code.claude.com/docs/en/slash-commands
- OpenAI Codex Custom Prompts（`$1`-`$9`・`$ARGUMENTS`・`$$`、deprecated）: https://developers.openai.com/codex/custom-prompts
- OpenAI Codex CLI Slash Commands（`/prompts:name KEY=v`）: https://developers.openai.com/codex/cli/slash-commands
- OpenAI Codex Skills（Skill 本体の前処理なし）: https://developers.openai.com/codex/skills
- Agent Skills Specification（変数・注入は含まれない）: https://agentskills.io/specification
- GitHub openai/codex Issue #5019（動的注入 "not planned"）: https://github.com/openai/codex/issues/5019
