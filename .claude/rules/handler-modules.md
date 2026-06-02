---
paths:
  - "src/handlers/**"
---

# Handler Module Layout

Each domain handler under `src/handlers/` is a **directory module**
(`src/handlers/<domain>/`), not a single flat file. Files within it follow a
fixed naming convention so any handler reads the same way.

## File roles

| File | Holds |
|---|---|
| `mod.rs` | The `<Domain>Handler` struct, `impl Handler` (the `kind`/`detect`/`parse`/`lift`/`lower` dispatch), shared `const`s, the `mod` submodule declarations, and the `#[cfg(test)]` tests. |
| `parse.rs` | Reading a file into the shared `Value` (`{frontmatter, body, path}`) and its parse helpers. Omit if `parse` is a one-liner that stays in `mod.rs`. |
| `lift.rs` | The lift logic (`Value` → `IRNode`), both directions. |
| `lower.rs` | The lower logic (`IRNode` → `EmitPlan`), both directions. |
| `lower_c2x.rs` / `lower_x2c.rs` | Used **instead of** `lower.rs` only when each direction is large enough to stand alone (rough guide: a single `lower.rs` would exceed ~300–400 lines). Otherwise keep one `lower.rs`. |
| domain-specific files | Self-contained concerns get their own file, named after the concern: e.g. `openai_yaml.rs` (skills), `aux_files.rs` (skills), `import.rs` (memory `@import`), `toml_convert.rs` (hooks TOML⇄JSON), `marketplace.rs` (plugins). |

## Conventions

- **Visibility stays minimal.** The handler struct and its `map` field remain
  `pub` (`pick_handler` and tests construct them with field syntax). Submodules
  are declared non-`pub` (`mod lift;`, not `pub mod lift;`). Helpers shared
  across files within one handler are `pub(super)` / `pub(crate)` — never wider
  than needed.
- **`pub mod <domain>;` in `src/handlers/mod.rs` does not change** when a flat
  file becomes a directory module; Rust resolves `<domain>` to `<domain>/mod.rs`.
- **Splitting is behavior-preserving.** Moving code between these files must not
  change the conversion output or diagnostics — it is a pure reorganization.
- Put a `#[cfg(test)]` test next to the code it exercises (its submodule), or in
  `mod.rs` when it drives the handler end-to-end.
