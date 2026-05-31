# ccx — Claude Code ⇄ OpenAI Codex CLI 設定双方向変換 CLI

**ccx** は Claude Code（`.claude/`、JSON）と OpenAI Codex CLI（`.codex/`、TOML）の設定ファイルを双方向変換する Rust 製 CLI です。
Skills / Plugins / Hooks / MCP / Memory / Subagents / Settings などの全領域を対象とし、変換できないフィールドは conversion report に明記します。

> 調査日: 2026-05-30 / 対象: Claude Code（`code.claude.com/docs`, SchemaStore）, OpenAI Codex CLI（`developers.openai.com/codex`, `github.com/openai/codex`）。
> Codex 側の仕様は流動的です。各 `mappings/*.yaml` の `notes`/`source` を参照してください。

---

## インストール

**事前要件**: Rust 1.80 以上（`cargo` が使えること）

```sh
# リポジトリをクローン
git clone https://github.com/rikeda71/ccx
cd ccx

# ビルド
cargo build --release

# バイナリを PATH に追加
cp target/release/ccx ~/.local/bin/
```

---

## 使い方

### 変換コマンド

```sh
# Claude → Codex 変換
ccx c2x <path>

# Codex → Claude 変換
ccx x2c <path>

# 変換可能性の事前診断（書き込まない）
ccx check <path>
```

`<path>` にはファイル（例: `.claude/skills/deploy/SKILL.md`）またはディレクトリを指定します。
ディレクトリを指定した場合は再帰的に対象ファイルを検出します。

### 基本的な例

```sh
# Claude の SKILL.md を Codex 形式に変換
ccx c2x .claude/skills/deploy/SKILL.md

# Codex の config.toml を Claude 形式に変換
ccx x2c .codex/config.toml

# .mcp.json の変換可能性を診断
ccx check .mcp.json
```

---

## コマンドオプション

`c2x` / `x2c` サブコマンドは以下のオプションを共通で受け付けます。

| オプション | 説明 | 既定値 |
|---|---|---|
| `--out <dir>` | 出力先ディレクトリ | `<path>.converted/` |
| `--only <domains>` | 変換対象ドメインをカンマ区切りで限定（`skills,mcp` など） | 全ドメイン |
| `--scope <project\|user>` | 降格先スコープ（`.rules` / agents の配置） | `project` |
| `--skill-target <auto\|skill\|subagent>` | Skill の変換先を強制指定 | `auto` |
| `--interactive` | グレーケースを TTY 対話で確認する | false |
| `--rewrite-body` | 本文の変数/記法を自動書き換え（既定は検出のみ） | false |
| `--dual-manifest` | plugin を `.claude-plugin/` に残しつつ `.codex-plugin/` を追加生成 | false |
| `--hooks-target <user\|project>` | hooks の書き出し先 | `user` |
| `--report[=json]` | 詳細レポートを出力。`=json` で機械可読 JSON | なし |
| `--dry-run` | 書き込まず report のみ出力 | false |
| `--strict` | dropped が 1 件でもあれば exit 2（CI 用） | false |
| `--keep-claude-frontmatter` | Claude 固有 frontmatter キーを出力に残置 | false |
| `--force` | 既存ファイルへの上書きを許可 | false |

### CI での使い方

```sh
# dropped フィールドがあれば非ゼロ終了（CI に組み込む場合）
ccx c2x .claude/skills/deploy/SKILL.md --strict

# JSON レポートを標準出力に出力
ccx c2x .mcp.json --dry-run --report=json
```

---

## conversion report の見方

変換後、または `--report` 指定時に以下の形式でレポートが出力されます。

```
  ◎ skills.name, skills.description  lossless
  ○ skills.when_to_use  lossy  when_to_use concatenated into description (lossy)
  △ skills.model  lossy (degrade)  skills.model → .codex/agents/deploy.toml (degrade: skills.model→subagent)
  ✕ skills.user-invocable  dropped  Codex に概念なし
  ⚠ body L3: $ARGUMENTS[0] - index-shift to $1
Summary: 2 lossless, 2 lossy(1 degraded), 1 dropped, 1 body-warning
```

| 記号 | 意味 |
|---|---|
| ◎ | lossless: 完全に変換可能 |
| ○ | lossy: 変換されるが意味に差異あり |
| △ | degrade: 別スコープ（`.rules` / subagent TOML）への降格 |
| ✕ | dropped: 変換先なしで破棄 |
| ⚠ | body-warning: 本文に手動確認が必要な記法 |

`--strict` フラグ使用時、dropped が 1 件以上あると exit code 2 で終了します。

---

## 変換ドメインと損失サマリ

| 概念 | Claude Code | OpenAI Codex CLI | 対応度 |
|---|---|---|---|
| 再利用可能な指示 | Skills `SKILL.md` | Skills `SKILL.md` | ◎ ファイル名同一 |
| 配布バンドル | Plugins `.claude-plugin/plugin.json` | Plugins `.codex-plugin/plugin.json` | ○ 構造ほぼ同一 |
| サブエージェント | `agents/*.md` | `[agents.*]` + standalone TOML | △ 設計差大 |
| ライフサイクル hooks | Hooks（30 events） | Hooks（10 events） | ○ Claude が広い |
| MCP サーバー | `.mcp.json`（JSON） | `[mcp_servers.*]`（TOML） | ○ STDIO 共通 |
| 中核設定 | `settings.json`（JSON） | `config.toml`（TOML） | △ 形式・粒度差大 |
| 指示メモリ | `CLAUDE.md` | `AGENTS.md` | ○ 名前差のみ |
| 本文の変数/引数 | `$ARGUMENTS[N]`（0起点）| `$1`-`$9`（1起点） | △ ずれ・損失 |

凡例: ◎ ほぼ無損失 / ○ 形式変換は要るが意味は保持 / △ 設計差が大きく部分的・要手動

### 確定損失フィールド（dropped）

以下のフィールドは Codex に対応概念がなく、変換時に破棄されます:
- `user-invocable`, `paths`（glob 自動発火）, `arguments`/`argument-hint`（引数機構）
- 本文の動的注入（`` !`cmd` ``）と `${CLAUDE_*}` 変数
- plugin の `lspServers`/`outputStyles`/`themes`/`monitors`/`bin`/`userConfig`/`channels`
- Claude 固有 hook イベント（`Stop`, `Notification`, `Setup` ほか 20+）, `http`/`mcp_tool` hook タイプ

---

## プロジェクト構成

```
src/        Rust 実装
mappings/   変換テーブル YAML（287 エントリ・正本データ）
docs/       設計書
tests/      統合テスト・fixtures
```

変換ルールは `mappings/*.yaml` に宣言済みで、CLI はそれを解釈・実行するエンジンです。
CLI の型・処理フロー・降格仕様の詳細は [`docs/12-cli-spec.md`](docs/12-cli-spec.md) を参照してください。

---

## 既知の制限

- Codex 側の仕様は流動的です（plugin 同梱 hooks は `#16430` で未ロードの可能性、skill loader は未知 frontmatter を fail-open で無視、など）。変換結果は実機での動作確認を推奨します。
- `settings.json ⇄ config.toml` の全自動変換は非現実的なため、権限/env/MCP/hooks/model の部分集合に絞っています。
- `--interactive` フラグは現バージョンでは TTY 検出のみ（対話的確認は将来実装）。

---

## 出典

**Claude Code**: `code.claude.com/docs`（skills/plugins-reference/hooks/mcp/settings/memory）, SchemaStore。
**OpenAI Codex**: `developers.openai.com/codex`（skills/plugins/hooks/mcp/config-reference/permissions/subagents）, `github.com/openai/codex`。

各エントリの一次情報 URL は対応する `mappings/*.yaml` の `source` と `docs/*.md` の「出典」節にあります。
