# ccx — Claude Code ⇄ OpenAI Codex CLI 設定相互変換 CLI

Claude Code（`.claude/`, JSON）と OpenAI Codex CLI（`.codex/`, TOML）の設定 — Skills / Plugins / Hooks / MCP / メモリファイル等 — を**双方向変換**する Rust 製 CLI。
変換ルールは `mappings/*.yaml` に宣言済みで、CLI はそれを解釈・実行するエンジン。**設計は完了済み・実装はこれから**。

## 読む順序（実装前に必読）

1. **`docs/12-cli-spec.md`** — 実装の正本。型・IR・transform・ハンドラ・降格エンジン・CLI・フェーズ・Rust スケルトン
2. **`mappings/SCHEMA.md` + `mappings/*.yaml`** — 変換テーブルの正本データ（287 エントリ）
3. `docs/13-feature-matrix.md` — 機能対応一覧（何が変換可/不可/将来追従か）
4. `docs/11-interop-claude-plugin-in-codex.md` — Codex 側の前提（fail-open・既知バグ等）
5. `README.md` — 全体像・概念対応・損失サマリ（背景理解）

補助: `docs/02`〜`09`（領域別フィールド仕様）、`docs/10`（概念設計）。

## 開発コマンド

```
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt
cargo run -- check <path>
cargo run -- c2x <path>
```

## ディレクトリ構成

```
src/        Rust 実装（スケルトン配置済み、本体は todo!()）
mappings/   変換テーブル YAML（正本データ）
docs/       設計書（正本）
tests/      統合テスト・fixtures
```

## 実装の進め方

`docs/12 §14` のフェーズに従い **M0 から順に**着手する。**MVP は M0 + M1**。
各フェーズの完了条件詳細は `docs/12 §14` を参照。スケルトンは `docs/12 §16` を骨子に埋める。

| フェーズ | 範囲 |
|---|---|
| **M0** | mappings ローダ + 不変条件 assert、IR / transform / report / detect の骨格 |
| **M1** | Skills + MCP 双方向。本文スキャナ + 降格 |
| M2 | Hooks + Memory |
| M3 | Plugins（再帰）+ marketplace |
| M4 | Subagents + Settings 部分集合 |

## 最重要原則

- **`mappings/*.yaml` と `docs/12` が正本**。設計書とコード stub に齟齬があれば `docs/12` に従う
- **降格（skill→session/subagent）はスコープが変わる**。conversion report に**必ず明記**。`dropped` も silent にせず列挙
- **Codex 側は流動的**（詳細は `docs/11`）。降格結果は実機検証を推奨

## 作業規約

細則は `.claude/rules/` にある（`src/**`/`tests/**` を触るとき `rules/rust.md`、`mappings/**` を触るとき `rules/mappings.md`、`docs/**` を触るとき `rules/docs.md` が自動ロードされる）。
