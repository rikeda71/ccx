# メモリ/指示ファイル: Claude Code ⇄ Codex

> ファイル名が `CLAUDE.md` ⇔ `AGENTS.md` とほぼ 1:1 対応し、内容は双方向 lossless 変換が可能。ただし Claude 固有の **@import**・**CLAUDE.local.md**・**managed policy**・**パス条件ロード**・**HTML コメント除去**、Codex 固有の **AGENTS.override.md**・**project_doc_max_bytes (32 KiB)**・**project_doc_fallback_filenames** には直接対応物がなく、変換時に展開・警告・破棄の処理が必要になる。

---

## 0. 概要

Claude Code は `CLAUDE.md` を、OpenAI Codex CLI は `AGENTS.md` を主要な永続指示ファイルとして使用する。両者とも:

- プレーン Markdown 形式
- プロジェクトルートから CWD へ向けて階層的に走査し、ファイルを連結（後勝ち）
- 会話開始時に全件ロードしてコンテキストウィンドウに注入

という共通設計を持つ。`AGENTS.md` は Linux Foundation 系オープン標準（Agentic AI Foundation）として 18+ ツール（GitHub Copilot、Gemini CLI、Aider、Cursor、Zed 等を含む 30+ エージェント）が対応しており、`CLAUDE.md` → `AGENTS.md` 変換はポータビリティ獲得として機能する。

最大の差分は **Claude 固有機能群の消滅リスク**（@import、CLAUDE.local.md、managed、パス条件ロード）と、**Codex の 32 KiB 上限**（@import 展開後のサイズオーバーフロー警告が必要）の 2 点に集約される。

---

## 1. Claude Code 側の仕様

### 配置・ファイル・スコープ

Claude Code は以下のパスを**この順序**でロードし、全件を連結してコンテキストに注入する（root → CWD 方向、後勝ち）。

| スコープ | パス | 共有範囲 | Git 管理 |
|---|---|---|---|
| **Managed policy** | macOS: `/Library/Application Support/ClaudeCode/CLAUDE.md`<br>Linux/WSL: `/etc/claude-code/CLAUDE.md`<br>Windows: `C:\Program Files\ClaudeCode\CLAUDE.md` | マシン上の全ユーザー | 不可（組織強制） |
| **User** | `~/.claude/CLAUDE.md` | 全プロジェクト（自分のみ） | 不可（個人） |
| **Project** | `./CLAUDE.md` または `./.claude/CLAUDE.md` | チーム全員 | 推奨 |
| **Local** | `./CLAUDE.local.md` | 自分のみ（プロジェクト固有） | 不可（.gitignore 推奨） |
| **Subdirectory** | サブディレクトリ内の `CLAUDE.md` | そのディレクトリ作業時のみ | — |
| **Rules** | `.claude/rules/*.md`（再帰的） | プロジェクト or ユーザー | — |

**ロード順の詳細:**

1. Managed policy → User → `./CLAUDE.md` (or `./.claude/CLAUDE.md`) → `./CLAUDE.local.md` の順に読み込む
2. ファイルシステムルートから CWD に向かって上位ディレクトリを走査し、各階層の `CLAUDE.md` + `CLAUDE.local.md` を連結
3. サブディレクトリ内のファイルは**起動時ではなく on-demand**（Claude がそのディレクトリ内のファイルを読んだとき）にロードされる
4. `.claude/rules/*.md` は `paths` frontmatter がなければ起動時にロード、`paths` frontmatter があれば対象ファイル読み取り時にロード
5. Managed policy ファイルは `claudeMdExcludes` で除外不可

### 全フィールド表（Claude 固有機能）

CLAUDE.md 自体に YAML frontmatter フィールドはない（プレーン Markdown）。関連する設定・機構を以下にまとめる。

| 機能 / 設定 | 型 | 場所 | デフォルト | 説明 |
|---|---|---|---|---|
| `@path/to/file` インポート | 構文（Markdown 本文内） | CLAUDE.md 本文 | — | 相対・絶対・`@~/` パスで外部ファイルをインライン展開。最大深度 4 ホップ（再帰）。コードブロック内の `@` は除外 |
| `claudeMdExcludes` | `string[]`（glob） | `.claude/settings.json` または `settings.local.json` | `[]` | 特定 CLAUDE.md を除外するグロブパターン。Managed policy は除外不可 |
| `autoMemoryEnabled` | boolean | `settings.json` | `true` | Auto memory（`~/.claude/projects/.../MEMORY.md`）の有効/無効 |
| `autoMemoryDirectory` | string（絶対パス or `~/`） | `settings.json` | `~/.claude/projects/<project>/memory/` | Auto memory の保存先 |
| `claudeMd` | string | `managed-settings.json` | — | Managed settings ファイル内に直接 CLAUDE.md 内容を埋め込む機構 |
| `paths` frontmatter | `string[]`（glob） | `.claude/rules/*.md` のヘッダ | — | 指定 glob にマッチするファイル読み取り時のみ当該 rules ファイルをロード |
| HTML コメント `<!-- -->` | 構文（Markdown 本文内） | CLAUDE.md 本文 | — | コンテキスト注入前に除去（コードブロック内は保持）。人間向けメモ用 |

**Auto memory ファイル構成:**

```
~/.claude/projects/<project>/memory/
├── MEMORY.md          # インデックス（最大 200 行 or 25 KB をセッション開始時にロード）
├── debugging.md       # トピック別詳細ノート（on-demand 読み込み）
└── ...
```

---

## 2. Codex 側の仕様

### 配置・ファイル・スコープ

Codex は以下の順序で候補ファイルを探索し連結する。

| スコープ | パス | 説明 |
|---|---|---|
| **Global** | `$CODEX_HOME/AGENTS.override.md`（優先）<br>`$CODEX_HOME/AGENTS.md`（フォールバック）<br>（デフォルト: `~/.codex/`） | ユーザーグローバル指示。どちらか1つのみ採用 |
| **Project** | git root → CWD の各階層の `AGENTS.override.md`（優先）<br>または `AGENTS.md`（フォールバック） | 各ディレクトリで最初に見つかった1ファイルのみ採用 |

**ロード順の詳細:**

1. プロジェクトルートは `project_root_markers`（デフォルト: `.git`）で判定。マーカーが見つからない場合は CWD のみを対象とする
2. プロジェクトルートから CWD へ向けて各階層を走査（CWD より深い階層は**走査しない**）
3. 各ディレクトリで `AGENTS.override.md` → `AGENTS.md` → `project_doc_fallback_filenames` の順に探索し、最初に見つかった空でないファイル 1 つを採用
4. 採用したファイルを root → CWD 順に改行 2 つで連結（後勝ち）
5. 全ファイルの合計が `project_doc_max_bytes` を超えた場合は超過分を切り捨て

### 全フィールド表（config.toml の関連設定）

| 設定キー | 型 | デフォルト | 説明 |
|---|---|---|---|
| `project_doc_max_bytes` | `usize` | `32768`（32 KiB） | プロジェクト指示ファイル合計の最大バイト数。超過分は切り捨て |
| `project_doc_fallback_filenames` | `string[]` | `[]`（空） | `AGENTS.md` が見つからない場合に試みる代替ファイル名リスト（例: `TEAM_GUIDE.md`） |
| `project_root_markers` | `string[]` | `[".git"]` | プロジェクトルート判定用マーカーファイル/ディレクトリ名 |
| `features.child_agents_md` | boolean | false | 有効時、AGENTS.md のスコープ・優先順位に関する追加ガイダンスをユーザー指示メッセージに付加 |
| `CODEX_HOME` | 環境変数 | `~/.codex` | Codex のホームディレクトリ（global AGENTS.md の探索元） |

**AGENTS.override.md の動作:**

- 同一ディレクトリに `AGENTS.md` と `AGENTS.override.md` が共存する場合、`AGENTS.override.md` のみが採用され `AGENTS.md` は無視される
- 削除せずに一時的に base instruction を差し替えるための機構

---

## 3. 変換テーブル

`mappings/memory.yaml` の人間可読版。

| id | Claude | Codex | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `memory.filename` | `CLAUDE.md` | `AGENTS.md` | both | ◎ | — | ファイル名リネームのみ。内容はそのまま |
| `memory.project-path` | `./CLAUDE.md` or `./.claude/CLAUDE.md` | `./AGENTS.md`（git root〜CWD 各階層） | both | ◎ | — | `path:remap`。パス規約の付け替え |
| `memory.user-path` | `~/.claude/CLAUDE.md` | `~/.codex/AGENTS.md`（`$CODEX_HOME/AGENTS.md`） | both | ◎ | — | `path:remap` |
| `memory.import-syntax` | `@path/to/file`（本文内） | （対応なし） | claude_to_codex | △ | — | `inline_imports`：インライン展開してから変換。コードブロック内 `@` は除外。展開後 32 KiB 超過リスク → warn |
| `memory.local-file` | `./CLAUDE.local.md` | （対応なし） | claude_to_codex | ✕ | — | Codex に非コミット個人ファイルの概念なし。dropped + warn |
| `memory.managed-policy` | `/etc/claude-code/CLAUDE.md` 等 | （対応なし） | claude_to_codex | ✕ | — | 組織強制ファイルの Codex 等価物なし。dropped + warn |
| `memory.override-file` | （対応なし） | `AGENTS.override.md` | codex_to_claude | ✕ | — | Claude に同階層 `CLAUDE.md` を置換する概念なし。dropped + warn |
| `memory.subdirectory-load` | サブディレクトリ内 `CLAUDE.md`（on-demand） | （CWD より深い階層は走査しない） | claude_to_codex | △ | — | Codex は git root→CWD のみ走査。CWD より深い階層の CLAUDE.md は lossy |
| `memory.rules-paths-frontmatter` | `.claude/rules/*.md` の `paths` frontmatter | （対応なし） | claude_to_codex | ✕ | — | glob 条件付きロードの Codex 等価物なし。dropped + warn |
| `memory.html-comments` | `<!-- -->` 除去（コンテキスト注入前） | （未定義） | both | △ | — | Codex 側での HTML コメント処理は未定義。動作差異あり（notes 参照） |
| `memory.claudeMdExcludes` | `claudeMdExcludes` 設定 | （対応なし） | claude_to_codex | ✕ | — | Codex に特定ファイルを除外する機構なし。dropped + warn |
| `memory.project-doc-max-bytes` | （対応なし・CLAUDE.md は長さ制限なし） | `project_doc_max_bytes`（既定 32 KiB） | codex_to_claude | △ | — | Claude 側に上限なし。CLAUDE.md→AGENTS.md 変換後に超過リスク → サイズチェック warn |
| `memory.fallback-filenames` | （対応なし） | `project_doc_fallback_filenames` | codex_to_claude | ✕ | — | Claude に代替ファイル名の概念なし。dropped |
| `memory.child-agents-md-feature` | （対応なし） | `features.child_agents_md` | codex_to_claude | ✕ | — | Claude に等価フィーチャーフラグなし。dropped |
| `memory.merge-order` | root→CWD 連結・後勝ち（全件ロード） | root→CWD 連結・後勝ち（`project_doc_max_bytes` 上限） | both | △ | — | 上限以内なら動作は同等。超過時は Codex 側が切り捨て→lossy |
| `memory.auto-memory` | `~/.claude/projects/.../MEMORY.md`（Auto memory） | （対応なし） | claude_to_codex | ✕ | — | Codex に自動メモリ機構なし。dropped |

---

## 4. 変換時の注意・既知の落とし穴

### 4.1 @import のインライン展開と 32 KiB 制約

`CLAUDE.md` → `AGENTS.md` 変換で最も注意すべきポイント。

**展開ルール:**
- `@relative/path`（CLAUDE.md からの相対パス）
- `@/absolute/path`（絶対パス）
- `@~/home-relative`（ホームディレクトリ相対）
- コードブロック（``` ` ``` で囲まれた部分）内の `@` は展開しない
- 再帰的インポートは最大 4 ホップ（コミュニティ報告では 5 ホップのケースあり、バージョン依存の可能性あり）

**32 KiB 問題:**
Codex の `project_doc_max_bytes` デフォルト値は `32 * 1024 = 32768` バイトで、**全プロジェクトドキュメントの合計**に適用される。@import を多用する CLAUDE.md はインライン展開後に合計サイズが急増し、超過分が静かに切り捨てられる。変換ツールは展開後サイズをチェックし、28 KiB を超えたら警告を出すことを推奨する（グローバル指示分のバッファを考慮）。

### 4.2 CLAUDE.local.md の消失

`./CLAUDE.local.md` は個人の非コミット設定であり Codex に同等概念がない。変換時は **dropped**。内容を手動で `~/.codex/AGENTS.md`（グローバル）に移動するか破棄するかを選択させること。

### 4.3 Managed policy CLAUDE.md の消失

組織強制の `/etc/claude-code/CLAUDE.md` 等は Codex に対応物がない。`dropped + warn`。組織全体で Codex に移行する場合は `~/.codex/AGENTS.md` のデプロイメント管理（Ansible/MDM 等）を別途検討する必要がある。

### 4.4 AGENTS.override.md → Claude 方向

同一ディレクトリの `AGENTS.md` を差し替えるメカニズムが Claude にはない。Codex → Claude 変換時は override の内容を `CLAUDE.md` にそのまま書き込み、元の `AGENTS.md` との差分管理が必要である旨を警告すること（dropped + warn）。

### 4.5 サブディレクトリ on-demand ロードの非対称性

Claude はサブディレクトリ（CWD より深い階層）内の `CLAUDE.md` を、そのディレクトリ内のファイルが読まれた時点でロードする（on-demand）。Codex は git root → CWD 間のみを走査し、**CWD より深い階層は一切走査しない**。モノレポで各サブパッケージに CLAUDE.md を配置している構成では、Codex 側で読まれないファイルが発生する（lossy）。変換時は CWD より深い階層にある CLAUDE.md を検出して警告すること。

### 4.6 HTML コメントの動作差異

Claude Code はコンテキスト注入前に `<!-- ... -->` ブロックコメントを除去する（コードブロック内は保持）。Codex 側の HTML コメント処理は公式に未定義であり、コメントがそのままモデルに渡る可能性がある。人間向けメモを HTML コメントで書いていた場合は変換後に想定外の挙動が生じることがある（notes に動作差異を明記すること）。

### 4.7 AGENTS.md はオープン標準であることの含意（Claude → Codex 変換のポータビリティ）

`AGENTS.md` は [Agentic AI Foundation（Linux Foundation）](https://agents.md/) が策定するオープン標準で、60,000 以上のオープンソースプロジェクトで採用されている。`CLAUDE.md` → `AGENTS.md` 変換後は Claude Code 以外の 18+ エージェントでも同ファイルが参照されるため、Claude 固有の命令（`/memory`、`/init` コマンド等の参照や `!`cmd`` 動的注入など）が含まれていると他エージェントで誤動作する可能性がある。変換時に Claude 固有構文をスキャンして警告すること。

### 4.8 `/memory` コマンドと Auto memory の Codex 非対応

Claude の `/memory` コマンド（CLAUDE.md 一覧表示・編集）および Auto memory（`~/.claude/projects/.../MEMORY.md`）は Codex に対応物がない。いずれも `dropped`。CLAUDE.md に「Auto memory を参照せよ」等の記述がある場合は変換後に残ってしまうため検出して警告すること。

### 4.9 変換の双方向手順まとめ

**CLAUDE.md → AGENTS.md（claude_to_codex）:**

1. @import を再帰的にインライン展開（最大 4 ホップ、コードブロック内除外）
2. 展開後のファイルサイズを合算して 32 KiB に対するマージンを計算・警告
3. ファイル名を `CLAUDE.md` → `AGENTS.md`、`.claude/` → `.codex/`（グローバル設定の場合は `~/.claude/` → `~/.codex/`）にリネーム
4. `CLAUDE.local.md` を dropped として conversion report に記録 + ユーザー警告
5. Managed policy ファイルを dropped として記録 + ユーザー警告
6. `.claude/rules/*.md` の `paths` frontmatter を dropped として記録
7. HTML コメントの動作差異を notes として report に追記

**AGENTS.md → CLAUDE.md（codex_to_claude）:**

1. ファイル名を `AGENTS.md` → `CLAUDE.md`、`~/.codex/` → `~/.claude/`（グローバルの場合）にリネーム
   - または、既存 CLAUDE.md に `@AGENTS.md` インポートのラッパーを生成する方式も選択可
2. `AGENTS.override.md` を dropped として conversion report に記録 + ユーザー警告
3. `project_doc_fallback_filenames` を dropped として記録
4. `features.child_agents_md` を dropped として記録
5. CLAUDE.md のサイズ上限は公式には存在しないが、200 行超で adherence 低下の注意事項を report に記載

---

## 5. 出典

- Claude Code Memory: https://code.claude.com/docs/en/memory
- Claude Code Large Codebases / Monorepos: https://code.claude.com/docs/en/large-codebases
- AGENTS.md オープン標準（Agentic AI Foundation）: https://agents.md/
- OpenAI Codex AGENTS.md ガイド: https://developers.openai.com/codex/guides/agents-md
- OpenAI Codex GitHub（docs/agents_md.md）: https://github.com/openai/codex/blob/main/docs/agents_md.md
- OpenAI Codex ソース（codex-rs/core/src/agents_md.rs）: https://github.com/openai/codex/blob/main/codex-rs/core/src/agents_md.rs
- OpenAI Codex ソース（codex-rs/config/src/config_toml.rs）: https://github.com/openai/codex/blob/main/codex-rs/config/src/config_toml.rs
