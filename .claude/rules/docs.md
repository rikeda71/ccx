---
paths:
  - "docs/**"
---

# docs 編集ルール

`docs/` は設計の正本。特に `docs/12-cli-spec.md` は実装の唯一の権威。

## 設計変更時の整合ルール

- 設計を変更する場合は `docs/12-cli-spec.md` と関連する `mappings/*.yaml` を**両方**更新し整合を保つ
- `docs/13-feature-matrix.md` の変換可/不可/将来追従の分類は `mappings/*.yaml` の `loss` 分布（lossless / lossy / dropped の件数・内訳）と一致させる
- `docs/12` と他の `docs/` に矛盾が生じた場合は `docs/12` を優先する

## 参照ルール

- 実装のフロー・型・インタフェースに不明点がある場合は `docs/12` の該当章（§番号）を参照する
- Codex 側の挙動に不確実性がある場合は `docs/11-interop-claude-plugin-in-codex.md` と各エントリの `notes` を参照する
