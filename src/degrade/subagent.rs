// 実装は docs/12 §8.2 §7.2.1 §16 参照
// skill(model/effort/context:fork) → subagent への降格エンジン。
// .codex/agents/<skill>.toml を生成し、config.toml に [agents.*] / [features] を追記する。

use crate::core::ir::{DiagLevel, Diagnostic, IRNode, SideArtifact};
use crate::core::transforms::{claude_tier, tier_to_codex};
use crate::handlers::{LowerOpts, SkillTargetMode};

/// skill → skill か skill → subagent かの変換先を示す。
/// cli.rs の ConvertOpts には依存しない（循環依存回避）。
/// LowerOpts を介して参照する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillTarget {
    Skill,
    Subagent,
}

/// IR ノードと LowerOpts から変換先（SkillTarget）を決定する。
///
/// # 決定優先順位（§7.2.1 / §16 参照）
/// 1. 明示オプション（LowerOpts.skill_target が Auto 以外なら従う）
/// 2. 決定的ケースの自動判定:
///    - skill.model / skill.effort / skill.context==fork あり → Subagent
///    - 権限なし（純粋指示） → Skill
/// 3. グレーケース（権限あり・session 降格で許容できるか不明）:
///    - interactive あり → TTY 対話（dialoguer で確認）
///    - 非対話 → 保守的デフォルト（Subagent）、report に選択理由を必ず明記
pub fn decide_skill_target(ir: &IRNode, opts: &LowerOpts) -> SkillTarget {
    // ① 明示オプション
    match opts.skill_target {
        SkillTargetMode::Skill => return SkillTarget::Skill,
        SkillTargetMode::Subagent => return SkillTarget::Subagent,
        SkillTargetMode::Auto => {}
    }

    // ② 決定的ケース
    let has_model = ir.fields.contains_key("skills.model");
    let has_effort = ir.fields.contains_key("skills.effort");
    let has_fork = ir
        .fields
        .get("skills.context-fork")
        .is_some_and(|f| f.value == serde_json::Value::String("fork".into()));

    if has_model || has_effort || has_fork {
        return SkillTarget::Subagent;
    }

    let has_perms = ir.fields.contains_key("skills.allowed-tools")
        || ir.fields.contains_key("skills.disallowed-tools");
    if !has_perms {
        return SkillTarget::Skill; // 純粋指示 → skill
    }

    // ③ グレーケース（権限あり）
    if opts.interactive {
        ask_user_skill_target(ir)
    } else {
        // 保守的デフォルト: 権限あり → subagent（失わない方）
        SkillTarget::Subagent
    }
}

/// TTY 対話で変換先を確認する（dialoguer を使用）。
fn ask_user_skill_target(ir: &IRNode) -> SkillTarget {
    use dialoguer::Select;

    let skill_name = ir.source_path.as_str();
    let items = &[
        "skill (権限は session 降格・自動発火を維持)",
        "subagent (権限を subagent に束ねる・明示起動)",
    ];

    let selection = Select::new()
        .with_prompt(format!(
            "skill '{}' は allowed-tools を持ちます。変換先を選択してください",
            skill_name
        ))
        .items(items)
        .default(1) // 保守的デフォルト: subagent
        .interact();

    match selection {
        Ok(0) => SkillTarget::Skill,
        _ => SkillTarget::Subagent,
    }
}

/// skill(model/effort/context:fork) → .codex/agents/<skill>.toml の生成。
///
/// # 生成内容（§8.2 参照）
/// ```toml
/// name = "<skill>"
/// description = "<when_to_use or description>"
/// developer_instructions = "<skill 本文>"
/// model = "<tier_to_codex(claude_tier(model))>"
/// model_reasoning_effort = "xhigh"   # max→xhigh (enum_map)
/// ```
/// + config.toml への非破壊追記（toml_edit）:
/// ```toml
/// [agents.<skill>]
/// config_file = ".codex/agents/<skill>.toml"
/// [features]
/// multi_agent = true
/// ```
///
/// diagnostic: 「自動 fork ではなく spawn_agent の明示起動になる」
pub fn degrade_to_subagent(skill_name: &str, ir: &IRNode) -> (Vec<SideArtifact>, Vec<Diagnostic>) {
    let mut artifacts = Vec::new();
    let mut diagnostics = Vec::new();

    // 本文
    let body = ir.body.as_ref().map(|b| b.raw.as_str()).unwrap_or("");

    // description / when_to_use
    let description = ir
        .fields
        .get("skills.description")
        .and_then(|f| f.value.as_str())
        .unwrap_or("");

    // model → tier 変換
    let model_str = ir
        .fields
        .get("skills.model")
        .and_then(|f| f.value.as_str())
        .unwrap_or("");
    let codex_model = if model_str.is_empty() {
        tier_to_codex(crate::core::transforms::Tier::Mid).to_string()
    } else if let Some(tier) = claude_tier(model_str) {
        tier_to_codex(tier).to_string()
    } else {
        // 未知モデル → warn して値をそのまま使う
        diagnostics.push(Diagnostic {
            level: DiagLevel::Warn,
            id: Some("skills.model".to_string()),
            message: format!(
                "Unknown model '{}': using as-is in subagent TOML",
                model_str
            ),
        });
        model_str.to_string()
    };

    // effort → model_reasoning_effort 変換
    let effort_str = ir
        .fields
        .get("skills.effort")
        .and_then(|f| f.value.as_str())
        .unwrap_or("");
    let reasoning_effort = match effort_str {
        "max" => "xhigh",
        "xhigh" => "xhigh",
        "high" => "high",
        "medium" => "medium",
        "low" => "low",
        "" => "",
        _ => effort_str,
    };

    // .codex/agents/<skill>.toml の生成
    let agents_toml_path = format!(".codex/agents/{}.toml", skill_name);
    let mut toml_lines = vec![
        format!(r#"name = "{}""#, skill_name),
        format!(r#"description = "{}""#, description.replace('"', r#"\""#)),
    ];

    if !body.is_empty() {
        // TOML の multi-line string
        toml_lines.push(format!("developer_instructions = '''\n{}\n'''", body));
    }

    if !codex_model.is_empty() {
        toml_lines.push(format!(r#"model = "{}""#, codex_model));
    }

    if !reasoning_effort.is_empty() {
        toml_lines.push(format!(
            r#"model_reasoning_effort = "{}""#,
            reasoning_effort
        ));
    }

    artifacts.push(SideArtifact {
        path: agents_toml_path.clone(),
        content: toml_lines.join("\n") + "\n",
        note: format!("skill '{}' degraded to subagent", skill_name),
    });

    // config.toml 追記（toml_edit で非破壊追記）
    let config_update = format!(
        "[agents.{}]\nconfig_file = \"{}\"\n\n[features]\nmulti_agent = true\n",
        skill_name, agents_toml_path
    );
    artifacts.push(SideArtifact {
        path: "config.toml".to_string(),
        content: config_update,
        note: format!(
            "[agents.{}] and [features].multi_agent=true added",
            skill_name
        ),
    });

    // diagnostic: 自動 fork → 明示 spawn_agent の変化を警告
    diagnostics.push(Diagnostic {
        level: DiagLevel::Warn,
        id: Some("skills.context-fork".to_string()),
        message: format!(
            "skill '{}' degraded to subagent (.codex/agents/{}.toml). \
             自動 fork ではなく spawn_agent の明示起動になります。\
             features.multi_agent=true の設定も必要です。",
            skill_name, skill_name
        ),
    });

    (artifacts, diagnostics)
}
