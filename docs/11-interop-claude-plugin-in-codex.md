# Claude plugin を Codex で読み込んだとき: コンポーネント別の有効性

> Codex は Claude plugin を「marketplace カタログとしては読める」が、「中身の機能は一部しか効かない」。
> skill は **name/description しか使われず、Claude 固有の frontmatter 制御は黙って無視される**。
> ただし「無視」は **fail open**（エラーにならずロード成功）なので、静的なテキスト指示だけの skill は Claude → Codex で9割そのまま機能する。

---

## 0. この文書の目的

Claude Code 上で動作している plugin（`.claude-plugin/plugin.json` + 配下の skills/hooks/MCP 等）を、
Codex CLI 環境に持ち込んだとき「何が効いて、何が黙って落ちるか」を一次情報から確定する。

特に以下の3点が核心:

1. plugin カタログが「読める」と「中身の機能が効く」は別物。
2. skill frontmatter の Claude 固有フィールドはロードエラーにならず **黙って無視**（fail open）。
3. `hooks.json` は Issue #16430 により plugin root からロードされないバグが存在する。

---

## 1. plugin 読み込みの実態

### 1-1. Codex の marketplace カタログ発見パス

Codex が marketplace を発見する優先順位（公式 `/codex/plugins/build` より）:

| 優先順 | パス | 備考 |
|---|---|---|
| 1 | curated 公式カタログ | Codex 組み込み |
| 2 | `$REPO_ROOT/.agents/plugins/marketplace.json` | Codex ネイティブ |
| 3 | `$REPO_ROOT/.claude-plugin/marketplace.json` | 公式呼称 **"legacy-compatible marketplace"** |
| 4 | `~/.agents/plugins/marketplace.json` | ユーザーグローバル |

上表の③は Claude Code 互換パスとして Codex が**カタログファイルを読む**だけ。
配下の plugin 本体（`.claude-plugin/plugin.json`）を Codex がネイティブ解釈するわけではない。

### 1-2. ネイティブマニフェストの違い

| 項目 | Claude Code | Codex |
|---|---|---|
| plugin マニフェスト | `.claude-plugin/plugin.json` | `.codex-plugin/plugin.json` |
| 両対応の方法 | — | **デュアルマニフェスト**（両ファイルを並置）が必要 |

Codex がネイティブに読むのは `.codex-plugin/plugin.json`。
`.claude-plugin/plugin.json` を直接解釈するネイティブパスは存在しない。

### 1-3. `codex-plugin-cc` の誤解

`codex-plugin-cc` は「Codex から Claude Code を呼び出す **一方向ブリッジ**」であり、
Claude plugin ↔ Codex plugin の互換レイヤーではない（逆方向・異なる用途）。混同注意。

---

## 2. plugin コンポーネント別の有効性

| コンポーネント | Codex での扱い | 判定 | 根拠・備考 |
|---|---|---|---|
| `skills/<n>/SKILL.md` | 読み込まれ、モデルへ注入される | **有効**（ただし frontmatter 制限あり → 後述） | `injection.rs`, `render.rs` |
| `.mcp.json`（mcpServers） | Codex の MCP 設定と同等に機能 | **有効** | `/codex/plugins/build` |
| `.app.json`（apps） | Codex の apps 設定として読まれる | **有効** | `/codex/plugins/build` |
| `hooks/hooks.json` | ドキュメント上は有効だが **Issue #16430 のバグで plugin root からロードされない可能性が高い** | **バグで効かない可能性** | hook discovery が config フォルダのみスキャンする実装バグ |
| `commands/<n>.md` | Codex に slash command 概念なし | **無視〜要変換** | 変換方法は不確実 |
| `agents/<n>.md` | Codex の agents 形式と異なる | **無視〜部分**（形式差） | agents 形式の乖離 |
| `lspServers` | Codex 非対応 | **無視** | Codex LSP 概念なし |
| `outputStyles` | Codex 非対応 | **無視** | — |
| `themes` | Codex 非対応 | **無視** | — |
| `monitors` | Codex 非対応 | **無視** | — |

**後方互換ポイント**: Codex は hook 実行時に環境変数 `CLAUDE_PLUGIN_ROOT` および `CLAUDE_PLUGIN_DATA` を後方互換設定する。
hook が実行できた場合でも、MCP は効く。

---

## 3. skill frontmatter フィールド別の有効性（核心）

### 3-1. loader の動作

Codex の skill ローダー（`core-skills/loader.rs`）は `deny_unknown_fields` を**使っていない**。
`SkillFrontmatter` 構造体が認識するフィールドは `name` / `description` / `metadata.short-description` のみ。
未知フィールドは**黙って無視（fail open）**される。

> 重要: **ロード自体は成功する**。エラーにもならず、警告も出ない。
> これは Claude 固有の frontmatter を含む SKILL.md をそのまま Codex に持ち込んでも、
> Codex がクラッシュしたり読み飛ばしたりしないことを意味する。

### 3-2. フィールド別判定表

| frontmatter フィールド | Codex で読むと | 判定 | 代替・注記 |
|---|---|---|---|
| `name` | そのまま skill 名として使われる | **使われる** | — |
| `description` | そのまま skill 説明として使われる | **使われる** | — |
| `metadata.short-description` | 読まれる | **使われる** | — |
| `allowed-tools` | 黙って無視 | **無視（エラーなし）** | tool pre-approve は**効かない**。代替: user/project 層 `.rules`（execpolicy） |
| `disallowed-tools` | 黙って無視 | **無視（エラーなし）** | 同上 |
| `disable-model-invocation` | 黙って無視 | **無視（エラーなし）** | 相当は `agents/openai.yaml` の `policy.allow_implicit_invocation: false`。openai.yaml が無ければ暗黙発火 ON がデフォルト |
| `user-invocable` | 黙って無視 | **無視（エラーなし）** | Codex に概念なし |
| `model` | 黙って無視 | **無視（エラーなし）** | skill 単位のモデル指定不可 |
| `effort` | 黙って無視 | **無視（エラーなし）** | 同上（Codex subagent で代替） |
| `argument-hint` | 黙って無視 | **無視（エラーなし）** | — |
| `arguments` | 黙って無視 | **無視（エラーなし）** | — |
| `paths`（glob 自動発火） | 黙って無視 | **無視（エラーなし）** | 自動発火なし |
| `hooks` | 黙って無視 | **無視（エラーなし）** | session/project hooks への降格が必要 |
| `when_to_use` | 黙って無視 | **無視（エラーなし）** | — |
| `shell` | 黙って無視 | **無視（エラーなし）** | — |

**一言まとめ**: `name` と `description` だけが Codex 側で実際に読まれる。
それ以外の Claude 固有フィールドは全て **fail open（ロード成功・機能なし）**。

---

## 4. skill 本文（SKILL.md 本体）の有効性（核心）

### 4-1. 実行モデルの違い

| 項目 | Claude Code | Codex |
|---|---|---|
| 処理方式 | **前処理置換型**: ハーネスが変数展開・シェル実行してからモデルに渡す | **モデル解釈型**: SKILL.md を生のままモデルへ注入（`injection.rs` で確認） |
| 変数展開 | ハーネスが実行 | **モデルが受け取る**（展開なし） |
| 動的注入 | ハーネスがシェル実行 | **ハーネスは何もしない** |

### 4-2. 本文要素別判定表

| 本文の要素 | Codex で読むと | 判定 | リスク |
|---|---|---|---|
| 静的なテキスト指示（自然言語） | そのままモデルに届く | **効く** | なし |
| `$ARGUMENTS` / `$1`-`$9` | 展開されずリテラル文字列としてモデルに届く | **無視（展開なし）** | モデルが「$ARGUMENTS」という文字列を読む |
| `$name` / `${CLAUDE_SKILL_DIR}` 等 | 展開されずリテラルのまま | **無視（展開なし）** | `${CLAUDE_SKILL_DIR}/scripts/x.py` のようなパス参照は**壊れる** |
| `` !`cmd` `` 動的注入 | 実行されずリテラルのまま | **無視〜誤動作リスク** | Issue #5019 (not_planned): Codex には動的注入相当なし |
| `` ```! `` ブロック動的注入 | 実行されずリテラルのまま | **無視〜誤動作リスク** | 同上 |
| 補助ファイル参照（`scripts/`, `references/`, `assets/`） | Codex の `render.rs` も同じ慣習をモデルに案内 | **効く** | パス変数を使わない場合は問題なし |

### 4-3. 実用上の分類

| skill の特性 | Codex での動作 | 推奨 |
|---|---|---|
| 静的なテキスト指示のみ | **9割そのまま機能** | そのまま持ち込み可 |
| `$ARGUMENTS` 等の変数を多用 | 変数展開が機能しない（リテラル文字列が届く） | 変数部分を Codex 向けに書き換えか削除 |
| `` !`cmd` `` 動的注入を使う | 実行されずリテラルのまま届く（誤動作リスク） | 削除または静的な代替に置換 |
| `allowed-tools` 等でツール制御 | tool 制御が全て無効 | `.rules` への降格が必要。無策なら **ツール制御は効かない** |
| `disable-model-invocation` で発火制御 | 暗黙発火 ON のまま（openai.yaml なければ） | `agents/openai.yaml` に `policy.allow_implicit_invocation: false` を設定 |

---

## 5. 全体ポイント整理

### 5-1. 何が効くか（まとめ）

| 機能カテゴリ | 効くか | 補足 |
|---|---|---|
| skill のテキスト指示（静的） | **効く** | 9割そのまま機能 |
| skill の name/description | **効く** | ロードに使われる |
| MCP サーバー（.mcp.json） | **効く** | ほぼ無損失 |
| apps（.app.json） | **効く** | — |
| hooks（hooks.json） | **バグで効かない可能性（#16430）** | plugin root からロードされないバグ |
| skill の変数展開 | **効かない（fail open）** | リテラル文字列として届く |
| skill の動的注入 `` !`cmd` `` | **効かない（誤動作リスク）** | Issue #5019 |
| ツール権限制御（frontmatter） | **効かない（fail open）** | .rules への降格が必要 |
| 発火制御（disable-model-invocation） | **効かない（fail open）** | openai.yaml での設定が必要 |
| commands/ | **効かない（無視）** | Codex に slash command 概念なし |
| lspServers/outputStyles/themes/monitors | **効かない（無視）** | Codex 非対応 |

### 5-2. fail open の意義（再強調）

Codex の `core-skills/loader.rs` は `deny_unknown_fields` を使っていないため:

- **ロードは常に成功する**。Claude 固有の frontmatter を持つ SKILL.md を Codex に持ち込んでも、Codex はクラッシュしない。
- 機能しない部分は**黙って落ちるだけ**（エラーなし・警告なし）。
- これは「Codex は Claude plugin を読めるが、全機能が効くとは限らない」状態の技術的根拠。

---

## 6. 移行時の推奨手順

1. **静的指示 skill**: そのまま持ち込み可。変換不要。
2. **変数・動的注入を使う skill**: 変数部分を削除または Codex の `$1`-`$9` 記法へ書き換え（引数インデックスに注意: Claude は 0 基点、Codex は 1 基点）。
3. **ツール制御 skill**: frontmatter の `allowed-tools`/`disallowed-tools` を user/project 層 `.rules` の `execpolicy allow/forbidden` へ降格。
4. **hooks**: `hooks/hooks.json` は #16430 が修正されるまで、plugin 外（project/session 層）の hooks に移動を推奨。
5. **MCP**: そのまま使用可。変換は軽微（JSON → TOML への書式変換のみ）。
6. **commands/agents/LSP 等**: Codex 相当概念がないため手動変換か破棄。
7. **デュアルマニフェスト**: `.claude-plugin/plugin.json` に加えて `.codex-plugin/plugin.json` を並置。これが Codex にネイティブ認識させる唯一の方法。

---

## 7. 出典

以下はすべて本文記述の根拠となる一次情報 URL:

| 情報 | URL |
|---|---|
| Codex plugins build ガイド（marketplace パス、コンポーネント仕様） | `https://developers.openai.com/codex/plugins/build` |
| Codex hooks リファレンス | `https://developers.openai.com/codex/hooks` |
| Codex skills リファレンス | `https://developers.openai.com/codex/skills` |
| core-skills loader.rs（fail open / SkillFrontmatter 構造体） | `https://github.com/openai/codex/blob/main/codex-rs/core-skills/loader.rs` |
| core-skills model.rs（SkillFrontmatter フィールド定義） | `https://github.com/openai/codex/blob/main/codex-rs/core-skills/model.rs` |
| core-skills injection.rs（モデル解釈型注入の実装） | `https://github.com/openai/codex/blob/main/codex-rs/core-skills/injection.rs` |
| core-skills render.rs（補助ファイル慣習の案内） | `https://github.com/openai/codex/blob/main/codex-rs/core-skills/render.rs` |
| Issue #16430（plugin root hooks.json がロードされないバグ） | `https://github.com/openai/codex/issues/16430` |
| Issue #21753（関連: hook discovery の問題） | `https://github.com/openai/codex/issues/21753` |
| Issue #5019（動的注入 not_planned） | `https://github.com/openai/codex/issues/5019` |
| codex-plugin-cc（Claude Code から Codex を呼ぶ一方向ブリッジ） | `https://github.com/openai/codex-plugin-cc` |
