use std::collections::HashMap;
use std::path::Path;

use serde_json::{Map, Value};

use crate::core::ir::{DiagLevel, Diagnostic, IRNode, SideArtifact};
use crate::core::mappings::MapEntry;
use crate::core::transforms::ConvDir;
use crate::handlers::Handler;

use super::fs::collect_md_files;
use super::PluginsHandler;

impl PluginsHandler {
    /// Lifts manifest fields driven by mappings.
    pub(super) fn lift_manifest_fields(
        &self,
        frontmatter: &Map<String, Value>,
        idx: &HashMap<String, MapEntry>,
        dir: ConvDir,
        node: &mut IRNode,
    ) {
        // Save userConfig so we can warn about unresolved variable references later
        let user_config = frontmatter.get("userConfig");

        for (key, value) in frontmatter {
            // experimental is expanded so each sub-field gets its own mapping entry
            if key == "experimental" {
                if let Some(exp_obj) = value.as_object() {
                    for (sub_key, sub_value) in exp_obj {
                        let full_key = format!("experimental.{}", sub_key);
                        self.lift_single_field(&full_key, sub_value, idx, dir, node);
                    }
                } else {
                    // Malformed experimental value (not an object): treat as an unknown field
                    // so a dropped/unknown-field diagnostic is preserved.
                    self.lift_single_field(key, value, idx, dir, node);
                }
                continue;
            }

            // interface is expanded so each sub-field (interface.displayName, etc.)
            // gets routed individually through the mappings index
            if key == "interface" {
                if let Some(iface_obj) = value.as_object() {
                    for (sub_key, sub_value) in iface_obj {
                        let full_key = format!("interface.{}", sub_key);
                        self.lift_single_field(&full_key, sub_value, idx, dir, node);
                    }
                } else {
                    // Malformed interface value (not an object): treat as an unknown field
                    // so a dropped/unknown-field diagnostic is preserved.
                    self.lift_single_field(key, value, idx, dir, node);
                }
                continue;
            }

            self.lift_single_field(key, value, idx, dir, node);
        }

        // c2x: warn if userConfig is present; ${user_config.KEY} references in MCP/hooks may remain unresolved
        if dir == ConvDir::C2x {
            if let Some(uc) = user_config {
                if uc.is_object() || uc.is_array() {
                    node.diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: Some("plugins.userConfig".to_string()),
                        message: "userConfig found: ${user_config.KEY} references in MCP/hooks may remain unresolved after c2x conversion (Codex has no userConfig equivalent)".to_string(),
                    });
                }
            }
        }
    }

    pub(super) fn lift_single_field(
        &self,
        key: &str,
        value: &Value,
        idx: &HashMap<String, MapEntry>,
        dir: ConvDir,
        node: &mut IRNode,
    ) {
        let Some(entry) = idx.get(key) else {
            // Unknown field: treat as dropped
            node.diagnostics.push(Diagnostic {
                level: DiagLevel::Drop,
                id: None,
                message: format!("unknown plugin manifest field: {key}"),
            });
            return;
        };

        crate::handlers::lift_mapped_field(entry, key, value, dir, node);
    }

    /// Recursively converts the skills/ directory and appends the results to children.
    pub(super) fn lift_child_skills(
        &self,
        plugin_root: &str,
        frontmatter: &Map<String, Value>,
        dir: ConvDir,
        node: &mut IRNode,
    ) {
        // The `skills` manifest field is string|array.  Collect all paths.
        let skills_dirs: Vec<String> = match frontmatter.get("skills") {
            Some(Value::String(s)) => vec![s.clone()],
            Some(Value::Array(arr)) => {
                // Codex manifest `skills` is a single string, so a multi-path array
                // cannot be fully represented — warn so the caller can resolve it.
                node.diagnostics.push(Diagnostic {
                    level: DiagLevel::Warn,
                    id: Some("plugins.skills".to_string()),
                    message: format!(
                        "plugins.skills is an array with {} paths; all entries are converted as children but the Codex manifest `skills` field is a single string — only one path can be represented in the output manifest",
                        arr.len()
                    ),
                });
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            }
            _ => vec!["./skills/".to_string()],
        };

        let skills_handler = crate::handlers::skills::SkillsHandler {
            map: self.maps["skills"].clone(),
        };

        for skills_dir in &skills_dirs {
            // Normalize: ./skills/ → skills
            let skills_rel = skills_dir.trim_start_matches("./").trim_end_matches('/');
            let skills_path_str = format!("{}/{}", plugin_root, skills_rel);
            let skills_path = Path::new(&skills_path_str);

            if !skills_path.exists() {
                continue;
            }

            // Process each SKILL.md under the resolved skills directory
            if let Ok(entries) = std::fs::read_dir(skills_path) {
                for entry in entries.flatten() {
                    let skill_dir = entry.path();
                    if !skill_dir.is_dir() {
                        continue;
                    }
                    let skill_md = skill_dir.join("SKILL.md");
                    if !skill_md.exists() {
                        continue;
                    }

                    match skills_handler.parse(&skill_md) {
                        Ok(parsed) => match skills_handler.lift(&parsed, dir) {
                            Ok(child_ir) => {
                                node.children.push(child_ir);
                            }
                            Err(e) => {
                                node.diagnostics.push(Diagnostic {
                                    level: DiagLevel::Warn,
                                    id: None,
                                    message: format!("Failed to lift skill {:?}: {}", skill_md, e),
                                });
                            }
                        },
                        Err(e) => {
                            node.diagnostics.push(Diagnostic {
                                level: DiagLevel::Warn,
                                id: None,
                                message: format!("Failed to parse skill {:?}: {}", skill_md, e),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Recursively converts the hooks file and appends the result to children.
    pub(super) fn lift_child_hooks(
        &self,
        plugin_root: &str,
        frontmatter: &Map<String, Value>,
        dir: ConvDir,
        node: &mut IRNode,
    ) {
        let hooks_handler = crate::handlers::hooks::HooksHandler {
            map: self.maps["hooks"].clone(),
        };

        let hooks_value = frontmatter.get("hooks");

        // Inline object form: serialize and feed directly through the hooks handler.
        if let Some(hooks_obj) = hooks_value.and_then(|v| v.as_object()) {
            node.diagnostics.push(Diagnostic {
                level: DiagLevel::Warn,
                id: Some("plugins.hooks".to_string()),
                message: format!(
                    "Inline hooks object in plugin.json has {} entries; writing to hooks file for Codex compatibility",
                    hooks_obj.len()
                ),
            });

            // Build a synthetic parsed value as if it came from a hooks file.
            // The hooks handler expects the top-level value to be the hooks object itself.
            let synthetic = Value::Object(hooks_obj.clone());
            match hooks_handler.lift(&synthetic, dir) {
                Ok(mut child_ir) => {
                    child_ir.diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: Some("plugins.hooks".to_string()),
                        message: "Plugin-bundled hooks may not be loaded by Codex (#16430). Use --hooks-target=user|project to output hooks to ~/.codex/hooks.json or .codex/config.toml instead.".to_string(),
                    });
                    node.children.push(child_ir);
                }
                Err(e) => {
                    node.diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: None,
                        message: format!("Failed to lift inline hooks object: {}", e),
                    });
                }
            }
            return;
        }

        // String reference form: resolve path and parse the file.
        let hooks_path_str = hooks_value
            .and_then(|v| v.as_str())
            .unwrap_or("./hooks/hooks.json");

        let hooks_rel = hooks_path_str.trim_start_matches("./");
        let hooks_path_owned = format!("{}/{}", plugin_root, hooks_rel);
        let hooks_path = Path::new(&hooks_path_owned);

        if !hooks_path.exists() {
            return;
        }

        match hooks_handler.parse(hooks_path) {
            Ok(parsed) => match hooks_handler.lift(&parsed, dir) {
                Ok(mut child_ir) => {
                    child_ir.diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: Some("plugins.hooks".to_string()),
                        message: "Plugin-bundled hooks may not be loaded by Codex (#16430). Use --hooks-target=user|project to output hooks to ~/.codex/hooks.json or .codex/config.toml instead.".to_string(),
                    });
                    node.children.push(child_ir);
                }
                Err(e) => {
                    node.diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: None,
                        message: format!("Failed to lift hooks {:?}: {}", hooks_path, e),
                    });
                }
            },
            Err(e) => {
                node.diagnostics.push(Diagnostic {
                    level: DiagLevel::Warn,
                    id: None,
                    message: format!("Failed to parse hooks {:?}: {}", hooks_path, e),
                });
            }
        }
    }

    /// Recursively converts .mcp.json and appends the result to children.
    pub(super) fn lift_child_mcp(
        &self,
        plugin_root: &str,
        frontmatter: &Map<String, Value>,
        dir: ConvDir,
        node: &mut IRNode,
    ) {
        let mcp_handler = crate::handlers::mcp::McpHandler {
            map: self.maps["mcp"].clone(),
        };

        let mcp_value = frontmatter.get("mcpServers");

        // Inline object form: serialize and feed directly through the MCP handler.
        if let Some(mcp_obj) = mcp_value.and_then(|v| v.as_object()) {
            node.diagnostics.push(Diagnostic {
                level: DiagLevel::Warn,
                id: Some("plugins.mcpServers".to_string()),
                message: "Inline mcpServers object in plugin.json: Codex requires a file path reference. Will attempt to emit as .mcp.json.".to_string(),
            });

            // Wrap in the envelope that parse_json_file produces and lift_c2x/x2c expect.
            let synthetic = serde_json::json!({
                "frontmatter": { "mcpServers": mcp_obj },
                "body": "",
                "path": ""
            });
            match mcp_handler.lift(&synthetic, dir) {
                Ok(child_ir) => {
                    node.children.push(child_ir);
                }
                Err(e) => {
                    node.diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: None,
                        message: format!("Failed to lift inline mcpServers object: {}", e),
                    });
                }
            }
            return;
        }

        // String reference form: resolve path and parse the file.
        let mcp_path_str = mcp_value.and_then(|v| v.as_str()).unwrap_or("./.mcp.json");

        let mcp_rel = mcp_path_str.trim_start_matches("./");
        let mcp_path_owned = format!("{}/{}", plugin_root, mcp_rel);
        let mcp_path = Path::new(&mcp_path_owned);

        if !mcp_path.exists() {
            return;
        }

        match mcp_handler.parse(mcp_path) {
            Ok(parsed) => match mcp_handler.lift(&parsed, dir) {
                Ok(child_ir) => {
                    node.children.push(child_ir);
                }
                Err(e) => {
                    node.diagnostics.push(Diagnostic {
                        level: DiagLevel::Warn,
                        id: None,
                        message: format!("Failed to lift .mcp.json {:?}: {}", mcp_path, e),
                    });
                }
            },
            Err(e) => {
                node.diagnostics.push(Diagnostic {
                    level: DiagLevel::Warn,
                    id: None,
                    message: format!("Failed to parse .mcp.json {:?}: {}", mcp_path, e),
                });
            }
        }
    }

    /// Processes marketplace.json and stores it in side_artifacts.
    /// plugin_root is the directory containing plugin.json (e.g. `.claude-plugin/`).
    pub(super) fn lift_marketplace(&self, plugin_root: &str, dir: ConvDir, node: &mut IRNode) {
        // marketplace.json lives in the same directory as plugin.json:
        // Claude: .claude-plugin/marketplace.json (= {plugin_root}/marketplace.json)
        // Codex:  .agents/plugins/marketplace.json (= {plugin_root}/marketplace.json)
        let local_marketplace = format!("{}/marketplace.json", plugin_root);

        let marketplace_path = match dir {
            ConvDir::C2x => {
                let p = Path::new(&local_marketplace);
                if p.exists() {
                    Some(p.to_path_buf())
                } else {
                    None
                }
            }
            ConvDir::X2c => {
                let p = Path::new(&local_marketplace);
                if p.exists() {
                    Some(p.to_path_buf())
                } else {
                    None
                }
            }
        };

        let Some(mp_path) = marketplace_path else {
            return;
        };

        match std::fs::read_to_string(&mp_path) {
            Ok(content) => {
                // Save marketplace.json for conversion and emission during lower
                node.side_artifacts.push(SideArtifact {
                    path: mp_path.to_string_lossy().to_string(),
                    content,
                    note: "marketplace.json".to_string(),
                });
            }
            Err(e) => {
                node.diagnostics.push(Diagnostic {
                    level: DiagLevel::Warn,
                    id: None,
                    message: format!("Failed to read marketplace.json {:?}: {}", mp_path, e),
                });
            }
        }
    }

    /// Discovers `commands/` at the plugin root and stores the files as side artifacts.
    /// Both Claude and Codex use an identically named directory — conversion is a
    /// lossless path-remap.
    pub(super) fn lift_child_commands(&self, plugin_root: &str, node: &mut IRNode) {
        let commands_path_str = format!("{}/commands", plugin_root);
        let commands_path = Path::new(&commands_path_str);
        if !commands_path.exists() {
            return;
        }

        for file in collect_md_files(commands_path) {
            node.diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                id: Some("plugins.commands".to_string()),
                message: format!(
                    "commands/{}: path-remapped losslessly to output commands/",
                    file.rel_path.trim_start_matches("commands/")
                ),
            });
            node.side_artifacts.push(SideArtifact {
                // path stores the relative path within the plugin dir (e.g. "commands/foo.md")
                path: file.rel_path,
                content: file.content,
                note: "commands".to_string(),
            });
        }
    }

    /// Discovers `agents/` at the plugin root and stores the files as side artifacts.
    /// Both Claude and Codex auto-discover agent `.md` files here — conversion is a
    /// lossy path-remap (per-file frontmatter may need subagent-rule conversion).
    pub(super) fn lift_child_agents(&self, plugin_root: &str, node: &mut IRNode) {
        let agents_path_str = format!("{}/agents", plugin_root);
        let agents_path = Path::new(&agents_path_str);
        if !agents_path.exists() {
            return;
        }

        for file in collect_md_files(agents_path) {
            node.diagnostics.push(Diagnostic {
                level: DiagLevel::Warn,
                id: Some("plugins.agents".to_string()),
                message: format!(
                    "agents/{}: path-remapped to output agents/ (lossy — per-agent frontmatter may need subagent-rule conversion)",
                    file.rel_path.trim_start_matches("agents/")
                ),
            });
            node.side_artifacts.push(SideArtifact {
                // path stores the relative path within the plugin dir (e.g. "agents/bar.md")
                path: file.rel_path,
                content: file.content,
                note: "agents".to_string(),
            });
        }
    }
}
