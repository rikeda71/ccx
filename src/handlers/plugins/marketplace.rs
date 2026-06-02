use serde_json::{Map, Value};

use crate::core::ir::{DiagLevel, Diagnostic};

/// Completes a partial semver string (major-only → major.0.0; major.minor → major.minor.0).
pub(super) fn complete_semver(ver: &str) -> String {
    // Convert a 40-char git SHA to "0.0.0"
    if ver.len() == 40 && ver.chars().all(|c| c.is_ascii_hexdigit()) {
        return "0.0.0".to_string();
    }

    let parts: Vec<&str> = ver.split('.').collect();
    match parts.len() {
        1 => {
            // Major only
            if parts[0].parse::<u64>().is_ok() {
                format!("{}.0.0", parts[0])
            } else {
                "0.0.0".to_string()
            }
        }
        2 => {
            // Major.minor
            if parts[0].parse::<u64>().is_ok() && parts[1].parse::<u64>().is_ok() {
                format!("{}.{}.0", parts[0], parts[1])
            } else {
                "0.0.0".to_string()
            }
        }
        _ => ver.to_string(), // 3 or more components: pass through unchanged
    }
}

/// Converts marketplace.json for Codex (c2x).
/// - Claude-only top-level fields are dropped with DiagLevel::Drop diagnostics
/// - Normalizes the source schema (Claude `relative`/string → Codex `{source:"local",...}`)
/// - Fills in default policy values if missing
pub(super) fn transform_marketplace_c2x(
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    let Ok(mut json): Result<Value, _> = serde_json::from_str(content) else {
        return content.to_string();
    };

    // Drop top-level Claude-only fields that have no Codex marketplace equivalent.
    // Corresponding mappings entries all carry direction:claude_to_codex + loss:dropped.
    const CLAUDE_ONLY_FIELDS: &[(&str, &str)] = &[
        ("owner", "plugins.marketplace.owner"),
        (
            "allowCrossMarketplaceDependenciesOn",
            "plugins.marketplace.allowCrossMarketplaceDependenciesOn",
        ),
        (
            "forceRemoveDeletedPlugins",
            "plugins.marketplace.forceRemoveDeletedPlugins",
        ),
    ];
    if let Some(obj) = json.as_object_mut() {
        for (field, mapping_id) in CLAUDE_ONLY_FIELDS {
            if obj.remove(*field).is_some() {
                diagnostics.push(Diagnostic {
                    level: DiagLevel::Drop,
                    id: Some(mapping_id.to_string()),
                    message: format!("`{}` dropped (no Codex marketplace equivalent)", field),
                });
            }
        }
    }

    if let Some(plugins) = json.get_mut("plugins").and_then(|v| v.as_array_mut()) {
        for plugin_entry in plugins.iter_mut() {
            if let Some(obj) = plugin_entry.as_object_mut() {
                // Normalize the source schema
                normalize_marketplace_source_c2x(obj, diagnostics);

                // Fill in default policy if not set
                if !obj.contains_key("policy") {
                    obj.insert(
                        "policy".to_string(),
                        serde_json::json!({
                            "installation": "AVAILABLE",
                            "authentication": "ON_INSTALL"
                        }),
                    );
                    diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: Some("plugins.marketplace.plugins.policy".to_string()),
                        message: "marketplace plugin.policy auto-filled with defaults (installation=AVAILABLE, authentication=ON_INSTALL)".to_string(),
                    });
                }
            }
        }
    }

    serde_json::to_string_pretty(&json).unwrap_or_else(|_| content.to_string())
}

/// Converts marketplace.json for Claude (x2c).
/// - Normalizes the source schema (Codex `local` → Claude relative path)
/// - policy has no Claude equivalent (dropped)
pub(super) fn transform_marketplace_x2c(
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> String {
    let Ok(mut json): Result<Value, _> = serde_json::from_str(content) else {
        return content.to_string();
    };

    if let Some(plugins) = json.get_mut("plugins").and_then(|v| v.as_array_mut()) {
        for plugin_entry in plugins.iter_mut() {
            if let Some(obj) = plugin_entry.as_object_mut() {
                // Normalize the source schema
                normalize_marketplace_source_x2c(obj);

                // policy has no Claude equivalent (dropped)
                if obj.remove("policy").is_some() {
                    diagnostics.push(Diagnostic {
                        level: DiagLevel::Drop,
                        id: Some("plugins.marketplace.plugins.policy".to_string()),
                        message: "marketplace plugin.policy dropped (no Claude equivalent)"
                            .to_string(),
                    });
                }
            }
        }
    }

    serde_json::to_string_pretty(&json).unwrap_or_else(|_| content.to_string())
}

/// Normalizes the marketplace.json source schema for Codex.
/// - Relative path string → `{source: "local", path: "..."}`
/// - `github` passes through mostly unchanged (warn if field names differ)
/// - `npm` has no Codex equivalent: removes the source field and emits a Drop diagnostic
fn normalize_marketplace_source_c2x(
    obj: &mut Map<String, Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(source) = obj.get("source").cloned() {
        match &source {
            Value::String(s) => {
                // Relative path string → Codex local format
                let normalized = serde_json::json!({
                    "source": "local",
                    "path": s
                });
                obj.insert("source".to_string(), normalized);
            }
            Value::Object(src_obj) => {
                // Already in object form: inspect the source type
                if let Some(src_type) = src_obj.get("source").and_then(|v| v.as_str()) {
                    if src_type == "relative" {
                        // Claude `relative` → Codex `local`
                        let mut new_src = src_obj.clone();
                        new_src.insert("source".to_string(), Value::String("local".to_string()));
                        obj.insert("source".to_string(), Value::Object(new_src));
                    } else if src_type == "npm" {
                        // npm has no Codex equivalent; remove the field and report it dropped
                        let plugin_name = obj
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        obj.remove("source");
                        diagnostics.push(Diagnostic {
                            level: DiagLevel::Drop,
                            id: Some("plugins.marketplace.plugins.source".to_string()),
                            message: format!(
                                "marketplace plugin source type 'npm' dropped \
                                 (no Codex equivalent): plugin '{}'",
                                plugin_name
                            ),
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

/// Normalizes the marketplace.json source schema for Claude.
/// - `{source: "local", path: "..."}` → relative path string
fn normalize_marketplace_source_x2c(obj: &mut Map<String, Value>) {
    if let Some(source) = obj.get("source").cloned() {
        if let Some(src_obj) = source.as_object() {
            if let Some(src_type) = src_obj.get("source").and_then(|v| v.as_str()) {
                if src_type == "local" {
                    // Codex `local` → Claude relative path string
                    if let Some(path) = src_obj.get("path").and_then(|v| v.as_str()) {
                        obj.insert("source".to_string(), Value::String(path.to_string()));
                    }
                }
            }
        }
    }
}
