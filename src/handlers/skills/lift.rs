use serde_json::Value;

use crate::core::ir::{new_node, BodySegment, DiagLevel, Diagnostic, IRNode, Kind, Tool};
use crate::core::mappings::{index_by_claude_field, index_by_codex_field};
use crate::core::transforms::ConvDir;
use crate::scanner::body::{scan_body, BodyContext};

use super::SkillsHandler;

impl SkillsHandler {
    pub(super) fn lift_impl(&self, parsed: &Value, dir: ConvDir) -> anyhow::Result<IRNode> {
        let source_tool = match dir {
            ConvDir::C2x => Tool::Claude,
            ConvDir::X2c => Tool::Codex,
        };
        let source_path = parsed["path"].as_str().unwrap_or("").to_string();
        let mut node = new_node(Kind::Skill, source_tool, &source_path);

        let idx = match dir {
            ConvDir::C2x => index_by_claude_field(&self.map),
            ConvDir::X2c => index_by_codex_field(&self.map),
        };

        let frontmatter = match parsed["frontmatter"].as_object() {
            Some(fm) => fm,
            None => {
                return Ok(node);
            }
        };

        // Preserve original values so --keep-claude-frontmatter can write them
        // without accidentally writing post-transform (e.g. polarity-inverted) values.
        node.raw_frontmatter = Some(frontmatter.clone());

        for (key, value) in frontmatter {
            let Some(&entry) = idx.get(key.as_str()) else {
                node.diagnostics.push(Diagnostic {
                    level: DiagLevel::Drop,
                    id: None,
                    message: format!("unknown frontmatter key: {key}"),
                });
                continue;
            };

            crate::handlers::lift_mapped_field(entry, key, value, dir, &mut node);
        }

        // x2c: process agents/openai.yaml when present
        if dir == ConvDir::X2c {
            self.lift_openai_yaml_companion(&source_path, &mut node);
        }

        // Body scan
        let body_raw = parsed["body"].as_str().unwrap_or("").to_string();
        let findings = scan_body(&body_raw, dir, BodyContext::SkillBody);
        node.body = Some(BodySegment {
            raw: body_raw,
            findings,
        });

        Ok(node)
    }
}
