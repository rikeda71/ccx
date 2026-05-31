---
name: run-checks
description: >
  Rust 実装を変更したとき、またはコミット前に fmt・clippy・test を回す。
  「チェックして」「cargo チェック」「run checks」「CI を通して」と言われた場合に使用する。
allowed-tools:
  - Bash(cargo *)
---

## 手順

以下を順番に実行する。前のステップが失敗したら修正してから次に進む。

### 1. フォーマットチェック

```bash
cargo fmt --check
```

失敗した場合は `cargo fmt` を実行してから再確認する。

### 2. Clippy（警告をエラーとして扱う）

```bash
cargo clippy -- -D warnings
```

失敗した場合は各 clippy 指摘を修正する。
`#[allow(...)]` で抑制する場合は理由コメントを必ず添える。

### 3. テスト

```bash
cargo test
```

失敗したテストを修正する。スナップショット（`insta`）が更新された場合は `cargo insta review` で差分を確認してから受理する。

### 完了条件

3 ステップすべてが 0 exit で通れば完了を報告する。
