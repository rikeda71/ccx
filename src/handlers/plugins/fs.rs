use std::path::Path;

/// A path-remapped file discovered under `commands/` or `agents/` at a plugin root.
pub(super) struct PluginDirFile {
    /// Relative path within the plugin dir (e.g. `"commands/foo.md"`).
    pub(super) rel_path: String,
    pub(super) content: String,
}

/// Walks `dir` recursively and returns all `.md` files as `PluginDirFile` entries.
/// `rel_path` is relative to the *parent* of `dir` (i.e. the plugin root), so a file
/// at `plugin_root/commands/foo.md` produces `rel_path = "commands/foo.md"`.
pub(super) fn collect_md_files(dir: &Path) -> Vec<PluginDirFile> {
    let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let mut results = Vec::new();
    collect_md_files_recursive(dir, dir_name, &mut results);
    results
}

fn collect_md_files_recursive(dir: &Path, prefix: &str, out: &mut Vec<PluginDirFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let rel = format!("{}/{}", prefix, name);
        if path.is_dir() {
            collect_md_files_recursive(&path, &rel, out);
        } else if name.ends_with(".md") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                out.push(PluginDirFile {
                    rel_path: rel,
                    content,
                });
            }
            // unreadable files are skipped silently
        }
    }
}
