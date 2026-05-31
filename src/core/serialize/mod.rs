// 実装は docs/12 §3 参照
// frontmatter/json の薄いラッパを提供する。
// TOML の読み書き・非破壊マージは toml_edit::DocumentMut を直接使用する（自前エミッタは不要）。

pub mod frontmatter;
pub mod json;
