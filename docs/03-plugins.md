<!--
docs/03-plugins.md — Plugins 領域ドキュメント
Generated: 2026-05-30
Template: docs/_TEMPLATE.md
Mapping: mappings/plugins.yaml
-->

# Plugins: Claude Code ⇄ Codex

> プラグイン領域は両者で概念は対応するが、マニフェスト構造・マーケットプレイス設計・ソース型・UIメタデータの対称性が低い。Claude 固有の `lspServers`/`outputStyles`/`experimental.*`/`channels`/`userConfig` 等は Codex に受け皿がなく破棄。Codex 固有の `interface.*`（ブランドカラー・アイコン・スクリーンショット等）・`apps`/`.app.json` コネクタ・marketplace `policy` は Claude に受け皿がない。マニフェスト配置ディレクトリは両者とも `.claude-plugin/plugin.json` (Claude) ⇔ `.codex-plugin/plugin.json` (Codex) でパス付け替えのみ。

## 0. 概要

Claude Code と Codex CLI はいずれも「プラグイン」という概念を持ち、`plugin.json` マニフェストとマーケットプレイス (`marketplace.json`) によってスキル・エージェント・フック・MCP サーバーをバンドル配布する仕組みを備えている。

主な対応関係:
- マニフェスト配置: `.claude-plugin/plugin.json` ⇔ `.codex-plugin/plugin.json`（パス付け替えのみ）
- `name`/`version`/`description`/`author`/`homepage`/`repository`/`license`/`keywords` は lossless で双方向変換可
- Claude `displayName` ⇔ Codex `interface.displayName`（フィールド名 rename）
- `skills`/`mcpServers`/`hooks` は両者に存在するが、指すファイル構造に差異あり

変換の難所:
- Claude 固有フィールド（`lspServers`, `outputStyles`, `experimental.themes`, `experimental.monitors`, `channels`, `userConfig`, `settings`, `defaultEnabled`, `bin/`）は Codex に対応なし → **dropped**
- Codex 固有フィールド（`interface.brandColor`, `interface.composerIcon`, `interface.logo`, `interface.capabilities`, `interface.screenshots`, `interface.websiteURL`, `interface.privacyPolicyURL`, `interface.termsOfServiceURL`, `interface.defaultPrompt`, `apps`）は Claude に対応なし → **dropped**
- マーケットプレイスの `policy`（Codex 固有）は Claude に受け皿なし → Claude→Codex では手動追加、Codex→Claude では破棄
- 両対応プラグインは `.claude-plugin/plugin.json`（Claude 用）と `.codex-plugin/plugin.json`（Codex 用）を別々に持つ**デュアルマニフェスト戦略**が必要（後述）

---

## 1. Claude Code 側の仕様

### 配置・ファイル・スコープ

| ファイル | パス | スコープ |
|---|---|---|
| プラグインマニフェスト | `<plugin-root>/.claude-plugin/plugin.json` | plugin |
| マーケットプレイスマニフェスト | `<marketplace-root>/.claude-plugin/marketplace.json` | marketplace |
| MCP サーバー設定 | `<plugin-root>/.mcp.json` または `plugin.json` 内インライン | plugin |
| LSP サーバー設定 | `<plugin-root>/.lsp.json` または `plugin.json` 内インライン | plugin |
| フック設定 | `<plugin-root>/hooks/hooks.json` または `plugin.json` 内インライン | plugin |
| モニター設定 | `<plugin-root>/monitors/monitors.json` または `experimental.monitors` | plugin |
| スキル | `<plugin-root>/skills/<name>/SKILL.md` | plugin |
| コマンド | `<plugin-root>/commands/<name>.md` | plugin |
| エージェント | `<plugin-root>/agents/<name>.md` | plugin |
| 実行ファイル | `<plugin-root>/bin/` | plugin |
| テーマ | `<plugin-root>/themes/<name>.json` | plugin |

インストールスコープ:

| スコープ | 設定ファイル | 用途 |
|---|---|---|
| `user` | `~/.claude/settings.json` | 全プロジェクト共通（デフォルト） |
| `project` | `.claude/settings.json` | チーム共有（バージョン管理） |
| `local` | `.claude/settings.local.json` | ローカル限定（gitignore） |
| `managed` | managed-settings.json | 組織強制（読み取り専用） |

### plugin.json 全フィールド表

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `$schema` | string | No | — | SchemaStore URL。ロード時は無視（エディタ補完用） |
| `name` | string | Yes | — | 一意識別子（kebab-case、スペース不可）。コンポーネントの名前空間に使用 |
| `displayName` | string | No | `name` | UI 表示名。スペース・大小文字自由（v2.1.143+） |
| `version` | string | No | git SHA | semver。省略時は git commit SHA を使用 |
| `description` | string | No | — | プラグインの概要説明 |
| `author` | object | No | — | 作者情報（`name` 必須、`email`・`url` 任意） |
| `author.name` | string | Yes (in obj) | — | 作者・組織名 |
| `author.email` | string | No | — | 問い合わせ先メール |
| `author.url` | string | No | — | ウェブサイト・GitHub プロフィール等 |
| `homepage` | string (URI) | No | — | ドキュメント等のホームページ URL |
| `repository` | string | No | — | ソースコードリポジトリ URL |
| `license` | string | No | — | SPDX ライセンス識別子（例: `MIT`, `Apache-2.0`） |
| `keywords` | array[string] | No | — | 検索・発見用タグ |
| `defaultEnabled` | boolean | No | `true` | インストール後の初期有効状態（v2.1.154+） |
| `skills` | string\|array | No | `./skills/` | スキルディレクトリパス（デフォルトに追加） |
| `commands` | string\|array | No | `./commands/` | フラット .md スキルファイルパス（デフォルト置換） |
| `agents` | string\|array | No | `./agents/` | エージェントファイルパス（デフォルト置換） |
| `hooks` | string\|array\|object | No | `./hooks/hooks.json` | フック設定パスまたはインライン定義 |
| `mcpServers` | string\|array\|object | No | `./.mcp.json` | MCP サーバー設定パスまたはインライン定義 |
| `lspServers` | string\|array\|object | No | `./.lsp.json` | LSP サーバー設定パスまたはインライン定義 |
| `outputStyles` | string\|array | No | `./output-styles/` | 出力スタイル定義パス（デフォルト置換） |
| `experimental.themes` | string\|array | No | `./themes/` | カラーテーマ定義パス（デフォルト置換） |
| `experimental.monitors` | string\|array | No | `./monitors/monitors.json` | バックグラウンドモニター設定パス |
| `userConfig` | object | No | — | 有効化時にユーザーへ入力を求める値の定義 |
| `channels` | array | No | — | MCP サーバーにバインドするメッセージチャネル定義 |
| `dependencies` | array | No | — | 依存プラグイン（文字列または `{name, version}` オブジェクト） |
| `settings` | object | No | — | プラグイン有効時にマージされる設定値 |

#### userConfig フィールド（各キーのプロパティ）

| プロパティ | 型 | 必須 | 説明 |
|---|---|---|---|
| `type` | string | Yes | `string`/`number`/`boolean`/`directory`/`file` |
| `title` | string | Yes | 設定ダイアログのラベル |
| `description` | string | Yes | フィールドの説明テキスト |
| `sensitive` | boolean | No | `true` の場合、入力をマスクしシステムキーチェーンに保存 |
| `required` | boolean | No | `true` の場合、空値でバリデーション失敗 |
| `default` | any | No | ユーザー未入力時のデフォルト値 |
| `multiple` | boolean | No | `string` 型で配列入力を許可 |
| `min` / `max` | number | No | `number` 型の値範囲 |

#### channels エントリのフィールド

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `server` | string | Yes | バインドする MCP サーバーキー名 |
| `displayName` | string | No | チャネルの表示名 |
| `userConfig` | object | No | チャネル固有の userConfig（トップレベルと同スキーマ） |

### marketplace.json 全フィールド表（Claude）

ファイルパス: `<marketplace-root>/.claude-plugin/marketplace.json`

#### トップレベルフィールド

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `$schema` | string | No | SchemaStore URL（エディタ補完用） |
| `name` | string | Yes | マーケットプレイス識別子（kebab-case） |
| `version` | string | No | マーケットプレイスマニフェストのバージョン |
| `description` | string | No | マーケットプレイスの説明 |
| `owner` | object | Yes | 管理者情報（`name` 必須） |
| `owner.name` | string | Yes | 管理者・チーム名 |
| `owner.email` | string | No | 連絡先メール |
| `owner.url` | string | No | ウェブサイト |
| `plugins` | array | Yes | プラグインエントリの配列 |
| `metadata.pluginRoot` | string | No | 相対パスのベースディレクトリ（例: `"./plugins"` で各 source を短縮可） |
| `allowCrossMarketplaceDependenciesOn` | array | No | 依存可能な他マーケットプレイス名リスト |
| `forceRemoveDeletedPlugins` | boolean | No | マーケットプレイスから削除されたプラグインを自動アンインストール |

#### plugins[] エントリのフィールド

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `name` | string | Yes | プラグイン識別子（kebab-case） |
| `source` | string\|object | Yes | 取得元（下記「source タイプ別表」参照） |
| `displayName` | string | No | UI 表示名（v2.1.143+） |
| `version` | string | No | バージョン文字列（省略時は git SHA） |
| `description` | string | No | プラグインの説明 |
| `author` | object | No | 作者情報（`name` 必須） |
| `homepage` | string | No | ホームページ URL |
| `repository` | string | No | リポジトリ URL |
| `license` | string | No | SPDX ライセンス識別子 |
| `keywords` | array | No | 検索タグ |
| `category` | string | No | カテゴリ（UI 整理用） |
| `tags` | array | No | 追加タグ |
| `strict` | boolean | No（既定 `true`） | `plugin.json` を権威とするか。`false` でマーケットプレイスエントリが全定義 |
| `defaultEnabled` | boolean | No（既定 `true`） | インストール後の初期有効状態。`plugin.json` の同フィールドより優先（v2.1.154+） |
| `skills` / `commands` / `agents` / `hooks` / `mcpServers` / `lspServers` | mixed | No | コンポーネント設定のオーバーライド |

### source タイプ別表（Claude marketplace）

| タイプ | 指定方法 | 必須フィールド | 任意フィールド | 備考 |
|---|---|---|---|---|
| 相対パス | `"./plugins/my-plugin"` (string) | — | — | マーケットプレイスルート相対。`./` 始まり必須 |
| `github` | `{"source":"github","repo":"owner/repo"}` | `repo` | `ref`, `sha` | `sha` は 40 文字 hex |
| `url` | `{"source":"url","url":"https://..."}` | `url` | `ref`, `sha` | git URL（`.git` 拡張子任意） |
| `git-subdir` | `{"source":"git-subdir","url":"...","path":"..."}` | `url`, `path` | `ref`, `sha` | monorepo のサブディレクトリ。sparse clone |
| `npm` | `{"source":"npm","package":"@org/pkg"}` | `package` | `version`, `registry` | `npm install` で取得 |

---

## 2. Codex 側の仕様

### 配置・ファイル・スコープ

| ファイル | パス | スコープ |
|---|---|---|
| プラグインマニフェスト | `<plugin-root>/.codex-plugin/plugin.json` | plugin |
| マーケットプレイスマニフェスト | `<marketplace-root>/.agents/plugins/marketplace.json` | marketplace |
| （legacy-compatible marketplace）| `<marketplace-root>/.claude-plugin/marketplace.json` | marketplace |
| MCP サーバー設定 | `<plugin-root>/.mcp.json` | plugin |
| フック設定 | `<plugin-root>/hooks/hooks.json` | plugin |
| アプリコネクタ設定 | `<plugin-root>/.app.json` | plugin |
| スキル | `<plugin-root>/skills/` | plugin |

### plugin.json 全フィールド表（Codex）

| フィールド | 型 | 必須 | デフォルト | 説明 |
|---|---|---|---|---|
| `name` | string | Yes | — | プラグイン識別子（kebab-case、64 文字以内） |
| `version` | string | Yes | — | strict semver（例: `"1.0.0"`） |
| `description` | string | Yes | — | プラグインの概要 |
| `author` | object | No | — | 作者情報（`name` 必須、`email`・`url` 任意） |
| `author.name` | string | Yes (in obj) | — | 作者名 |
| `author.email` | string | No | — | 連絡先メール |
| `author.url` | string | No | — | URL |
| `homepage` | string | No | — | ホームページ URL |
| `repository` | string | No | — | リポジトリ URL |
| `license` | string | No | — | SPDX ライセンス識別子 |
| `keywords` | array[string] | No | — | 検索タグ |
| `skills` | string | No | `./skills/` | スキルディレクトリパス |
| `mcpServers` | string | No | `./.mcp.json` | MCP サーバー設定ファイルパス |
| `apps` | string | No | `./.app.json` | アプリコネクタ設定ファイルパス |
| `hooks` | string\|array | No | `./hooks/hooks.json` | フック設定ファイルパス |
| `interface` | object | No | — | UI/プレゼンテーション設定（下記参照） |
| `interface.displayName` | string | No | `name` | UI 表示名 |
| `interface.shortDescription` | string | No | — | 短い説明文（マーケットプレイス一覧用） |
| `interface.longDescription` | string | No | — | 詳細説明 |
| `interface.developerName` | string | No | — | 開発者・組織名 |
| `interface.category` | string | No | — | カテゴリ（例: `"Productivity"`, `"Developer Tools"`） |
| `interface.capabilities` | array[string] | No | — | `Interactive` / `Read` / `Write` の組み合わせ |
| `interface.websiteURL` | string | No | — | ウェブサイト URL |
| `interface.privacyPolicyURL` | string | No | — | プライバシーポリシー URL |
| `interface.termsOfServiceURL` | string | No | — | 利用規約 URL |
| `interface.defaultPrompt` | string\|array | No | — | スターター提案プロンプト（array の場合 ≤3 件、各 ≤128 文字） |
| `interface.brandColor` | string | No | — | ブランドカラー（16 進数 hex、例: `"#EA4335"`） |
| `interface.composerIcon` | string | No | — | コンポーザーアイコン画像パス（SVG 推奨） |
| `interface.logo` | string | No | — | ロゴ画像パス |
| `interface.screenshots` | array | No | — | スクリーンショット画像パスの配列 |

### marketplace.json 全フィールド表（Codex）

ファイルパス: `<marketplace-root>/.agents/plugins/marketplace.json`  
（`<marketplace-root>/.claude-plugin/marketplace.json` も互換読み込みされる）

#### トップレベルフィールド

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `name` | string | Yes | マーケットプレイス識別子 |
| `interface.displayName` | string | No | UI 表示名 |
| `plugins` | array | Yes | プラグインエントリの配列 |

#### plugins[] エントリのフィールド

| フィールド | 型 | 必須 | 説明 |
|---|---|---|---|
| `name` | string | Yes | プラグイン識別子 |
| `source` | object | Yes | 取得元（下記参照） |
| `policy` | object | Yes | インストール・認証ポリシー（Codex 固有） |
| `policy.installation` | string | Yes | `AVAILABLE` / `INSTALLED_BY_DEFAULT` / `NOT_AVAILABLE` |
| `policy.authentication` | string | No | `ON_INSTALL` / `ON_USE` |
| `policy.products` | array | No | 対象製品リスト |
| `category` | string | Yes | カテゴリ文字列 |

### source タイプ別表（Codex marketplace）

| タイプ | 指定方法 | 必須フィールド | 任意フィールド | 備考 |
|---|---|---|---|---|
| `local` | `{"source":"local","path":"./plugins/foo"}` | `path` | — | マーケットプレイスルート相対 |
| `git-subdir` | `{"source":"git-subdir","url":"...","path":"..."}` | `url`, `path` | `ref`, `sha` | monorepo のサブディレクトリ |
| `url` | `{"source":"url","url":"...","sha":"...","path":"..."}` | `url` | `sha`, `path` | git URL ソース |
| `github` | `{"source":"github","repo":"owner/repo","commit":"..."}` | `repo` | `commit`, `sha` | GitHub リポジトリ |

### .app.json

Codex 固有のアプリコネクタ設定ファイル。OAuth コネクタまたは Agent SDK アプリに対応する。

```json
{
  "apps": {
    "<service-key>": {
      "id": "<connector-id>"
    }
  }
}
```

| フィールド | 説明 |
|---|---|
| `apps.<key>` | サービスキー（例: `"github"`, `"gmail"`, `"slack"`） |
| `apps.<key>.id` | コネクタ ID。`connector_*` 形式（OAuth コネクタ）または `asdk_app_*` 形式（Agent SDK アプリ）|

Claude の `channels` フィールドと概念的に近いが（外部サービスへの接続を宣言）、スキーマは別物。相互変換不可（dropped）。

---

## 3. 変換テーブル

`mappings/plugins.yaml` の人間可読版。

| id | Claude フィールド | Codex フィールド | 方向 | 損失 | 降格/スコープ | 書式変換・注記 |
|---|---|---|---|---|---|---|
| `plugins.manifest-path` | `.claude-plugin/plugin.json` | `.codex-plugin/plugin.json` | both | ◎ | — | `path:remap`。ファイル名 `plugin.json` は共通 |
| `plugins.name` | `name` | `name` | both | ◎ | — | 両者必須。Codex は kebab-case ≤64 文字の制約あり。Claude 側も同様の制約を推奨 |
| `plugins.version` | `version`（省略時 git SHA） | `version`（strict semver 必須） | both | ○ | — | Codex は semver 必須。Claude→Codex で省略時は git SHA を semver 形式で補完要（`warn`） |
| `plugins.description` | `description` | `description` | both | ◎ | — | 両者任意（Codex は必須）。lossless |
| `plugins.author` | `author.{name,email,url}` | `author.{name,email,url}` | both | ◎ | — | 構造が同一。lossless |
| `plugins.homepage` | `homepage` | `homepage` | both | ◎ | — | lossless |
| `plugins.repository` | `repository` | `repository` | both | ◎ | — | lossless |
| `plugins.license` | `license` | `license` | both | ◎ | — | lossless |
| `plugins.keywords` | `keywords` | `keywords` | both | ◎ | — | lossless |
| `plugins.display-name` | `displayName` | `interface.displayName` | both | ○ | — | `rename`。キー名のみ変更 |
| `plugins.short-description` | `description`（転用） | `interface.shortDescription` | codex_to_claude | △ | — | Codex→Claude は `description` へマージ。Claude→Codex は `description` を `interface.shortDescription` へコピー（lossy）|
| `plugins.skills` | `skills`（path） | `skills`（path） | both | ○ | — | `path:remap`。デフォルト `./skills/` は共通 |
| `plugins.mcpServers` | `mcpServers`（path/obj） | `mcpServers`（path → `.mcp.json`） | both | ○ | — | Claude はインラインまたはパス。Codex はパス参照（`.mcp.json`）。インライン→ファイル書き出しが必要な場合あり |
| `plugins.hooks` | `hooks`（path/obj） | `hooks`（path） | both | △ | — | hook タイプ・イベント名のマッピングはスキル領域と同様。`path:remap` + イベント名変換 |
| `plugins.commands` | `commands`（path） | null | claude_to_codex | ✕ | — | Codex にフラット `.md` コマンド概念なし。`skills/` として扱う近似が可能だが異なる（warn） |
| `plugins.agents` | `agents`（path） | null | claude_to_codex | △ | — | Codex には `agents/` ディレクトリが存在するが構造が異なる可能性あり（要検証） |
| `plugins.lspServers` | `lspServers` | null | claude_to_codex | ✕ | — | Codex に LSP サポートなし。**dropped** |
| `plugins.outputStyles` | `outputStyles` | null | claude_to_codex | ✕ | — | Codex に出力スタイル機能なし。**dropped** |
| `plugins.experimental.themes` | `experimental.themes` | null | claude_to_codex | ✕ | — | Codex にテーマ機能なし。**dropped** |
| `plugins.experimental.monitors` | `experimental.monitors` | null | claude_to_codex | ✕ | — | Codex にモニター機能なし。**dropped** |
| `plugins.userConfig` | `userConfig` | null | claude_to_codex | ✕ | — | Codex にユーザー設定プロンプト機能なし。**dropped**（warn） |
| `plugins.channels` | `channels` | null | claude_to_codex | ✕ | — | `.app.json` と概念は近いがスキーマが異なり変換不可。**dropped**（warn） |
| `plugins.settings` | `settings` | null | claude_to_codex | ✕ | — | Codex に plugin-level settings マージ機能なし。**dropped** |
| `plugins.defaultEnabled` | `defaultEnabled` | null | claude_to_codex | ✕ | — | Codex 側の `policy.installation: INSTALLED_BY_DEFAULT` で部分的に近似可能だが marketplace entry の手動設定が必要（warn） |
| `plugins.dependencies` | `dependencies` | null | claude_to_codex | ✕ | — | Codex にプラグイン依存解決機能なし。**dropped**（warn） |
| `plugins.interface.brandColor` | null | `interface.brandColor` | codex_to_claude | ✕ | — | Claude にブランドカラー概念なし。**dropped** |
| `plugins.interface.composerIcon` | null | `interface.composerIcon` | codex_to_claude | ✕ | — | Claude に Composer アイコン概念なし。**dropped** |
| `plugins.interface.logo` | null | `interface.logo` | codex_to_claude | ✕ | — | Claude にロゴ概念なし。**dropped** |
| `plugins.interface.capabilities` | null | `interface.capabilities` | codex_to_claude | ✕ | — | Claude に capabilities 概念なし。**dropped** |
| `plugins.interface.screenshots` | null | `interface.screenshots` | codex_to_claude | ✕ | — | Claude にスクリーンショット概念なし。**dropped** |
| `plugins.interface.websiteURL` | null | `interface.websiteURL` | codex_to_claude | △ | — | `homepage` に近似可能だが重複の場合は warn |
| `plugins.interface.privacyPolicyURL` | null | `interface.privacyPolicyURL` | codex_to_claude | ✕ | — | Claude に受け皿なし。**dropped** |
| `plugins.interface.termsOfServiceURL` | null | `interface.termsOfServiceURL` | codex_to_claude | ✕ | — | Claude に受け皿なし。**dropped** |
| `plugins.interface.defaultPrompt` | null | `interface.defaultPrompt` | codex_to_claude | △ | — | Claude に直接対応なし。スキル本文への注記として落とす近似が可能 |
| `plugins.apps` | null | `apps`（`.app.json`） | codex_to_claude | ✕ | — | Claude の `channels` と概念が近いがスキーマが異なり変換不可。**dropped**（warn） |
| `plugins.marketplace.path` | `.claude-plugin/marketplace.json` | `.agents/plugins/marketplace.json` | both | ○ | — | `path:remap`。Codex は `.claude-plugin/` も互換読み込みするため、Claude 側マーケットプレイスはそのまま配置可能 |
| `plugins.marketplace.name` | `name` | `name` | both | ◎ | — | lossless |
| `plugins.marketplace.owner` | `owner.{name,email,url}` | null | claude_to_codex | ✕ | — | Codex marketplace に `owner` フィールドなし。**dropped** |
| `plugins.marketplace.plugins.source` | source（string/object） | source（object） | both | △ | — | `path:remap` + source スキーマ正規化。Claude の `relative`, `github`, `url`, `git-subdir`, `npm` ⇔ Codex の `local`, `github`, `url`, `git-subdir`。`npm` は Codex に対応なし → dropped（warn） |
| `plugins.marketplace.plugins.policy` | null | `policy.{installation,authentication,products}` | codex_to_claude | ✕ | — | Claude marketplace に policy 概念なし。**dropped** |
| `plugins.marketplace.allowCrossMarketplaceDependenciesOn` | `allowCrossMarketplaceDependenciesOn` | null | claude_to_codex | ✕ | — | Codex に対応なし。**dropped** |
| `plugins.marketplace.forceRemoveDeletedPlugins` | `forceRemoveDeletedPlugins` | null | claude_to_codex | ✕ | — | Codex に対応なし。**dropped** |
| `plugins.marketplace.metadata.pluginRoot` | `metadata.pluginRoot` | null | claude_to_codex | △ | — | Codex に直接対応なし。source パスで吸収可能だが情報は失われる |

---

## 4. 変換時の注意・既知の落とし穴

### マニフェストディレクトリの違い

`.claude-plugin/plugin.json` ⇔ `.codex-plugin/plugin.json` は単純なパス付け替えだが、**両対応プラグインはデュアルマニフェスト戦略が必要**（`.claude-plugin/plugin.json` と `.codex-plugin/plugin.json` を別々に用意）。

Codex のマーケットプレイスカタログ（`marketplace.json`）の発見パス優先順位は以下の通り（公式 `developers.openai.com/codex/plugins/build` 参照）:
1. curated 公式 Plugin Directory
2. `$REPO_ROOT/.agents/plugins/marketplace.json`（ネイティブパス）
3. `$REPO_ROOT/.claude-plugin/marketplace.json`（公式に **"legacy-compatible marketplace"** と呼称）
4. `~/.agents/plugins/marketplace.json`

これは**マーケットプレイスカタログ（`marketplace.json`）の発見**であって、配下の `.claude-plugin/plugin.json`（プラグインマニフェスト）を Codex がネイティブ解釈する意味ではない。Codex がネイティブに読むプラグインマニフェストは `.codex-plugin/plugin.json` のみ。

### Codex の `version` 必須制約

Claude では `version` は省略可能（省略時 git SHA）だが、Codex では strict semver が必須。Claude→Codex 変換時に `version` が省略されている場合は `"0.0.0"` や git SHA から semver への変換が必要。変換レポートに明記すること。

### `commands` の扱い

Claude では `skills/` と `commands/` を区別する（フラット `.md` ファイル）。Codex には `commands` 概念がなく `skills/` のみ。Claude `commands/` を Codex `skills/` として移植できるが、ディレクトリ構造の変換（SKILL.md ラッパーの追加）が必要な場合がある。

### dropped フィールドのまとめ

Claude→Codex で破棄されるフィールド（変換レポートに全件列挙すること）:
- `lspServers`, `outputStyles`, `experimental.themes`, `experimental.monitors`
- `userConfig`, `channels`, `settings`, `dependencies`
- `defaultEnabled`（`policy.installation` で手動近似を提案）
- marketplace: `owner`, `allowCrossMarketplaceDependenciesOn`, `forceRemoveDeletedPlugins`
- source type: `npm`（Codex marketplace に対応なし）

Codex→Claude で破棄されるフィールド:
- `interface.brandColor`, `interface.composerIcon`, `interface.logo`
- `interface.capabilities`, `interface.screenshots`
- `interface.privacyPolicyURL`, `interface.termsOfServiceURL`
- `apps`（`.app.json` 全体）
- marketplace: `policy`, `interface.displayName`（`name` に降格）

### .app.json と channels の非対称性

Codex の `.app.json` は OAuth コネクタ（`connector_*`）または Agent SDK アプリ（`asdk_app_*`）の ID を宣言するファイルで、認証済みの外部サービス接続を提供する。Claude の `channels` は MCP サーバーを会話チャネルとしてバインドするもので、概念が近いが設計が根本的に異なる。直接変換は不可。

### openai/codex-plugin-cc について

`openai/codex-plugin-cc` は **Claude Code 専用のプラグイン**（`.claude-plugin/plugin.json` を持つ）であり、「両対応ハイブリッドプラグイン」ではない。

このプラグインは **Claude Code セッションの中から Codex CLI をサブプロセス起動して呼び出す一方向ブリッジ**であり、`/codex:review` 等のコマンドと Stop hook によるレビューゲートを提供する。Codex の plugin marketplace からインストールするものではなく、Claude Code にインストールして使うものである。

「両対応の実例」として引用するのは誤り。両対応プラグインは `.claude-plugin/plugin.json`（Claude 用）と `.codex-plugin/plugin.json`（Codex 用）のデュアルマニフェスト戦略で実現する。

### 環境変数

Claude と Codex のプラグイン固有変数:

| 変数 | Claude | Codex | 説明 |
|---|---|---|---|
| `${CLAUDE_PLUGIN_ROOT}` | Yes | Yes（後方互換） | プラグインインストールディレクトリの絶対パス。Codex は hook 実行時に Claude 向け hook スクリプトのパス解決のため後方互換目的で設定 |
| `PLUGIN_ROOT` | No | Yes（ネイティブ） | Codex のネイティブ変数名。`CLAUDE_PLUGIN_ROOT` と同値 |
| `${CLAUDE_PLUGIN_DATA}` | Yes | Yes（後方互換） | 更新をまたいで永続化するデータディレクトリ。Codex は後方互換目的で設定 |
| `PLUGIN_DATA` | No | Yes（ネイティブ） | Codex のネイティブ変数名。`CLAUDE_PLUGIN_DATA` と同値 |
| `${CLAUDE_PROJECT_DIR}` | Yes | No | プロジェクトルートディレクトリ |
| `${user_config.KEY}` | Yes | No | userConfig で定義したユーザー入力値 |
| `CLAUDE_PLUGIN_OPTION_<KEY>` | Yes | No | userConfig 値の環境変数エクスポート |

### Codex が Claude plugin を読んだときのコンポーネント可否

Codex は `.codex-plugin/plugin.json` のみをネイティブに解釈する。Claude plugin マニフェスト（`.claude-plugin/plugin.json`）を Codex のマーケットプレイスカタログ経由で参照させた場合、各コンポーネントの扱いは以下の通り:

| コンポーネント | Codex での扱い | 備考 |
|---|---|---|
| `skills` | **有効**（部分） | ファイルは読み込まれるが frontmatter は `name`/`description` のみ使用（docs/02 参照） |
| `mcpServers`（`.mcp.json`） | **有効** | `.mcp.json` 形式は Codex もネイティブ対応 |
| `apps`（`.app.json`） | **有効** | Codex の `.app.json` コネクタとして読み込み可能 |
| `hooks` | **有効（要注意）** | ドキュメント上は有効だが Issue #16430 のバグで現状ロードされない可能性あり（docs/05 参照） |
| `commands` | **無視〜要変換（不確実）** | Codex に slash command 概念がなく、スキルへの変換が必要な場合あり |
| `agents` | **無視〜部分** | 形式差により完全には機能しない可能性あり |
| `lspServers` | **無視** | Codex に LSP サポートなし |
| `outputStyles` | **無視** | Codex 非対応 |
| `experimental.themes` | **無視** | Codex 非対応 |
| `experimental.monitors` | **無視** | Codex 非対応 |

なお Codex は hook 実行時に `CLAUDE_PLUGIN_ROOT` / `CLAUDE_PLUGIN_DATA` 環境変数を後方互換目的で設定する（Claude 向け hook スクリプトのパス解決のため）。

### CLI コマンド

| 操作 | Claude Code | Codex CLI |
|---|---|---|
| マーケットプレイス追加 | `claude plugin marketplace add <source>` | `codex plugin marketplace add <source>` |
| マーケットプレイス一覧 | `claude plugin marketplace list` | `codex plugin marketplace list` |
| マーケットプレイス削除 | `claude plugin marketplace remove <name>` | `codex plugin marketplace remove <name>` |
| マーケットプレイス更新 | `claude plugin marketplace update [name]` | `codex plugin marketplace upgrade [name]` |
| プラグインインストール | `claude plugin install <name>@<marketplace>` | （`/plugins` UI またはマーケットプレイス経由） |
| プラグイン有効化 | `claude plugin enable <name>` | — |
| プラグイン無効化 | `claude plugin disable <name>` | — |
| プラグイン削除 | `claude plugin uninstall <name>` | — |
| プラグイン一覧 | `claude plugin list` | — |
| バリデーション | `claude plugin validate [path]` | — |
| スキャフォールド | `claude plugin init <name>` | — |

---

## 5. 出典

- https://code.claude.com/docs/en/plugins-reference
- https://code.claude.com/docs/en/plugin-marketplaces
- https://www.schemastore.org/claude-code-plugin-manifest.json
- https://www.schemastore.org/claude-code-marketplace.json
- https://developers.openai.com/codex/plugins
- https://developers.openai.com/codex/plugins/build
- https://github.com/openai/plugins（`plugins/*/`. codex-plugin/plugin.json` 実例）
- https://github.com/openai/plugins/blob/main/.agents/plugins/marketplace.json
- https://github.com/openai/codex-plugin-cc（Claude Code 専用・Codex CLI 呼び出しブリッジの実例）
