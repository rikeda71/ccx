# 12. CLI 実装設計書（実装着手レベル）

[10. CLI Design](10-cli-design.md) が概念設計（IR・降格エンジン・損失レポート・ロードマップ）なのに対し、本書は **そのまま実装に着手できる粒度**の設計を与える。対象は「実現可能な範囲」に絞り、価値と難所が集中する **Skills / Hooks / Plugins（＋ MCP は Plugins の部品）** を中核とする。変換は `mappings/*.yaml`（287 エントリ）を**正本**として宣言的に駆動する。

---

## 1. 設計目標とスコープ

### 目標
- `mappings/*.yaml` を読み、Claude Code ⇄ Codex の設定ファイルを**双方向変換**する CLI（`ccx`）。
- 変換のたびに **conversion report**（何が lossless / lossy / dropped / degrade / warn か）を必ず出す。
- 「壊れる変換」を黙って通さない。`dropped` と本文の危険箇所は必ず可視化する。

### スコープ（実現可能な範囲）

| 区分 | 領域 | v1 | v2 | v3+ |
|---|---|---|---|---|
| コア | **Skills**（本文スキャナ含む） | ● | | |
| コア | **Hooks**（JSON⇄TOML 構造変換） | | ● | |
| コア | **Plugins**（skills/hooks/mcp を内包・再帰） | | | ● |
| 部品 | **MCP**（Plugins の部品。単体でも可） | ● | | |
| 随伴 | Memory（CLAUDE.md⇄AGENTS.md） | | ● | |
| 将来 | Subagents / Settings 部分集合 | | | ●(v4) |

- v1 で「単一 skill / .mcp.json の往復変換 + report」が動く（最小で価値が出る単位）。
- Subagents・Settings は設計の射程に入れるが v4 送り（権限の軸違い・全自動非現実的のため部分集合のみ）。

### 非目標（割り切り）
- `settings.json ⇄ config.toml` の**全自動変換はしない**（権限/env/model の部分集合のみ、§15）。
- 本文の**自動書き換えは既定では行わない**（`--rewrite-body` で opt-in。既定は検出＋警告）。
- モデル名対応（`claude-* ⇔ gpt-*`）は**自動推論しない**。コード内 const の**ティア（High/Mid/Low）マッピング**を引く（`model-map.yaml` は廃止、§6.2 参照）。未知モデルは値をそのまま残し warn。
- ラウンドトリップで `lossless` エントリのみ完全一致を保証。`lossy`/`dropped` は既知差分として許容。

---

## 2. 技術選定

**言語: Rust**。理由は以下の通り。

### 2.1 言語比較（コア要件別）

| 要件 | **Rust** | Python | Go | TypeScript |
|---|---|---|---|---|
| config.toml 非破壊マージ | **◎** `toml_edit` | ○ `tomlkit` | ✗ 非破壊 TOML ライブラリなし | ✗ 非破壊 TOML ライブラリなし |
| YAML round-trip（キー順保持） | ○ `serde-saphyr`（コメント非保持） | ○ `ruamel.yaml`（コメント保持） | △ `gopkg.in/yaml.v3` | △ `yaml`(eemeli) Document API |
| JSON | ◎ `serde_json` | ◎ 標準 | ◎ 標準 | ◎ ネイティブ |
| 配布（単一バイナリ・クロスプラットフォーム） | **◎** `cargo dist` / musl / Homebrew tap | △ pip + venv 必要 | ◎ 静的バイナリ | △ Node ランタイム必要 / `npx` |
| Codex 親和（型・merge 流用） | **◎** `codex-rs` クレート流用可 | ✗ | ✗ | △ 型のみ参照可 |
| 開発速度 | △（学習コスト・コンパイル時間） | ◎ | ○ | ○ |

**結論**: Rust が最適。Python は次点（配布が弱い）。Go は配布最強だが TOML 非破壊不可。TypeScript は TOML 非破壊不可で要件不適。

### 2.2 Rust 選定理由

1. **`toml_edit` による config.toml 非破壊マージ**が唯一確実に実現できる。`DocumentMut` API でコメント・順序を保持しつつ `[agents.*]`/`[features]`/`[[hooks.*]]` を追加・upsert できる。codex-rs の `mcp_edit.rs`/`plugin_edit.rs`/`marketplace_edit.rs` が同パターンを実証済み。
2. **Codex 親和**: `codex-config` クレートの型（`ConfigToml`/`AgentsToml`/`HooksToml`/`HookHandlerConfig`/`FeaturesToml`）、`merge.rs` の `merge_toml_values`、`codex-utils-json-to-toml` を流用可能。codex-rs クレートが crates.io 非公開の場合は型コピーが安全な代替手段。`config.schema.json` から `typify` で型生成も選択肢。
3. **単一バイナリ配布**: `cargo dist` で macOS/Linux/Windows クロスビルド + Homebrew tap。`x86_64-unknown-linux-musl` で libc 非依存。LTO + `strip` でサイズ削減（5–15 MB）。
4. **型安全な mappings 処理**: `#[derive(Deserialize)]` で mappings の型を静的に表現でき、IR のフィールド対応を型安全に書ける。

### 2.3 主要ライブラリ

| 用途 | クレート | 備考 |
|---|---|---|
| TOML 読み書き（format 保持） | `toml_edit` | `DocumentMut`、`ArrayOfTables::push()` で非破壊追記。codex-rs の `*_edit.rs` と同パターン |
| TOML 型変換 | `toml`（serde 0.9） | `toml_edit` と併用。型付きデシリアライズに使用 |
| YAML（mappings, frontmatter） | `serde-saphyr` | キー順保持。ただし 0.0.x で API 不安定。コメントは非保持（要件上「割り切り可」） |
| frontmatter 分離 | `gray_matter` | `---` 区切りで frontmatter と本文を分離 |
| JSON | `serde_json` | `Value` 型で汎用値保持 |
| CLI | `clap`（derive） | サブコマンド・フラグを derive マクロで定義 |
| 正規表現 | `regex` | 本文スキャナ（§9）。`once_cell::Lazy` で正規表現を静的初期化 |
| エラー処理 | `anyhow` | `anyhow::Result` / `anyhow::bail!` / `anyhow::Context` で統一 |
| 対話 UI | `dialoguer` | `--interactive` 時の TTY 対話確認（`Select` / `Confirm`） |
| テスト（ゴールデン/スナップショット） | `insta` | ゴールデン比較。`cargo insta review` でスナップショット更新 |
| プロパティテスト（任意） | `proptest` | ラウンドトリップ不変条件の網羅的検証 |

### 2.4 Codex 親和（型・merge 流用）

- `codex-config` の `ConfigToml`・`AgentsToml`・`HooksToml` 等は git dependency または型コピーで流用。
- `merge.rs` の `merge_toml_values` を `config.toml` 非破壊マージに流用可。
- `codex-utils-json-to-toml` を JSON→TOML 変換に流用可（hooks の JSON⇄TOML 構造変換で活用）。
- codex-rs は crates.io 非公開の可能性が高い。その場合は型コピーが安全。`config.schema.json` から `typify` による型生成も選択肢。
- execpolicy の `.rules` は Starlark 形式で `format!` マクロ生成（§8.1）。

### 2.5 配布

`cargo dist` を使い、GitHub Actions で macOS（x86_64/aarch64）・Linux（x86_64-unknown-linux-musl）・Windows のクロスビルドを行う。成果物は単一バイナリ。Homebrew tap で `brew install` に対応。LTO + `strip` でバイナリサイズ 5–15 MB 程度。

### 2.6 弱点

- **学習コスト**: 所有権・ライフタイム・トレイトの習熟が必要。
- **コンパイル時間**: 依存クレートが多いと初回ビルドが遅い（`sccache` 等で緩和可）。
- **`serde-saphyr` が 0.0.x**: API 不安定。YAML 書き込みの仕様変更に追従が必要。
- **toml_edit の散在 array-of-tables 再整列**: ソースに散在した `[[array-of-tables]]` は parse 時に連続位置へ再整列されうる（コメント・値は保持）。この場合は変換レポートで warn を出す。

---

## 3. プロジェクト構成

```
ccx/
├── Cargo.toml                # [[bin]] name = "ccx"
├── mappings/                 # ← 本リポジトリの mappings/*.yaml を同梱（正本）
│   ├── SCHEMA.md
│   └── *.yaml
├── src/
│   ├── main.rs               # エントリポイント（clap derive）
│   ├── cli.rs                # CLI 定義・ディスパッチ
│   ├── core/
│   │   ├── ir.rs             # IR 型定義（§5）
│   │   ├── mappings.rs       # mappings ローダ + 索引（§6.1）
│   │   ├── transforms.rs     # transform レジストリ（§6.2）
│   │   ├── report.rs         # conversion report（§10）
│   │   ├── detect.rs         # ファイル種別判定（§7.1）
│   │   └── serialize/
│   │       ├── mod.rs
│   │       ├── json.rs       # serde_json ラッパ
│   │       └── frontmatter.rs# gray_matter + serde-saphyr ラッパ
│   │       # ※ TOML 書き込みは toml_edit を直接使用（自前エミッタ不要）
│   ├── handlers/
│   │   ├── mod.rs            # Handler トレイト（§7）
│   │   ├── skills.rs
│   │   ├── hooks.rs
│   │   ├── mcp.rs
│   │   └── plugins.rs        # skills/hooks/mcp ハンドラを内部で呼ぶ
│   ├── degrade/
│   │   ├── mod.rs
│   │   ├── rules.rs          # allowed-tools → .rules（execpolicy）
│   │   ├── subagent.rs       # skill(model/effort) → .codex/agents/*.toml
│   │   └── hooks_scope.rs    # skill hooks → session/project hooks
│   └── scanner/
│       └── body.rs           # 本文の変数・記法スキャナ（§9）
├── tests/
│   ├── fixtures/             # 入力サンプル（claude/ , codex/）
│   ├── snapshots/            # insta スナップショット（golden）
│   └── roundtrip.rs
└── README.md
```

> **TOML 書き込みについて**: `core/serialize/` に自前 TOML エミッタは置かない。`config.toml` の読み書き・非破壊マージはすべて `toml_edit::DocumentMut` を使う。`frontmatter.rs` / `json.rs` は `serde-saphyr` / `serde_json` の薄いラッパのみ。

---

## 4. アーキテクチャ概観

```
            ┌─────────────────────────────── pipeline (per file/bundle) ────────────────────────────┐
 入力 ─▶ detect ─▶ parse ─▶ lift(IR) ─▶ map(transform/degrade) ─▶ lower ─▶ serialize ─▶ 出力ファイル群
   │       │         │         │              │                      │          │
   │   ファイル種別  形式パース  領域ハンドラが  mappings 駆動 +        ターゲット  TOML/JSON/MD
   │   (skill/mcp.. )(JSON/TOML/  正規化キーへ   transform レジストリ + IR へ      (toml_edit /
   │              frontmatter)              degrade エンジン                  serde_json)
   └──────────────────────────────────────────────────────────────────────────────▶ report (常時)
```

- **mappings 駆動**: フィールドの対応・損失・transform・degrade は `mappings/*.yaml` に宣言。CLI コードはそれを解釈するエンジン。新フィールド追加は原則 YAML 編集だけで済む。
- **領域ハンドラ**: parse / lift / lower の領域固有部分（本文の扱い、plugin の再帰、TOML の array-of-tables 化）を担当。
- **Plugins ハンドラは統合点**: 配下の skills/hooks/mcp を各ハンドラに委譲して再帰変換し、結果を束ねる。

---

## 5. データモデル（IR）

```rust
// core/ir.rs
use std::collections::HashMap;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tool { Claude, Codex }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loss { Lossless, Lossy, Dropped }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind { Skill, Plugin, Subagent, Hooks, Mcp, Memory, Settings }

#[derive(Debug, Clone)]
pub struct IRField {
    pub id: String,                       // mappings の entry id（例 "mcp.timeout"）
    pub value: Value,                     // lift 後の正規化値（serde_json::Value）
    // origin フィールドは削除。どちら由来かは IRNode.source_tool で一元管理する（重複排除）。
    pub loss: Loss,
    pub transforms_applied: Vec<String>,  // 適用した transform 名（report 用）
    pub degrade: Option<DegradeInfo>,     // 降格が起きた場合
    pub warning: Option<String>,          // warn:true 起因の警告
    pub dropped: Option<DroppedInfo>,
}

#[derive(Debug, Clone)]
pub struct DegradeInfo { pub to: String, pub target: String }

#[derive(Debug, Clone)]
pub struct DroppedInfo { pub reason: String }

#[derive(Debug, Clone)]
pub struct BodySegment {                  // skill/command/prompt 本文の解析結果
    pub raw: String,
    pub findings: Vec<BodyFinding>,       // 変数・記法・動的注入の検出（§9）
}

#[derive(Debug, Clone)]
pub struct IRNode {
    pub kind: Kind,
    pub source_tool: Tool,
    pub source_path: String,
    pub fields: HashMap<String, IRField>, // id -> field（順序不要な検索用）
    pub body: Option<BodySegment>,
    pub children: Vec<IRNode>,            // plugin が内包する skills/hooks/mcp
    pub side_artifacts: Vec<SideArtifact>,// 降格で生成する追加ファイル（.rules / agents.toml 等）
    pub diagnostics: Vec<Diagnostic>,    // warn / dropped / degrade の記録
}

#[derive(Debug, Clone)]
pub struct SideArtifact { pub path: String, pub content: String, pub note: String }

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagLevel,
    pub id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagLevel { Info, Warn, Drop }
```

> IR を経由する利点: (a) 双方向を1モデルで扱う、(b) `diagnostics` を機械集計して report 化、(c) ラウンドトリップで IR 差分を比較できる。

---

## 6. mappings ローダ・transform・TOML 入出力

### 6.1 mappings ローダ（`core/mappings.rs`）
```rust
// core/mappings.rs
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct MapEntry {
    pub id: String,
    pub claude: Option<FieldSpec>,
    pub codex:  Option<FieldSpec>,
    pub direction: MappingDirection,  // mappings YAML 上の方向宣言（Both/ClaudeToCodex/CodexToClaude）
    pub loss: LossSpec,
    pub degrade: Option<DegradeSpec>,
    pub transform: Option<String>,   // "unit:ms_to_sec; rename" など ; 区切り
    pub warn: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldSpec {
    pub field: Option<String>,
    pub r#type: Option<String>,
    pub scope: Option<String>,
}

/// mappings YAML 上のエントリが「どちらの方向に有効か」を示す。
/// pipeline 方向（`ConvDir`）とは別型。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingDirection { Both, ClaudeToCodex, CodexToClaude }

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LossSpec { Lossless, Lossy, Dropped }

#[derive(Debug, Clone, Deserialize)]
pub struct DegradeSpec { pub to: String, pub target: String }

#[derive(Debug, Clone, Deserialize)]
pub struct DomainMap { pub domain: String, pub entries: Vec<MapEntry> }

/// 全 YAML ファイルを読み込み、domain → DomainMap の HashMap を返す。
/// 起動時不変条件を assert（id 一意・direction/loss 値域・degrade⇒lossy）。
pub fn load_mappings(dir: &std::path::Path) -> HashMap<String, DomainMap> { /* ... */ }

/// lift 時に「このフィールドはどの id か」を引く索引を構築する。
/// `ConvDir::C2x` なら claude フィールド名、`ConvDir::X2c` なら codex フィールド名で索引。
pub fn index_by_claude_field(dm: &DomainMap) -> HashMap<String, &MapEntry> { /* ... */ }
pub fn index_by_codex_field(dm: &DomainMap) -> HashMap<String, &MapEntry> { /* ... */ }

/// `MappingDirection` と実行方向 `ConvDir` を照合し、このエントリを適用すべきか判定する。
pub fn applies_direction(entry: &MapEntry, dir: ConvDir) -> bool {
    match (&entry.direction, dir) {
        (MappingDirection::Both, _)                           => true,
        (MappingDirection::ClaudeToCodex, ConvDir::C2x)      => true,
        (MappingDirection::CodexToClaude, ConvDir::X2c)      => true,
        _                                                     => false,
    }
}
```
起動時に全 YAML を読み、`id` 一意性・`direction`/`loss` 値・`degrade⇒lossy` を assert（SCHEMA 不変条件、§13）。

### 6.2 transform レジストリ（`core/transforms.rs`）

#### モデルティアマッピング（`model_tier`）

モデル名対応は `config/model-map.yaml` を廃止し、**コード内 const のティア方式**に変更する。モデル/エイリアスを **3 ティア（High / Mid / Low）に正規化**し、相手側の「その時点の最新モデル名」へ写す。最新モデル名はコード内 const で持ち、新モデルが出たら CLI リリースで const を更新する（ユーザー編集ファイルは持たない）。

```rust
// core/transforms.rs に追加
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier { High, Mid, Low }

/// Claude モデル名 → Tier
pub fn claude_tier(m: &str) -> Option<Tier> {
    if m.contains("opus")   { Some(Tier::High) }
    else if m.contains("sonnet") { Some(Tier::Mid)  }
    else if m.contains("haiku")  { Some(Tier::Low)  }
    else { None }
}

/// Codex モデル名 → Tier
pub fn codex_tier(m: &str) -> Option<Tier> {
    if m.ends_with("-high") || m.ends_with("-xhigh") { Some(Tier::High) }
    else if m.ends_with("-mini") { Some(Tier::Low)  }
    else { Some(Tier::Mid) }
}

// ── その時点の最新モデル名（CLI リリース時に更新するプレースホルダ）──
// ※ 具体的なモデル名は実装時に最新の Codex/Claude リリースを確認して設定すること。
const CODEX_LATEST: &[(Tier, &str)] = &[
    (Tier::High, "gpt-5.x-high"),  // ← リリース時に最新名へ更新
    (Tier::Mid,  "gpt-5.x"),       // ← リリース時に最新名へ更新
    (Tier::Low,  "gpt-5.x-mini"),  // ← リリース時に最新名へ更新
];
const CLAUDE_LATEST: &[(Tier, &str)] = &[
    (Tier::High, "claude-opus-latest"),   // ← リリース時に最新名へ更新
    (Tier::Mid,  "claude-sonnet-latest"), // ← リリース時に最新名へ更新
    (Tier::Low,  "claude-haiku-latest"),  // ← リリース時に最新名へ更新
];

/// ティアからモデル名を引く
pub fn tier_to_codex(t: Tier)  -> &'static str { /* CODEX_LATEST から検索 */ }
pub fn tier_to_claude(t: Tier) -> &'static str { /* CLAUDE_LATEST から検索 */ }
```

ティア対応方針:
- Claude `opus`（最上位）⇔ Codex の最新・強めモデル（High）
- Claude `sonnet`（中位）⇔ Codex の最新・mini 以外の標準モデル（Mid）
- Claude `haiku`（軽量）⇔ Codex の最新・軽量モデル（Low）
- 逆方向も同様（例: Codex `-high` 系 → Claude `opus` 最新）

`effort` の `max→xhigh` マッピングも `enum_map` transform で統一管理する（mappings に `enum_map:{max:xhigh,high:high,medium:medium,low:low}` を宣言）。未知モデル名は値をそのまま残し warn を出す。

> **往復一貫性の保証**: `codex_tier()` の判定（`-high/-xhigh`→High, `-mini`→Low, その他→Mid）と `CODEX_LATEST` const に登録するモデル名（High→`-high` 系、Mid→mini なし、Low→`-mini` 系）は必ず整合させること。`tier_to_codex(codex_tier(m))` の結果を `codex_tier` に再適用しても同一 `Tier` に戻ることを実装時に確認する（往復テストは §13 を参照）。同様に `claude_tier` と `CLAUDE_LATEST` も整合を保つ。

> **重要**: `format:json_to_toml`・`format:toml_to_json`・`inline_imports` は**値変換関数ではない**。transform レジストリでは **no-op** として登録し、実際の形式変換・import 展開は handler の lower と serializer が担う。mappings に `transform: format:*` と書かれている場合、それは「この変換に形式変換が伴う」という宣言であり、transform 関数に処理を委ねるものではない。

```rust
// core/transforms.rs
use serde_json::Value;
use std::collections::HashMap;

/// pipeline の実行方向（CLI サブコマンドに対応）。
/// mappings エントリの有効方向を示す `MappingDirection` とは別型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvDir { C2x, X2c }

pub struct TransformCtx<'a> {
    pub direction: ConvDir,             // 実行方向（C2x / X2c）
    pub args: Option<HashMap<String, String>>,  // enum_map 等の引数（TransformSpec.args から詰める）
    pub field: &'a MapEntry,
}

pub type TransformFn = fn(&Value, &TransformCtx) -> Value;

/// 1 つの transform 指定を表す。`spec` 文字列を `parse_transform` で分解した結果。
pub struct TransformSpec {
    pub name: String,                            // transform 名（例 "enum_map", "unit:ms_to_sec"）
    pub args: Option<HashMap<String, String>>,   // `enum_map:{max:xhigh}` の `{...}` 部分
}

// 静的レジストリ（once_cell::Lazy で初期化）
pub fn get_transform(name: &str) -> Option<TransformFn> { /* ... */ }

// 登録される transform 関数:
// "unit:ms_to_sec"   : v / 1000.0
// "unit:sec_to_ms"   : (v * 1000.0).round()
// "polarity:invert"  : !v
// "enum_map"         : ctx.args[v] ?? v  （args は apply_transforms が TransformSpec.args から注入）
// "index_shift"      : ctx.direction で +1（$ARGUMENTS[0]→$1）/ -1（$1→$ARGUMENTS[0]）解決
//   transform 名は方向非依存の "index_shift" とし、符号は ctx.direction で解決する。
// "str_to_list:space": v.split_whitespace().collect()
// "list_to_str:space": v.join(" ")
// "rename"           : v そのまま（キー差は lower 側で解決）
// "extract:bearer_env": 正規表現で "Bearer ${TOKEN}" → "TOKEN"
// "path:remap"       : .claude/⇄.agents/ 等のパスを置換
// "format:json_to_toml": v そのまま（no-op。serializer が処理）
// "format:toml_to_json": v そのまま（no-op。serializer が処理）
// "inline_imports"   : v そのまま（no-op。handler の lower が処理）

/// `"unit:ms_to_sec; enum_map:{max:xhigh,high:high}"` を分解し `Vec<TransformSpec>` を返す。
/// `{...}` ブロックは key:value ペアに分解して `TransformSpec.args` に格納する。
pub fn parse_transform(spec: &str) -> Vec<TransformSpec> { /* ... */ }

/// `apply_transforms` は各 `TransformSpec` を順に適用する。
/// `enum_map` 等の引数は `TransformSpec.args` を `TransformCtx.args` に詰めてから `TransformFn` を呼ぶ。
pub fn apply_transforms(
    value: &Value,
    spec: Option<&str>,
    ctx: &TransformCtx,
) -> (Value, Vec<String>) { /* ... */ }
```

### 6.3 TOML 入出力（`toml_edit`）

`config.toml` の読み書き・非破壊マージは `toml_edit::DocumentMut` を使う。**自前エミッタは不要**。

- 既存の `config.toml` を `DocumentMut` として読み込み、`[agents.*]`/`[features]`/`[[hooks.*]]` を追加・upsert して書き戻す。コメント・キー順序は保持される。
- array-of-tables の追記は `ArrayOfTables::push()` で行う。codex-rs の `mcp_edit.rs` と同パターン。
- `toml`（serde 0.9）は型付きデシリアライズの補助として併用する（`toml_edit` との使い分け: 書き込み・構造操作は `toml_edit`、型変換は `toml`）。
- **制限**: ソースに散在した `[[array-of-tables]]` は `toml_edit` の parse 時に連続位置へ再整列されうる（コメント・値は保持）。この場合は変換レポートで warn を出す。
- 既存キーへの上書きは禁止（warn を出してスキップ。例: 既存 `[features] multi_agent = false` に `true` を書こうとした場合はスキップして warn）。
- **部分文字列パッチ（sed 的な文字列置換）は禁止**。`toml_edit` が format 保持を保証するため不要。

---

## 7. 領域ハンドラ

```rust
// handlers/mod.rs
use std::path::Path;

/// skill → skill か skill → subagent かの選択モード。
/// `decide_skill_target` 内で参照し、`LowerOpts.skill_target` として渡す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillTargetMode { Auto, Skill, Subagent }

pub struct LowerOpts {
    pub out: Option<String>,                // 出力先ディレクトリ（省略時はソース隣の *.converted/）
    pub scope: Scope,                       // 降格先スコープ（.rules / agents の ~/.codex/ vs .codex/ 配置）
    pub dual_manifest: bool,               // plugin: .claude-plugin/ を残置しつつ .codex-plugin/ を追加生成
    pub hooks_target: Scope,               // hooks の書き出し先（#16430 回避）
    pub skill_target: SkillTargetMode,     // skill の変換先選択モード（§7.2.1）
    pub interactive: bool,                 // グレーケース TTY 対話確認フラグ
}

#[derive(Clone, Copy)]
pub enum Scope { User, Project }

pub trait Handler {
    fn kind(&self) -> Kind;
    fn detect(&self, path: &Path) -> bool;
    fn parse(&self, path: &Path) -> anyhow::Result<serde_json::Value>;
    /// `dir: ConvDir` で実行方向を受け取る（`MappingDirection` ではなく pipeline 方向）。
    fn lift(&self, parsed: &serde_json::Value, dir: ConvDir) -> anyhow::Result<IRNode>;
    fn lower(&self, ir: &IRNode, dir: ConvDir, opts: &LowerOpts) -> anyhow::Result<EmitPlan>;
}
// lower 内で opts を参照し、出力パス・SideArtifact 配置（~/.codex/... vs .codex/...）を解決する。

pub struct EmitPlan {
    pub files: Vec<EmitFile>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct EmitFile { pub path: String, pub content: String }
```

**`parse` の契約**: `parse(&self, path: &Path) -> anyhow::Result<serde_json::Value>` が返す `Value` は以下の構造を持つ（handler 間で共通の内部表現）:

```json
{
  "frontmatter": { "name": "...", "description": "..." },  // frontmatter キー（YAML or TOML 由来）
  "body": "...",       // frontmatter を除いた本文（Markdown の場合は --- 以降、TOML の場合は空）
  "path": "/abs/path"  // 入力ファイルの絶対パス（IRNode.source_path に引き継ぐ）
}
```

frontmatter キーの serialize 順序は**mappings エントリの定義順**に従う（`serde-saphyr` のキー順保持機能を活用し、出力 frontmatter のキー順は mappings の entry 定義順と一致させる）。TOML/JSON ソースの場合は `frontmatter` にすべてのトップレベルフィールドを格納し `body` は空文字列とする。

共通の lift は「ソースの各フィールド → `index_by_*_field` で entry を引く → `MappingDirection` と `ConvDir` の照合（`applies_direction`） → `apply_transforms` → `IRField` 化」。領域固有は以下。

### 7.1 detect（`core/detect.rs`）

引数がファイルかディレクトリかで分岐して判定する。

**ファイル指定の場合**: パスのファイル名パターンと先頭バイトで即時判定:
- `SKILL.md`（`**/skills/*/SKILL.md`）→ `Kind::Skill`
- `.mcp.json` → `Kind::Mcp`
- `plugin.json`（`.claude-plugin/` or `.codex-plugin/` 下）→ `Kind::Plugin`
- `CLAUDE.md` / `AGENTS.md` → `Kind::Memory`
- `config.toml` → 中身をパースして後述のテーブル判定で確定

**ディレクトリ指定の場合**: `fast-glob`（Rust では `glob` / `walkdir`）で以下のパターンを再帰発見:
```
**/skills/*/SKILL.md         → Kind::Skill
**/.mcp.json                 → Kind::Mcp
**/*plugin.json              → Kind::Plugin（.claude-plugin/ or .codex-plugin/ 配下）
**/CLAUDE.md                 → Kind::Memory
**/AGENTS.md                 → Kind::Memory
```

**config.toml の種別判定**: `toml_edit` でパースし、テーブルの有無で判定する:
- `[mcp_servers]` テーブルあり → `Kind::Mcp`（MCP 設定を含む config）
- `[hooks]` テーブルあり → `Kind::Hooks`（hooks を含む config）
- 両方あり → `Kind::Plugin`（または複合として両ハンドラへ委譲）

**x2c 方向の追加ルール（Codex → Claude）**:
- `.agents/skills/<n>/agents/openai.yaml` を検出したら `Kind::Skill` として扱い、同一ディレクトリの `SKILL.md` と結合して lift する（`policy.allow_implicit_invocation` 等を読み込む）。
- `.codex/agents/<n>.toml` を検出したら `Kind::Subagent` として detect し、`name` フィールドと一致する `.agents/skills/<n>/SKILL.md` があれば対応 skill と結合して lift する（subagent→skill の逆変換）。

**c2x 方向の追加ルール（Claude → Codex）**: `agents/openai.yaml` は lift 時の入力ではなく lower 時の SideArtifact として生成される（§7.2 参照）。

### 7.2 Skills（`handlers/skills.rs`）
- parse: `gray_matter` で frontmatter（YAML）+ body 分離。frontmatter は `serde-saphyr` でデシリアライズ。
- lift: frontmatter キーを `skills.*` entry に対応づけ。**未知/対応なしキー**は `dropped` 診断（Codex→Claude では Codex 側が少ないので主に Claude→Codex で発生）。body は `scan_body()`（§9）に通して `BodySegment` 化。
- lower（c2x）: `name`/`description` を出力。`when_to_use`→description へ連結。`disable-model-invocation` は **degrade エンジン経由ではなく、skill handler の special-case** として処理する: mappings 上は `degrade:null`（降格なし）だが、handler は `fields` 内に `skills.disable-model-invocation == true` が存在する場合、`.agents/skills/<n>/agents/openai.yaml`（内容: `policy:\n  allow_implicit_invocation: false`）を `SideArtifact` として `IRNode.side_artifacts` に追加する。この処理は §16 の `run_degrade` 呼び出しとは別経路。`lift` 側では mappings 通り `polarity:invert` のみ適用し、`lower(c2x)` がこの SideArtifact 生成を行う。`allowed-tools`/`model`/`effort`/`context:fork` は §8 の降格エンジンへ委譲（SideArtifact 生成 + diagnostics）。`disallowed-tools` も §8.1 降格エンジンへ委譲。`argument-hint` は `dropped`。`shell: powershell` の場合は hooks の `commandWindows` への出力を **propose**（warn のみ、自動変換しない）。`paths`/`user-invocable`/`arguments` は `dropped`。
- lower（c2x/x2c 共通）: SKILL.md と同一ディレクトリ配下の非 `.md` ファイル（`scripts/`・`references/`・`assets/` 等）はパス付け替え（`.claude/skills/<n>/` ⇄ `.agents/skills/<n>/`）のみで emit する（内容変換なし）。`${CLAUDE_SKILL_DIR}` を使う参照が本文・スクリプト内に含まれる場合は本文スキャナが警告する。
- **`disable-model-invocation` の取り扱い**: mappings では `degrade:null` だが、handler はこれを特殊ケースとして `agents/openai.yaml` の生成/読込を行う。`degrade` truthy 時に `run_degrade` を呼ぶ §16 のスケルトンロジックとは別経路（openai.yaml 系と本文降格は handler が別途処理する）。
- **重要（fail-open の対称性）**: Codex は未知 frontmatter を黙って無視（[docs/11](11-interop-claude-plugin-in-codex.md)）。CLI も「Claude→Codex で落とすフィールドは出力 frontmatter から除くが、`--keep-claude-frontmatter` を付ければ無害なので残置可」（Codex は無視するだけ）をオプション化。

### 7.2.1 変換先の判断（skill か subagent か）

Claude skill を Codex へ変換する際、**2 つの変換先**がある。

- **skill→skill**（`.agents/skills/<n>/SKILL.md`）: `name`/`description`/本文のみ保持。description マッチで**自動発火・呼び出しが手軽**（`$name` 呼び出しも可）。ただし `model`/`effort`/skill 単位の権限は**失われる**（dropped または session 降格）。
- **skill→subagent**（`.codex/agents/<n>.toml` + `[agents.*]`、§8.2）: `model`/`effort`/権限を subagent に**束ねて保存**できる。ただし **自動 fork せず `spawn_agent` の明示起動**が要るため手軽さは落ちる。

**本質的トレードオフ**: skill＝発火が自動・手軽だが制御を失う／subagent＝制御を保存できるが明示起動。

**判断表**:

| skill の特徴 | 推奨変換先 | 理由 |
|---|---|---|
| `model` / `effort` 指定あり | **subagent** | skill では保てない（subagent の `model`/`model_reasoning_effort` でのみ保存） |
| `context: fork` あり | **subagent** | 元々サブエージェント実行を意図している |
| `allowed-tools`/`disallowed-tools` で skill 限定の権限が要る（session 全体に広げたくない） | **subagent** | `[permissions.*]` を subagent に閉じ込め（§8.1 のスコープ選択） |
| 純粋な指示（上記の制御 frontmatter なし、name/description/本文中心） | **skill** | 自動発火の手軽さを保つ。損失も小さい |
| 権限はあるが session 降格で許容でき、かつ自動発火も欲しい | **要判断（対話 / 引数）** | 手軽さ vs 権限保存のトレードオフ |

判断は CLI の `--skill-target` フラグ（§11）と `decide_skill_target()` 関数（§16）で制御される。`auto` モードでは決定的ケースを自動判定し、グレーケースは `--interactive` で対話確認、または保守的デフォルト（権限あり → subagent）を採用する。

### 7.3 Hooks（`handlers/hooks.rs`）
- JSON `{"hooks":{"Event":[{matcher,hooks:[{type,...}]}]}}` ⇄ TOML `[[hooks.Event]]`/`[[hooks.Event.hooks]]`。
- イベント: 共通 10 は `both`。Claude 固有 20+ は c2x で `dropped`。
- hook タイプ: `command` は both（`args`/`shell`/`if`/`once`/`asyncRewake` は c2x で dropped、`commandWindows` は x2c 側）。`http`/`mcp_tool` は dropped、`prompt`/`agent` は「Codex は parse のみ実行せず」を warn。
- hooks/mcp の JSON⇄TOML 形式変換は **handler と `toml_edit` の責務**。mappings の `transform: format:json_to_toml` は「形式変換が伴う」ことの宣言に過ぎず、transform レジストリは no-op として扱う（実際の変換は `toml_edit` が行う）。
- matcher:
  - exact（英数字・`_`・`|` のみ）→ Codex は常に regex なので `"Bash"`→`"^Bash$"`・`"Edit|Write"`→`"^(Edit|Write)$"` に正規化（**lossy+warn**、hooks.yaml 準拠）。
  - wildcard（`"*"` または `""`）→ Codex 向けは `""` に正規化（全マッチ、**lossy+warn**）。Codex で `"*"` はそのまま regex 評価されるため期待通りに動かない。
  - 変換規則まとめ: `"Bash"` → `"^Bash$"`（single exact）、`"Edit|Write"` → `"^(Edit|Write)$"`（alternation exact）、`"*"` または `""` → `""`（wildcard=全マッチ）。それ以外の regex 的文字を含む場合はそのまま渡して warn。
- hooks の `args` を shell コマンドとして合成する際は `shell_escape`（`shlex::quote` 相当）でシェル特殊文字をエスケープする。`command` フィールドが配列形式（exec form）の場合はスペース結合 + エスケープ、文字列形式（shell form）はそのまま emit する。
- 出力 JSON のネスト差（Claude `hookSpecificOutput.*` ⇄ Codex flat）は**フック側スクリプトの責務**なので CLI は触らない（変換するのは設定であって hook の入出力契約ではない）旨を notes に。
- **plugin 同梱 hooks**: c2x で出力しても **Codex は #16430 で plugin root の hooks を読まない**。→ `--hooks-target=user|project` で `~/.codex/hooks.json` か `.codex/config.toml` の `[hooks]` に**書き出す**降格を既定にし、「plugin 同梱では効かない」warn を必ず出す。

### 7.4 MCP（`handlers/mcp.rs`）
- ほぼ mappings 駆動の機械変換。transport 判定（Claude `type` ⇄ Codex は `command`有=stdio/`url`有=http）。`timeout`(ms)⇄`tool_timeout_sec`(sec)、`disabled`⇄`enabled`(polarity)、`headers`⇄`http_headers`(rename)、Bearer 抽出、scopes str⇄list。
- Codex 固有（`enabled_tools`/`approval_mode`/`startup_timeout_sec` 等）は x2c で `dropped`、Claude 固有（`alwaysLoad`/`headersHelper`/`sse`/`ws`）は c2x で `dropped`。
- **http transport の `env` 制限**: http transport では Codex の `env` は使用不可（stdio 専用）。c2x で http transport の `env` は出力せず、以下のフローで変換する:
  1. env エントリの値が `${VAR}` 形式（環境変数参照）の場合 → `env_http_headers` へ変換（ヘッダ名: `env` のキー名、値: 環境変数名 `VAR`）。
  2. リテラル値の場合 → `env_http_headers` に出力せず warn＋手動対応を促す（リテラル値を HTTP ヘッダに安全に含めるかは人間が判断）。
  3. 変換後の `env_http_headers` 形式: `{ "Header-Name": "$VAR" }` または `{ "Authorization": "$API_KEY" }` 等。

### 7.5 Plugins（`handlers/plugins.rs`）— 統合点
1. manifest（`plugin.json`）を mappings 駆動で変換（`.claude-plugin/`⇄`.codex-plugin/` は `path:remap`）。
2. 配下の `skills/`・`hooks/`・`.mcp.json` を**各ハンドラに委譲して再帰変換**し、`IRNode.children` に格納。
3. **c2x での dropped/lossy 分類**（plugins.yaml 準拠）:
   - **lossy+warn**: `commands`・`agents`（dropped ではない。`commands`→`skills` ラッパー提案等の変換を試みる）。
   - **dropped+warn**: `lspServers`・`outputStyles`・`experimental.themes`・`experimental.monitors`・`settings`・`channels`・`userConfig`・`dependencies`。
   - `userConfig` の `${user_config.KEY}` が MCP・hooks 等に残って未解決になるリスクには追加 warn を出す。
4. `marketplace.json` は両者ほぼ同形（Codex は `.claude-plugin/marketplace.json` を legacy-compatible で読む）。`source` スキーマだけ正規化。marketplace 変換時に `policy` が未設定なら既定値（`installation=AVAILABLE`・`authentication=ON_INSTALL`）を自動補完し、変換レポートに明記する。
5. c2x では **デュアルマニフェスト**（`.claude-plugin/` 残置 + `.codex-plugin/plugin.json` 生成）を既定にできる（`--dual-manifest`）。

---

## 8. 降格エンジン（`degrade/`）

skill スコープが Codex に無い分を、別スコープのファイル生成で補う。各降格は **SideArtifact（生成ファイル）＋ diagnostic（降格の記録）** を返す。

### 8.1 allowed-tools / disallowed-tools の降格（ツール種別ごとに振り分け）
`allowed-tools`/`disallowed-tools` は **ツールの種類で降格先が異なる**。Bash はコマンドポリシー、Write/Read/Edit はファイルシステム権限、MCP はサーバ単位の許可リストへ。
```
allowed-tools: ["Bash(git add *)", "Write(**/*.py)"]
        ↓ c2x
# Bash 系 → .codex/rules/<skill>.rules（execpolicy, project スコープ。要 trust_level="trusted"）
prefix_rule(pattern=["git","add"], decision="allow", justification="from skill <name>")
# ファイル系 → [permissions.<skill>].filesystem（config.toml, toml_edit で追記）
[permissions.<skill>.filesystem]
"**/*.py" = "write"
```
`.rules` は Starlark 形式を `format!` で生成。`[permissions.*]` は `toml_edit` で非破壊追記。

**パターン解析（ツール種別ごとの降格先）**:
| Claude パターン | Codex 降格先 |
|---|---|
| `Bash(<cmd> <args>)` | `.codex/rules/*.rules` の execpolicy `allow`/`forbidden`（語を prefix 配列化） |
| `Write(<glob>)` / `Edit(<glob>)` | `[permissions.<name>].filesystem.<glob> = "write"` |
| `Read(<glob>)` | `[permissions.<name>].filesystem.<glob> = "read"` |
| `WebFetch`/`WebSearch` | `[permissions.<name>].network`（ドメイン）or `features.web_search`（粗い on/off） |
| `mcp__<server>__<tool>` | `[mcp_servers.<server>].enabled_tools` / `disabled_tools` |
| 組み込み（`AskUserQuestion` 等） | 変換先なし → **dropped** |

- **Codex の `filesystem` は glob 対応**（`"**/*.env" = "deny"` の前例あり）なので `Write(**/*.py)` → `"**/*.py" = "write"` と**パターン自体は保てる**。ただし **ツール軸（Write）→ リソース軸（filesystem write）への変換**になる点に注意。
- Bash のワイルドカードは末尾なら prefix マッチに丸め、途中（`git add *.py`）なら prefix までに丸めて warn。

**スコープの選択（重要）**: 上記の降格先（`.rules`・`[permissions.*]`）は本来 **session/project スコープ**であり skill 単位ではない。**「その skill が動いている間だけ」を保ちたいなら §8.2 の subagent に束ねる** —— subagent の `config_file` 内に `[permissions.*]` を持たせれば、その subagent の実行中だけ権限が適用され、Codex で **skill 単位権限に最も近い**形になる（ただし subagent は自動 fork せず `spawn_agent` の明示起動が要る）。skill 単位にこだわらなければ session/project へ降格（スコープ拡大、warn）。※ subagent の `config_file` への `[permissions.*]` 同梱は実機検証を推奨。

diagnostic: 「skill→subagent（または session/project）へ降格。元の skill 限定スコープは失われる／明示起動になる」。

### 8.2 skill(model/effort/context:fork) → subagent（`degrade/subagent.rs`）
```
model: opus, effort: max, context: fork
        ↓ c2x
# .codex/agents/<skill>.toml
name = "<skill>"
description = "<when_to_use or description>"
developer_instructions = "<skill 本文>"
model = "<tier_to_codex(claude_tier(model)) でティア変換>"
model_reasoning_effort = "xhigh"   # max→xhigh (enum_map)
# config.toml 追記（toml_edit::DocumentMut で非破壊追記）:
[agents.<skill>]
config_file = ".codex/agents/<skill>.toml"
[features]
multi_agent = true
```
- diagnostic: 「自動 fork ではなく `spawn_agent` の明示起動になる」。

### 8.3 skill hooks → session/project hooks（`degrade/hooks_scope.rs`）
skill frontmatter の `hooks` を session/project の `[hooks.*]` に移送＋「skill スコープではなくなる」warn。

---

## 9. 本文スキャナ（`scanner/body.rs`）

```rust
// scanner/body.rs
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct BodyFinding {
    pub kind: FindingKind,
    pub matched: String,
    pub line: usize,
    pub action: Action,
    pub rewrite: Option<String>,  // action=Rewrite のときの置換後
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingKind {
    ArgIndexed, ArgNamed, EnvVar, DynamicInline, DynamicBlock, InvokeSlash, InvokeNamespaced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action { Rewrite, Warn, Drop }

pub fn scan_body(body: &str, dir: ConvDir) -> Vec<BodyFinding> { /* ... */ }
```
検出パターン（[docs/09](09-variables-and-templating.md) と一致）と c2x の方針（`regex` crate、`once_cell::Lazy` で静的初期化）:

| 検出 | 正規表現（概略） | c2x の action |
|---|---|---|
| 位置引数(0基点) | `\$ARGUMENTS\[(\d+)\]` / `\$(\d+)` | rewrite: index_shift +1（`$ARGUMENTS[0]`→`$1`） |
| bare `$ARGUMENTS` | `\$ARGUMENTS(?!\[)` | warn（Codex は Custom Prompts のみ対応、Skill 本体では非対応） |
| `$$`（x2c のみ） | `\$\$` | rewrite → `$` |
| 名前付き引数 | `\$([a-z][a-z0-9_]*)` | warn（呼び出しが `KEY=value` に変わる） |
| `${CLAUDE_*}` | `\$\{CLAUDE_[A-Z_]+\}` | drop（Codex に同等なし） |
| 動的注入 inline | `(^|\s)!\`[^\`]+\`` | warn（Codex 非対応・literal 化リスク） |
| 動的注入 block | ` ```! ` | warn |
| 呼び出し slash | `/[\w-]+`（本文の手順説明） | warn → `$name` 提案 |
| 名前空間呼び出し | `/[\w-]+:[\w-]+` | drop |

> **注記**: `$0` は bash のスクリプト名（`$0`）と衝突するため、auto-rewrite の対象から除外し propose のみとする（`$ARGUMENTS[0]`→`$1` の変換は `[1]` 以降のみ自動適用し、`[0]` は warn＋propose）。

**rewrite-body の適用フロー**:

- `scan_body(raw, dir)` は**検出のみ**を行い `Vec<BodyFinding>` を返す（本文を書き換えない）。
- `opts.rewrite_body == true` の場合、handler の lower は続けて `rewrite_body(raw: &str, findings: &[BodyFinding]) -> String` を呼ぶ。この関数は `action == Action::Rewrite` の finding のみを対象に本文を置換し、書き換え後の文字列を返す。その結果を `EmitFile.content` に入れて出力する。
- **既定（`rewrite_body == false`）は検出＋report のみ**。本文ファイルはそのまま emit し、`diagnostics` に `Warn` エントリを追加するだけ（誤検出で本文を壊さないため）。

---

## 9.5 バージョン依存と将来の Codex 機能追従

### 9.5.1 バージョン検出

`ccx` は変換時に対象ツールのバージョンを検出し、mappings の `min_version`/`max_version` と照合する。Codex 側機能の有無をバージョン範囲で宣言することで、同じ mappings YAML が複数バージョンにわたって正しく機能する。

### 9.5.2 将来対応可能フィールドの追従方針

mappings 駆動設計の利点として、**将来 Codex に機能が追加されたとき、CLI 本体を改修せず mappings エントリの更新だけで追従できる**。

具体的には、現状 `dropped` になっているフィールドのうち「Codex が実装すれば `both`/`lossy` に昇格できるもの」を `notes` に明記しておく運用を採る:

```yaml
# mappings エントリ例
- id: skill.user-invocable
  claude:
    field: user-invocable
    type: bool
  codex: null
  direction: claude_to_codex
  loss: dropped
  notes: "Codex 未実装のため現状 dropped。Codex に user-invocable 相当が実装されれば both 化可。status: awaiting-codex"
```

「Codex 実装待ち」フィールドの代表例:
- `user-invocable` / `paths`（skill 呼び出し制御）
- `http` / `mcp_tool` hook タイプ（Codex 側 hook 拡張待ち）
- Claude 固有イベント（`PreToolUse`/`PostToolUse` 等の Codex パリティ進行中、#21753）

これらは `notes` に `status: awaiting-codex` を付けておき、Codex がリリースで実装したら担当者が `loss: dropped` → `loss: both`/`lossy` に変更し、`codex.field` と必要な transform を追記するだけで CLI 本体の改修なしに追従できる。

---

## 10. conversion report（`core/report.rs`）

IR の `diagnostics` と各 `IRField` を集計。

```rust
// core/report.rs

/// 各診断エントリの共通表現。
#[derive(Debug, Clone)]
pub struct DiagEntry {
    pub id: Option<String>,     // mappings の entry id（例 "skill.allowed-tools"）
    pub message: String,
}

/// `build_report` が返す集計済みレポート。
/// `dropped`/`degraded` は必ず列挙（silent な切り捨て厳禁）。
pub struct Report {
    pub lossless:      Vec<String>,     // lossless フィールド id 一覧
    pub lossy:         Vec<DiagEntry>,  // lossy 変換（変換成功だが意味差あり）
    pub dropped:       Vec<DiagEntry>,  // 変換先なしで切り捨てられたフィールド
    pub degraded:      Vec<DiagEntry>,  // degrade エンジンで別スコープへ降格されたフィールド
    pub body_warnings: Vec<DiagEntry>,  // 本文スキャナが検出した警告
}

/// IR ノードと EmitPlan から Report を構築する。
pub fn build_report(ir: &IRNode, plan: &EmitPlan) -> Report { /* ... */ }
```

```
$ ccx c2x ./skills/deploy --report
✔ skills/deploy/SKILL.md → .agents/skills/deploy/SKILL.md
  ◎ name, description                         lossless
  ○ when_to_use → description(連結)            lossy
  △ allowed-tools → .codex/rules/deploy.rules  lossy  (degrade: skill→project)
  △ model: opus→gpt-5.x, effort: max→xhigh     lossy  (degrade: skill→subagent .codex/agents/deploy.toml)
  ✕ user-invocable                             dropped (Codex に概念なし)
  ✕ paths                                      dropped
  ⚠ body L42: !`git diff` は Codex で実行されません（要手修正 / --rewrite-body 非対象）
  + 生成: .codex/rules/deploy.rules, .codex/agents/deploy.toml, config.toml(追記)
Summary: 2 lossless, 3 lossy(2 degraded), 2 dropped, 1 body-warning
```
- `--report=json` で機械可読（CI 用）。`--dry-run` は書き込まず report のみ。
- 不変条件: `dropped` と `degrade` は**必ず**列挙。silent な切り捨て厳禁。

---

## 11. CLI インターフェース（`src/cli.rs`）

### 11.0 出力ディレクトリ構造

`--out` 省略時の既定は **入力パスに `.converted` サフィックスを付けたディレクトリ**（例: `.claude/skills/deploy` → `.claude/skills/deploy.converted/`）。プロジェクト全体変換（ルートを入力）の場合は `./.codex-converted/` を既定とする。

| 入力種別 | --out 省略時の出力先 |
|---|---|
| 単一 skill ディレクトリ | `<input>.converted/` |
| `.mcp.json` ファイル | `./<filename>.converted.json` |
| プロジェクトルート | `./.codex-converted/` |

**出力先内のファイル配置**（`--out <root>` を基準とした相対パス）:

| 生成物 | 配置先 |
|---|---|
| 変換後 SKILL.md | `<root>/.agents/skills/<n>/SKILL.md` |
| 変換後 .mcp.json | `<root>/.mcp.json` |
| `.rules`（execpolicy） | `<root>/.codex/rules/<skill>.rules` |
| `.codex/agents/<n>.toml`（subagent） | `<root>/.codex/agents/<n>.toml` |
| `config.toml` 追記 | `<root>/config.toml`（既存があれば非破壊マージ、なければ新規生成） |
| `agents/openai.yaml`（SideArtifact） | `<root>/.agents/skills/<n>/agents/openai.yaml` |

`SideArtifact.path` および `EmitFile.path` はすべて **出力ルートからの相対パス** で保持する。`lower` / `write_plan` でルートを結合して絶対パスに変換する。

### 提供形態

`ccx` は**一方向変換ツール**である（双方向同期はしない。`c2x` と `x2c` は独立した一方向変換）。

変換粒度は以下の3段階を提供する:

| 粒度 | 例 | 説明 |
|---|---|---|
| **単一指定** | `ccx c2x ./.claude/skills/deploy` | 既定の最小単位。1 skill / mcp / hooks ファイル単位 |
| **領域一括** | `ccx c2x . --only skills,mcp` | 指定領域のファイルをまとめて変換 |
| **プロジェクト移行** | `ccx c2x . --out ./codex-project` | ルートを指定して別ディレクトリへ出力 |

**既定は別ディレクトリ出力**（`--out` 省略時は `*.converted/` へ出力）。in-place 書き換えは既定にしない。出力後に `dropped`/`degrade` を conversion report で確認してから手動で採用する運用を推奨する。オールインワン全自動（`--force --no-report`）も可能だが既定にしない。

```
ccx c2x <path> [options]      # Claude → Codex（一方向）
ccx x2c <path> [options]      # Codex → Claude（一方向）
ccx check <path>              # 変換可能性の事前診断（dropped 件数の見積り、書き込まない）

options:
  --out <dir>            出力先（既定: *.converted/ サブディレクトリ）
  --only <domains>       skills,hooks,mcp,plugins のみ（カンマ区切り）
  --scope <user|project> 降格先スコープ（.rules / agents の配置）。既定 project
  --hooks-target <user|project>  hooks の書き出し先（#16430 回避）。既定 user
  --rewrite-body         本文の変数/記法を自動書き換え（既定: 検出のみ）
  --dual-manifest        plugin で .claude-plugin/ を残し .codex-plugin/ を追加生成
  --report[=json]        詳細レポート
  --dry-run              書き込まず report のみ
  --strict               dropped が1件でもあれば非ゼロ終了（CI 用）
  --skill-target <auto|skill|subagent>
                         skill の変換先を選択（既定: auto）。auto は §7.2.1 の判断表に従い自動判定。
  --interactive          auto がグレーと判断した skill について TTY で対話確認する。
                         例: "skill 'deploy' は allowed-tools を持ちます。[s]kill 変換（権限は session 降格・自動発火を維持）/ [a]gent 変換（権限を subagent に束ねる・明示起動）? [s/a]"
  --force                既存ファイルへの上書きを許可
```

**`--skill-target` の決定優先順位**:
1. 明示 `--skill-target skill|subagent` があればそれに従う。
2. `auto`: 決定的ケースは自動（`model`/`effort`/`context:fork` あり → subagent、純粋指示 → skill）。
3. グレーケース（権限あり・session 降格で許容できるかどうか不明）:
   - `--interactive` あり → TTY 対話で確認。
   - 非対話（CI 等）→ 保守的デフォルト（権限/制御がある → subagent[失わない方] / なければ skill）を採用し **report に選択理由を必ず明記**。

**終了コード**: `0` 成功（dropped あっても可）／`1` 入力エラー・パース失敗／`2` `--strict` で dropped 検出。

---

## 12. エラー処理・fail-open 方針

- パース失敗（不正 JSON/TOML/frontmatter）は**そのファイルを skip + error 診断**、他は続行（バンドル変換で1ファイルの失敗が全体を止めない）。
- 未知フィールドは**落とす（drop 診断）が処理は続行**（Codex の loader と同じ fail-open 哲学）。
- 変換先が無いフィールドは黙殺せず必ず diagnostic 化。
- 生成ファイルの**上書きは既定で拒否**（`--force` で許可）。`.rules`/`config.toml` は追記 or マージ（既存を壊さない）。

### config.toml マージ仕様

`config.toml` への書き込み（`[agents.*]`・`[features]`・`[[hooks.*]]` の追加）は `toml_edit::DocumentMut` を使った非破壊追記で行う:

1. **`toml_edit` で既存ファイルを `DocumentMut` として読み込む**
2. **`DocumentMut` を直接操作**して `[agents.*]`/`[features]`/`[[hooks.*]]` を追加・upsert
3. **`DocumentMut::to_string()` で書き戻す**（コメント・順序保持が保証される）

array-of-tables の追記は `ArrayOfTables::push()` で行う。codex-rs の `mcp_edit.rs` と同パターン。Codex の `merge_toml_values`（`merge.rs`）を流用可能。**部分文字列パッチ（sed 的な文字列置換）は禁止**（`toml_edit` が format 保持を保証するため不要）。

既存キーへの上書きは禁止。例: 既存 `[features] multi_agent = false` に `true` を書こうとした場合はスキップして warn を出す。

**注意**: ソースに散在した `[[array-of-tables]]` は `toml_edit` の parse 時に連続位置へ再整列されうる（コメント・値は保持）。この場合は変換レポートで warn を出す。

---

## 13. テスト戦略（`tests/`）

1. **mappings 検証**（起動時 assert をユニットテスト化）: id 一意、`direction`/`loss` の値域、`degrade⇒lossy`、`dropped`に`transform`が無い、`source` 存在。← 既に `python`/`ruby` で 287 件 0 issue を確認済み。これを `cargo test` で CI 化。
2. **ユニット**: 各 transform（`unit:ms_to_sec` 等）、本文スキャナ（`scan_body`）、`toml_edit` による非破壊マージ。
3. **ゴールデン/スナップショット**: `insta` を使用。`tests/fixtures/claude/* → tests/snapshots/*` の固定入出力比較。`cargo insta review` でスナップショット更新。
4. **ラウンドトリップ**: `c2x → x2c` で IR 差分が「既知の lossy/dropped」だけになることを検証。`lossless` フィールドは完全一致を要求。
5. **プロパティテスト**（任意）: `proptest` で mappings 不変条件・ラウンドトリップ不変条件を網羅的に検証。
6. **ティア往復テスト**（§6.2 往復一貫性保証）:
   - `tier_to_codex → codex_tier` の往復: 各 `Tier` に対して `codex_tier(tier_to_codex(t)) == Some(t)` を assert。
   - `tier_to_claude → claude_tier` の往復: 各 `Tier` に対して `claude_tier(tier_to_claude(t)) == Some(t)` を assert。
   - `CODEX_LATEST` / `CLAUDE_LATEST` の const 更新時は必ずこのテストを通してから PR をマージすること。

> **§13.4 degrade 経由フィールドのラウンドトリップ**: degrade の逆変換（x2c）は構造的に復元不能なものがある（session hooks→skill hooks 等）。degrade 経由フィールドについては IR 差分ではなく **side_artifacts の差分で検証**し、「元の SideArtifact が再生成されること」をもって等価とみなす。IR 差分で完全一致を求めない。

---

## 14. 実装フェーズ（マイルストーン）

| M | 内容 | 完了条件 |
|---|---|---|
| **M0** | スキャフォールド: `clap` derive による CLI、mappings ローダ＋assert、IR 型、transform レジストリ、report、detect | `ccx check` が dropped 件数を出す |
| **M1** | **Skills 双方向**（本文スキャナ）＋ **MCP 双方向** | 単一 SKILL.md / .mcp.json の往復 + report。`allowed-tools→.rules`・`skill→subagent` 降格 |
| **M2** | **Hooks 双方向**（JSON⇄TOML array-of-tables、`toml_edit` 利用）＋ Memory（CLAUDE.md⇄AGENTS.md, `@import` inline 展開） | hooks.json ⇄ config.toml、#16430 警告 |
| **M3** | **Plugins**（skills/hooks/mcp を内包・再帰）＋ marketplace ＋ `--dual-manifest` | plugin 一式の往復、デュアルマニフェスト |
| **M4** | Subagents（skill→subagent 統合）＋ Settings 部分集合（権限/env/model）＋ report の CI 統合（`--strict`） | 部分集合の変換 |

「実現可能な範囲」を最短で出すなら **M0+M1 が MVP**（skill と mcp の往復が動き、report が出る）。

---

## 15. 割り切り・非対応・既知の制約

- **Settings 全自動変換はしない**。権限はツール軸⇄リソース軸で機械対応できないため、`permissions.allow(Bash)`→`.rules`、`deny`→`forbidden`、Read/Write→`[permissions.*].filesystem`、env/model の**部分集合のみ**。それ以外は report に列挙して手動に委ねる。
- **モデル名**はコード内 const のティアマッピング（§6.2）で解決する（`model-map.yaml` は廃止）。未知モデルは値をそのまま残し warn。
- **本文の自動書き換え**は `--rewrite-body` opt-in。`!`cmd`` の自動除去はしない（提案のみ）。
- **Codex 側のバグ・流動性**を前提に warn を出す: plugin 同梱 hooks（#16430）、`[[skills.config]]` オーバーライド（#14161）、イベントパリティ進行中（#21753）。
- **コメント・書式の完全保持はしない**（YAML round-trip でコメントが落ちうる。`serde-saphyr` はキー順保持だがコメント非保持）。`lossless` は「値の等価」を意味し、整形の完全一致ではない。TOML については `toml_edit` がコメント・順序を保持する。
- 双方向だが**完全可逆ではない**（lossy/dropped がある）。可逆性は `lossless` エントリに限る。
- **`serde-saphyr` の API 不安定性**（0.0.x）を許容する。YAML 書き込み仕様変更時は当該クレートの追従が必要。

---

## 15.5 標準規格ハブへの拡張点

### IR の二層構造

IR は **「標準コア」＋「ツール固有拡張」の二層**に位置づける。

| 層 | 内容 | 例 |
|---|---|---|
| **標準コア** | ツール横断の最小公約数 | `name`/`description`（agentskills.io 準拠）、AGENTS.md（18+ ツール対応のオープン標準） |
| **ツール固有拡張** | 各ツールのリッチ機能 | Claude: `allowed-tools`/`user-invocable`/`hooks` 等。Codex: `policy`/`agents/*.toml` 等 |

標準コアは agentskills.io の `name`/`description` を最小コアにとどめる。リッチ機能は各ツール拡張として保持し、変換時に降格・dropped として扱う（最小公倍数に引きずられないため、最初から N ツール汎用の設計は狙わない）。

### ハブ＆スポーク構造

当面は **Claude ⇄ Codex** の 1 対 1 に集中するが、IR を `各ツール ⇄ 標準 IR` の**ハブ＆スポーク**構造にしておくことで、将来ハンドラを追加するだけで N ツール対応に拡張できる。

```
Cursor ─────┐
Claude ──────┼──▶  標準 IR（標準コア + ツール固有拡張フィールド）  ◀──── Gemini CLI
Codex ───────┤
Zed ─────────┘
             ↑
       新ハンドラを追加するだけで
       CLI 本体の変更なく追従
```

拡張点の設計原則:
1. **ハンドラ追加で拡張**: 新ツール対応は `handlers/<tool>.rs` の追加と mappings YAML の追記のみ（エンジン本体は変更しない）。
2. **AGENTS.md をメモリハブ**: メモリファイルは `AGENTS.md`（OpenAI Agent Protocol 準拠、18+ ツール対応）を標準ハブとして積極的に使う。Claude の `CLAUDE.md` は AGENTS.md への降格・参照変換で扱う。
3. **標準コアへの収束を強制しない**: 各ツール拡張フィールドを標準化に合わせて削ぎ落とさず、ツール固有拡張として残したまま相手側で dropped にする。標準化は自然発生に任せる。

v1 では Claude⇄Codex のみ実装し、ハブ化は「IR に標準コアフィールドを明示的に保持する」「ハンドラを `Handler` トレイトに統一する」の2点を満たせば十分な拡張点が残る。

---

## 16. 主要モジュールのスケルトン

```rust
// src/main.rs
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();  // clap derive
    match cli.command {
        Commands::C2x { path, opts } => run(ConvDir::C2x, &path, &opts),
        Commands::X2c { path, opts } => run(ConvDir::X2c, &path, &opts),
        Commands::Check { path }     => check(&path),
    }
}

// src/cli.rs（clap derive）
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ccx")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    C2x { path: String, #[command(flatten)] opts: ConvertOpts },
    X2c { path: String, #[command(flatten)] opts: ConvertOpts },
    Check { path: String },
}

#[derive(Parser)]
pub struct ConvertOpts {
    #[arg(long)] pub out: Option<String>,
    #[arg(long)] pub scope: Option<String>,
    #[arg(long)] pub hooks_target: Option<String>,
    #[arg(long)] pub rewrite_body: bool,
    #[arg(long)] pub dual_manifest: bool,
    #[arg(long)] pub report: Option<Option<String>>, // --report[=json]
    #[arg(long)] pub dry_run: bool,
    #[arg(long)] pub strict: bool,
    #[arg(long)] pub force: bool,
    #[arg(long)] pub skill_target: Option<String>,   // §7.2.1: auto|skill|subagent（既定 auto）
    #[arg(long)] pub interactive: bool,              // §7.2.1: グレーケースを TTY 対話で確認
    #[arg(long)] pub only: Vec<String>,              // 変換対象ドメインをカンマ区切りで限定（例: skills,mcp）
    #[arg(long)] pub keep_claude_frontmatter: bool,  // Claude 固有 frontmatter キーを出力に残置（Codex は無視するだけなので無害）
}

/// 各ハンドラは対応する `DomainMap` を保持する（maps→handler 注入）。
/// `DomainMap` は `load_mappings` が返す `HashMap<String, DomainMap>` から domain 名で引く。
/// 例: `SkillsHandler { map: maps["skills"].clone() }`
///
/// `pick_handler` は `Kind`（detect 結果）と全 domain map を受け取り、
/// 対応するハンドラをボックス化して返す。
///
/// ```rust
/// pub fn pick_handler(kind: &Kind, maps: &HashMap<String, DomainMap>) -> Box<dyn Handler> {
///     match kind {
///         Kind::Skill   => Box::new(SkillsHandler  { map: maps["skills"].clone()  }),
///         Kind::Mcp     => Box::new(McpHandler      { map: maps["mcp"].clone()     }),
///         Kind::Hooks   => Box::new(HooksHandler    { map: maps["hooks"].clone()   }),
///         Kind::Plugin  => Box::new(PluginsHandler  { map: maps["plugins"].clone() }),
///         Kind::Memory  => Box::new(MemoryHandler   { map: maps["memory"].clone()  }),
///         Kind::Subagent => Box::new(SubagentHandler { map: maps["subagent"].clone() }),
///         _ => unimplemented!("handler for {kind:?}"),
///     }
/// }
/// ```
fn run(dir: ConvDir, path: &str, opts: &ConvertOpts) -> anyhow::Result<()> {
    let maps    = load_mappings(MAPPINGS_DIR);           // §6.1（+ 不変条件 assert）
    let kind    = detect(path);                          // §7.1
    let handler = pick_handler(&kind, &maps);            // maps を handler に注入
    let parsed  = handler.parse(path.as_ref())?;
    let ir      = handler.lift(&parsed, dir)?;           // mappings 駆動 + 本文スキャン + 降格
    let lower_opts = LowerOpts {                         // §7 で定義した型
        out: opts.out.clone(),
        scope: opts.scope.as_deref().map(parse_scope).unwrap_or(Scope::Project),
        dual_manifest: opts.dual_manifest,
        hooks_target: opts.hooks_target.as_deref().map(parse_scope).unwrap_or(Scope::User),
        skill_target: opts.skill_target.as_deref().map(parse_skill_target_mode).unwrap_or(SkillTargetMode::Auto),
        interactive: opts.interactive,
    };
    let plan    = handler.lower(&ir, dir, &lower_opts)?; // §7 各 lower（opts で出力パス解決）
    let report  = build_report(&ir, &plan);              // §10（Report 型: §10 参照）
    if !opts.dry_run { write_plan(&plan, opts)?; }       // §12 上書き保護
    print_report(&report, opts.report.as_deref().flatten());
    // report.dropped は Vec<DiagEntry>（§10 参照）
    let exit_code = if opts.strict && !report.dropped.is_empty() { 2 } else { 0 };
    std::process::exit(exit_code);
}

// handlers/skills.rs（lift の骨子）
// SkillsHandler は DomainMap を保持する（maps→handler 注入経路は §16 冒頭参照）
pub struct SkillsHandler { pub map: DomainMap }

impl Handler for SkillsHandler {
    fn lift(&self, parsed: &Value, dir: ConvDir) -> anyhow::Result<IRNode> {
        let mut node = new_node(Kind::Skill, dir, &parsed["path"].as_str().unwrap_or(""));
        let idx = match dir {
            ConvDir::C2x => index_by_claude_field(&self.map),
            ConvDir::X2c => index_by_codex_field(&self.map),
        };
        let frontmatter = parsed["frontmatter"].as_object().unwrap();
        for (key, value) in frontmatter {
            let Some(entry) = idx.get(key.as_str()) else {
                node.diagnostics.push(Diagnostic {
                    level: DiagLevel::Drop,
                    id: None,
                    message: format!("unknown frontmatter: {key}"),
                });
                continue;
            };
            if !applies_direction(entry, dir) { continue; }  // MappingDirection vs ConvDir 照合
            let (v, applied) = apply_transforms(value, entry.transform.as_deref(), &ctx(dir, entry));
            // ctx(dir, entry) は TransformCtx { direction: dir, args: None, field: entry } を返す。
            // apply_transforms が enum_map 等の args は TransformSpec.args から TransformCtx.args に注入する。
            node.fields.insert(entry.id.clone(), to_field(entry, v, applied));
            if entry.degrade.is_some() { run_degrade(entry, value, &mut node); }
            // NOTE: degrade truthy 時のみ run_degrade を呼ぶが、openai.yaml 系
            // (disable-model-invocation 等) と本文降格は handler が別途処理する（§7.2 参照）。
            if entry.warn == Some(true) { node.diagnostics.push(warn_for(entry)); }
        }
        let body_raw = parsed["body"].as_str().unwrap_or("").to_string();
        node.body = Some(BodySegment {
            findings: scan_body(&body_raw, dir),  // §9
            raw: body_raw,
        });
        Ok(node)
    }
}

// core/ir.rs に配置（cli.rs への依存を断ち切るため handlers/mod.rs の LowerOpts を参照）
// degrade/ 配下は cli.rs に依存せず LowerOpts のみに依存する。

/// skill→skill か skill→subagent かを決定した結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillTarget { Skill, Subagent }

/// IR ノードと `LowerOpts`（`SkillTargetMode` + `interactive` を含む）から変換先を決定する。
/// 明示オプション → 決定的ケース自動判定 → グレーケース（対話 or 保守的デフォルト）の順で解決。
/// `cli.rs` の `ConvertOpts` には依存しない（循環依存回避）。
pub fn decide_skill_target(ir: &IRNode, opts: &LowerOpts) -> SkillTarget {
    // ① 明示オプション（LowerOpts.skill_target が Auto 以外なら従う）
    match opts.skill_target {
        SkillTargetMode::Skill    => return SkillTarget::Skill,
        SkillTargetMode::Subagent => return SkillTarget::Subagent,
        SkillTargetMode::Auto     => {}
    }
    // ② 決定的ケース（model/effort/context:fork あり → subagent）
    let has_model   = ir.fields.contains_key("skill.model");
    let has_effort  = ir.fields.contains_key("skill.effort");
    let has_fork    = ir.fields.get("skill.context").map_or(false, |f| f.value == Value::String("fork".into()));
    if has_model || has_effort || has_fork {
        return SkillTarget::Subagent;
    }
    let has_perms = ir.fields.contains_key("skill.allowed-tools") || ir.fields.contains_key("skill.disallowed-tools");
    if !has_perms {
        return SkillTarget::Skill; // 純粋指示 → skill
    }
    // ③ グレーケース（権限あるが session 降格で許容できるか不明）
    if opts.interactive {
        // TTY 対話（dialoguer の Select/Confirm を利用）
        ask_user_skill_target(ir)
    } else {
        // 保守的デフォルト: 権限あり → subagent（失わない方）
        SkillTarget::Subagent
    }
}

// config.toml 非破壊マージ例（toml_edit）
// ※ toml_edit の `Item` には `.or_insert()` / `.is_none()` は存在しない。
//   正しくは `doc.entry(key).or_insert(...)` / `tbl.entry(key).or_insert(...)` を使い、
//   存在判定は `item.as_table().is_none()` 等の型変換メソッドで行う。
//   以下は正しい API を示す概念コード（実際の API は toml_edit 0.22.x のドキュメントを参照）。
use toml_edit::{DocumentMut, Item, Table, value};

fn upsert_agent_config(
    config_path: &std::path::Path,
    skill_name: &str,
    agents_toml_path: &str,
) -> anyhow::Result<()> {
    let src = std::fs::read_to_string(config_path).unwrap_or_default();
    let mut doc: DocumentMut = src.parse()?;

    // [agents.<skill>] を非破壊追記
    // entry() は DocumentMut / Table が提供する占有 API。
    let agents_item = doc.entry("agents").or_insert(Item::Table(Table::new()));
    let agents_tbl = agents_item.as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("[agents] is not a table"))?;
    let skill_item = agents_tbl.entry(skill_name).or_insert(Item::Table(Table::new()));
    let skill_tbl = skill_item.as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("[agents.{skill_name}] is not a table"))?;
    // 既存キーへの上書きは禁止（as_str() で存在確認）
    if skill_tbl.get("config_file").and_then(|v| v.as_str()).is_none() {
        skill_tbl.insert("config_file", value(agents_toml_path));
    }

    // [features] multi_agent = true を非破壊追記
    let features_item = doc.entry("features").or_insert(Item::Table(Table::new()));
    let features_tbl = features_item.as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("[features] is not a table"))?;
    if features_tbl.get("multi_agent").is_none() {
        features_tbl.insert("multi_agent", value(true));
    } else {
        // 既存キーへの上書きは禁止: warn を出してスキップ
        eprintln!("warn: [features].multi_agent already set, skipping");
    }

    std::fs::write(config_path, doc.to_string())?;  // コメント・順序保持
    Ok(())
}
```

---

## 付録: 設計の要点（なぜこの形か）

- **mappings 駆動**にすることで、仕様が流動的な Codex 側の変化に「YAML 編集」で追従できる（コード改修を最小化）。
- **降格を SideArtifact として明示**することで、「skill スコープ → session/subagent」という意味変化を report で必ず可視化できる。
- **Plugins を統合点**に置くことで、skills/hooks/mcp の変換器を1つに束ねられる（コードの DRY と、plugin 丸ごと変換の両立）。
- **本文スキャナを既定 read-only**にすることで、誤検出による本文破壊を避けつつ、危険箇所（`!`cmd``）を確実に可視化する。
- **`toml_edit` による非破壊マージ**で、`config.toml` のコメント・書式を壊さずに設定を追記できる。自前エミッタが不要になり、codex-rs の実証済みパターンをそのまま流用できる。
