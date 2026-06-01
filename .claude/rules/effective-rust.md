---
paths:
  - "src/**"
  - "tests/**"
---

# Effective Rust

Heuristics distilled from *Effective Rust* (David Drysdale). Apply them when
writing and reviewing code. They complement `rust.md` (quality gates) and
`code-style.md` (readability) — when they conflict, the local style of the
surrounding code wins.

## Types — make illegal states unrepresentable
- Prefer expressive types over primitives. Use an `enum` instead of a `bool`,
  integer, or stringly-typed tag for a finite set of choices. Introduce a
  **newtype** when a value carries units, an invariant, or must not be mixed
  with other `String`/`usize` values.
- Encode presence/absence with `Option`, fallibility with `Result` — never a
  sentinel value (`-1`, empty string, `0`).
- Use `Result<T, E>` and `?`; avoid `unwrap`/`expect`/`panic!` on runtime
  paths. Startup invariant checks that must abort may panic with a clear
  message; tests may `unwrap`.
- Prefer combinators (`map`, `and_then`, `unwrap_or`, `ok_or`, `?`,
  `map_or`, `matches!`) over verbose `match` when they read more clearly.
- Add `#[must_use]` to a function whose result is a bug to ignore.

## Traits & std idioms
- Implement the standard traits where they fit: `From`/`TryFrom` for
  conversions (then call `.into()`/`.try_into()`), `Display` for user-facing
  text, `Default`, `FromStr`, `Iterator`. **Derive** (`Debug`, `Clone`,
  `PartialEq`, `Eq`, `Hash`, `Default`, `Copy`) instead of hand-writing.
- Take borrowed types in public APIs (`&str`, `&[T]`, `impl AsRef<Path>`);
  return owned types. Don't take `String`/`Vec<T>` by value just to read it.
- Prefer iterators and adapters over index loops and manual `push`.

## Ownership, borrowing, allocation
- Avoid needless `clone()` / `to_string()`; clone only when ownership is truly
  required. Restructure code to satisfy the borrow checker before reaching for
  a clone.
- Don't `collect()` into a `Vec` only to iterate it once.
- Keep lifetimes simple; if an explicit lifetime makes a public API awkward,
  prefer owning the data.

## Errors
- Use `anyhow` at this crate's boundaries with `.context(...)`, and keep
  messages actionable. Separate "expected/recoverable" (`Result`) from
  "programmer bug" (assert/panic).

## Construction & API shape
- For a type with many optional fields, prefer the builder pattern or
  struct-update with `Default` over a long positional constructor.
- Keep the public surface minimal — `pub(crate)`/private unless a wider scope
  is genuinely needed. One responsibility per module.

## Safety & tooling
- Minimize `unsafe`; if unavoidable, document the invariant that makes it
  sound. This crate should contain **no** `unsafe`.
- Keep `cargo clippy --all-targets -- -D warnings` clean without blanket
  `#[allow(...)]`. Give every public item a one-line doc of its purpose and
  contract.
