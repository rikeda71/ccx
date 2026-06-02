use std::path::Path;

use anyhow::Context;

use crate::handlers::EmitFile;

/// Walks `skill_dir` and collects all non-`.md` files (excluding
/// `agents/openai.yaml`) as `EmitFile` values with paths remapped under
/// `out_skill_dir`.
///
/// Content is read as UTF-8; binary files are silently skipped.
pub(super) fn collect_aux_files(
    skill_dir: &Path,
    out_skill_dir: &str,
) -> anyhow::Result<Vec<EmitFile>> {
    let mut result = Vec::new();
    collect_aux_files_recursive(skill_dir, skill_dir, out_skill_dir, &mut result)?;
    Ok(result)
}

fn collect_aux_files_recursive(
    base_dir: &Path,
    current_dir: &Path,
    out_skill_dir: &str,
    result: &mut Vec<EmitFile>,
) -> anyhow::Result<()> {
    let entries = match std::fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry.with_context(|| {
            format!(
                "Failed to read directory entry in {}",
                current_dir.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_aux_files_recursive(base_dir, &path, out_skill_dir, result)?;
            continue;
        }
        // Skip .md files (SKILL.md is handled separately)
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            continue;
        }
        // Compute relative path from base_dir
        let rel = path.strip_prefix(base_dir).with_context(|| {
            format!(
                "Path {} is not under {}",
                path.display(),
                base_dir.display()
            )
        })?;
        // Skip agents/openai.yaml (handled separately as SideArtifact or via lift)
        let rel_str = rel.to_str().unwrap_or("");
        if rel_str == "agents/openai.yaml" || rel_str == "agents\\openai.yaml" {
            continue;
        }
        // Read content as UTF-8; skip silently if not valid UTF-8 (binary)
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let out_path = format!("{}/{}", out_skill_dir, rel_str.replace('\\', "/"));
        result.push(EmitFile {
            path: out_path,
            content,
        });
    }
    Ok(())
}

/// Extracts the skill name from source_path.
/// .claude/skills/<name>/SKILL.md → <name>
/// .agents/skills/<name>/SKILL.md → <name>
/// Anything else → "skill"
pub(super) fn extract_skill_name(source_path: &str) -> String {
    let path = Path::new(source_path);
    // Return the name of the parent directory of SKILL.md
    if let Some(parent) = path.parent() {
        if let Some(name) = parent.file_name() {
            let n = name.to_str().unwrap_or("unknown");
            if n != "skills" && n != ".claude" && n != ".agents" {
                return n.to_string();
            }
        }
    }
    // Fallback: the string "skill"
    "skill".to_string()
}
