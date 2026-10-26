---
paths:
  - "**/*.rs"
---

# Rust rules (load when a .rs file is read or edited)

## Code
- Money is `i64` minor units behind a validating newtype; never `f64`. `checked_*` on every balance change; an overflow rejects the row and the run continues.
- No `unwrap`, `expect`, `panic!`, `todo!` or `#[allow]` in `src/` (lint-enforced). Use `match`, `let ... else`, `?`.
- One error enum implementing `std::error::Error`, with a category and the row and field named.
- Struct fields private; modules private unless the binary or `tests/` needs them. `lib.rs` declares the modules; `main.rs` is a thin binary over the lib.
- File layout: `use`, consts, types (structs, enums), impls, free functions, then `#[cfg(test)] mod tests` last.
- Unit tests: small and specific to the file -> inline `mod tests`; a big table -> `src/<module>/tests.rs` via `#[cfg(test)] mod tests;`.
- Comments: none by default. A one-line `///` only on a `pub` item whose name does not already say what it does, starting with the item's name. A one-line `//` only for a non-obvious why. None on trait impls (`from_str`, `fmt`) or simple getters. No multi-line blocks.
- No dependency without asking. No literal binary or file names in messages: `env!("CARGO_BIN_NAME")`, the path from args.

## Traps seen before (copy the relevant ones into every builder blueprint)
1. A too-lenient decimal parser: `"-0".parse::<i64>()` is `Ok(0)`, `"+1"` is `Ok(1)`; parse the whole part as digits.
2. Display padding: `1` -> `0.0001`, `-500` -> `-0.0500`; test the extremes through the function the writer calls.
3. Release builds wrap on overflow silently; `checked_*` everywhere.
4. `HashMap::insert` on an existing key overwrites; check first.
5. `println!` panics on a closed stdout pipe; write through `BufWriter`, `flush()?`, exit 1.
6. `clippy::arithmetic_side_effects` is exempt only inside `#[test]` bodies, not in `cfg(test)` helpers.
7. A test that cannot fail proves nothing: flip one character in an expected file and see it go red.
8. A `size_of` check in a test does not fail the build; use `const _: () = assert!(...)`.
9. Crate facts (versions, dependency count, API) come from `cargo info`, `cargo tree` or docs.rs in this session; otherwise UNVERIFIED.
