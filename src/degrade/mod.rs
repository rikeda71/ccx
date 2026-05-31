// 実装は docs/12 §8 参照
// 降格エンジン: skill スコープが Codex に無い分を、別スコープのファイル生成で補う。
// 各降格は SideArtifact（生成ファイル）+ diagnostic（降格の記録）を返す。

pub mod hooks_scope;
pub mod rules;
pub mod subagent;
