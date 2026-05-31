// 実装は docs/12 §8.3 参照
// skill hooks → session/project hooks への降格エンジン。
// skill frontmatter の hooks を session/project の [hooks.*] に移送する。

use crate::core::ir::{DiagLevel, Diagnostic, SideArtifact};
use crate::handlers::Scope;

/// skill frontmatter の hooks を session/project hooks に移送する。
///
/// diagnostic: 「skill スコープではなくなる（session/project 全体に拡大）」warn を必ず出す。
///
/// # 書き出し先（LowerOpts.hooks_target で決定）
/// - Scope::User → ~/.codex/hooks.json
/// - Scope::Project → .codex/config.toml の [hooks] セクション（toml_edit で非破壊追記）
///
/// # plugin 同梱 hooks の注意（§7.3 #16430）
/// plugin root の hooks は Codex が読まないため、
/// --hooks-target=user|project で書き出す降格を既定にする。
pub fn degrade_skill_hooks(
    skill_name: &str,
    hooks_value: &serde_json::Value,
    hooks_target: &Scope,
) -> (Vec<SideArtifact>, Vec<Diagnostic>) {
    let mut artifacts = Vec::new();
    let mut diagnostics = Vec::new();

    let (target_path, target_desc) = match hooks_target {
        Scope::User => (
            "~/.codex/hooks.json".to_string(),
            "user scope (~/.codex/hooks.json)".to_string(),
        ),
        Scope::Project => (
            ".codex/config.toml".to_string(),
            "project scope (.codex/config.toml [hooks])".to_string(),
        ),
    };

    // hooks の JSON 表現を生成
    let hooks_content =
        serde_json::to_string_pretty(hooks_value).unwrap_or_else(|_| "{}".to_string());

    artifacts.push(SideArtifact {
        path: target_path.clone(),
        content: hooks_content,
        note: format!(
            "Hooks from skill '{}' degraded to {}",
            skill_name, target_desc
        ),
    });

    // スコープ拡大の警告
    diagnostics.push(Diagnostic {
        level: DiagLevel::Warn,
        id: Some("skills.hooks".to_string()),
        message: format!(
            "skill '{}' の hooks を {} に移送しました。\
             skill スコープ（skill 実行中のみ）から {} への拡大が発生します。\
             #16430: plugin 同梱 hooks は Codex が読まないため、--hooks-target で書き出し先を指定してください。",
            skill_name, target_desc, target_desc
        ),
    });

    (artifacts, diagnostics)
}
