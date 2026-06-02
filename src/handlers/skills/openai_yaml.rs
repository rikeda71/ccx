use std::path::Path;

use serde_json::Value;

use crate::core::ir::{DiagLevel, Diagnostic, IRField, IRNode, Loss};

use super::SkillsHandler;

/// Outcome of attempting to load `agents/openai.yaml` alongside a SKILL.md.
pub(super) enum OpenaiYamlResult {
    /// File is absent — not an error.
    Missing,
    /// File is present but could not be read or parsed.
    Broken(String),
    /// File loaded and parsed successfully.
    Ok(Value),
}

/// Loads `agents/openai.yaml` from the skill directory.
///
/// Returns `Missing` when the file does not exist, `Broken` when the file
/// exists but cannot be read or parsed (callers should surface a warning), and
/// `Ok` on success.
pub(super) fn load_openai_yaml(source_path: &str) -> OpenaiYamlResult {
    let skill_dir = match Path::new(source_path).parent() {
        Some(d) => d,
        None => return OpenaiYamlResult::Missing,
    };
    let openai_yaml = skill_dir.join("agents").join("openai.yaml");
    if !openai_yaml.exists() {
        return OpenaiYamlResult::Missing;
    }
    let content = match std::fs::read_to_string(&openai_yaml) {
        Ok(c) => c,
        Err(e) => {
            return OpenaiYamlResult::Broken(format!("agents/openai.yaml: failed to read: {}", e))
        }
    };
    match serde_saphyr::from_str::<Value>(&content) {
        Ok(v) => OpenaiYamlResult::Ok(v),
        Err(e) => OpenaiYamlResult::Broken(format!("agents/openai.yaml: failed to parse: {}", e)),
    }
}

impl SkillsHandler {
    /// Process the `agents/openai.yaml` companion file during x2c lift and insert
    /// the relevant fields into `node`.
    pub(super) fn lift_openai_yaml_companion(&self, source_path: &str, node: &mut IRNode) {
        let openai_result = load_openai_yaml(source_path);
        if let OpenaiYamlResult::Broken(msg) = &openai_result {
            // A present-but-broken companion file must surface a warning so
            // that data loss from policy.*/interface.*/dependencies.tools is visible.
            node.diagnostics.push(Diagnostic {
                level: DiagLevel::Warn,
                id: Some("skills.openai-yaml".to_string()),
                message: format!(
                    "fail-open WARNING: {} — policy.*/interface.*/dependencies.tools data may be lost",
                    msg
                ),
            });
        }
        if let OpenaiYamlResult::Ok(openai_val) = openai_result {
            // policy.allow_implicit_invocation → disable-model-invocation (polarity invert)
            if let Some(allow_implicit) =
                openai_val["policy"]["allow_implicit_invocation"].as_bool()
            {
                let disable_val = Value::Bool(!allow_implicit);
                node.fields.insert(
                    "skills.disable-model-invocation".to_string(),
                    IRField {
                        id: "skills.disable-model-invocation".to_string(),
                        value: disable_val,
                        loss: Loss::Lossless,
                        transforms_applied: vec!["polarity:invert".to_string()],
                        degrade: None,
                        warning: None,
                        dropped: None,
                    },
                );
            }

            // interface.display_name / icon_small / icon_large / brand_color → warn + lossy
            let iface = &openai_val["interface"];
            let has_visual_fields = ["display_name", "icon_small", "icon_large", "brand_color"]
                .iter()
                .any(|k| !iface[*k].is_null());
            if has_visual_fields {
                if let Some(entry) = self
                    .map
                    .entries
                    .iter()
                    .find(|e| e.id == "skills.openai-yaml.interface")
                {
                    let warning_msg =
                        format!("{}: {}", entry.id, entry.notes.as_deref().unwrap_or("warn"));
                    node.fields.insert(
                        "skills.openai-yaml.interface".to_string(),
                        IRField {
                            id: "skills.openai-yaml.interface".to_string(),
                            value: iface.clone(),
                            loss: Loss::Lossy,
                            transforms_applied: vec![],
                            degrade: None,
                            warning: Some(warning_msg.clone()),
                            dropped: None,
                        },
                    );
                    node.diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: Some("skills.openai-yaml.interface".to_string()),
                        message: warning_msg,
                    });
                }
            }

            // interface.default_prompt → lossy approximate: prepended to skill body
            if let Some(default_prompt) = iface["default_prompt"].as_str() {
                if !default_prompt.is_empty() {
                    if let Some(entry) = self
                        .map
                        .entries
                        .iter()
                        .find(|e| e.id == "skills.openai-yaml.default_prompt")
                    {
                        let _ = entry;
                        node.fields.insert(
                            "skills.openai-yaml.default_prompt".to_string(),
                            IRField {
                                id: "skills.openai-yaml.default_prompt".to_string(),
                                value: Value::String(default_prompt.to_string()),
                                loss: Loss::Lossy,
                                transforms_applied: vec![],
                                degrade: None,
                                warning: None,
                                dropped: None,
                            },
                        );
                    }
                }
            }

            // dependencies.tools → warn + lossy
            let deps_tools = &openai_val["dependencies"]["tools"];
            if !deps_tools.is_null() {
                if let Some(entry) = self
                    .map
                    .entries
                    .iter()
                    .find(|e| e.id == "skills.openai-yaml.dependencies-tools")
                {
                    let warning_msg =
                        format!("{}: {}", entry.id, entry.notes.as_deref().unwrap_or("warn"));
                    node.fields.insert(
                        "skills.openai-yaml.dependencies-tools".to_string(),
                        IRField {
                            id: "skills.openai-yaml.dependencies-tools".to_string(),
                            value: deps_tools.clone(),
                            loss: Loss::Lossy,
                            transforms_applied: vec![],
                            degrade: None,
                            warning: Some(warning_msg.clone()),
                            dropped: None,
                        },
                    );
                    node.diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: Some("skills.openai-yaml.dependencies-tools".to_string()),
                        message: warning_msg,
                    });
                }
            }
        }
    }
}
