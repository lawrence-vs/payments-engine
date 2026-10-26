---
name: rust-review
description: Use when reviewing a Rust diff or PR (any repo), or before approving Rust code that touches ownership, errors, public APIs, traits, async/tokio, rayon, serde, network input or unsafe; includes a ponytail over-engineering pass. Walks a pass/fail checklist and reports every item.
---

# rust-review

Review Rust as an API code review: idioms, ownership, the [API Guidelines](https://rust-lang.github.io/api-guidelines/). Each item passes, fails or is n/a. Report every item that applies, with a one-line reason for each fail.

Hint at the rule when you ask for a change; don't write the fix.

## 1. Clean
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test` passes (and `cargo +nightly miri test` if the diff has `unsafe`).
- [ ] No `#[allow(...)]` added to silence a lint.
- [ ] Tests not edited to make the change pass; nothing hard-codes the value a test expects.

## 2. Ownership and lifetimes
- [ ] Read-only parameters borrow (`&[T]`, `&str`), not `Vec<T>` / `String` / `&Vec<T>`.
- [ ] No `.clone()` / `to_vec()` where a borrow works, or that copies more than needed.
- [ ] Fields moved out of `&mut self` use `take` / `replace`, not `.clone()`.
- [ ] Inputs from different places have separate lifetimes; no `Box::leak`, no new `'static`, no `Rc<RefCell<_>>` added only to quiet the borrow checker.

## 3. Errors
- [ ] No `unwrap` / `expect` / `panic!` / panicking index in library code (tests and `main` are fine).
- [ ] Error types are enums with a message per failure and the cause kept as `source()`; `?` works through a `From` impl.
- [ ] No `Option` or `String` standing in for an error that callers need to tell apart.
- [ ] No byte-slice of a `&str` at a computed index (`rfind + 1`, `[..1]`, a char count); `char_indices` + `len_utf8`, `get(..n)` or `is_char_boundary` instead, and a test with a multibyte case.

## 4. API design and traits
- [ ] Meaning carried by types: newtypes (`#[repr(transparent)]`) instead of bare primitives, an enum instead of several bools.
- [ ] State transitions consume `self` and return the next type where misuse should not compile.
- [ ] Generic bounds say what `T` needs; `impl Trait` vs `Box<dyn Trait>` matches whether the type is fixed or chosen at runtime.
- [ ] Clocks, singletons and other globals are injected behind a trait.

## 5. Async
- [ ] No `std` `MutexGuard` held across `.await`; no lock held over I/O.
- [ ] Spawned tasks own their data (`move`); results come back through `JoinHandle` or a channel.
- [ ] Blocking calls (sync drivers, file or CPU work) run in `spawn_blocking`.
- [ ] One pool/client built at startup and passed in, not created per call.

## 6. Untrusted input
- [ ] Queues fed from the network are bounded and reject on overload (`try_send`).
- [ ] Every network await has a `timeout` (RPC streams: a deadline).
- [ ] Lengths and offsets from the wire are checked against a limit before allocating or indexing.
- [ ] Broadcast receivers handle `Lagged(n)` as skip-and-count, not as `Closed`.
- [ ] Shared caches are bounded and load each key once.

## 7. Generated code, unsafe and FFI
- [ ] Protobuf/gRPC code comes from `build.rs`, not hand-written or checked in.
- [ ] Any `unsafe` has a `// SAFETY:` comment, is the smallest block possible, and was not added to dodge the borrow checker.
- [ ] Raw pointers stay private; bounds are checked in safe methods; resources are freed in `Drop`.
- [ ] A `CString` passed to C is bound to a variable that outlives the call.

## 8. CPU work and rayon
- [ ] CPU-bound work runs on rayon (or `spawn_blocking`), never inline on a tokio worker; rayon is called from async via `spawn_blocking` or `rayon::spawn` + `oneshot`.
- [ ] Parallel code folds/reduces instead of locking a shared `Mutex<Vec<_>>`; no shared mutable state inside `par_iter`.
- [ ] Parallelism is justified: input is big enough, and a benchmark backs any speed claim.

## 9. serde and data at the boundary
- [ ] Untrusted input types use `#[serde(deny_unknown_fields)]` and validate into newtypes (`try_from`), so invalid states can't be constructed.
- [ ] No `f64` for money or quantities that must be exact; integer minor units or a decimal type.
- [ ] Wire types and domain types are separate when they need different invariants; no `pub` fields that bypass validation.
- [ ] Errors from deserialising carry the field/context, not a bare `serde_json::Error` string; nested JSON failures name the exact path (`serde_path_to_error`, verify the crate version first).
- [ ] Strict in, liberal out: input from untrusted clients denies unknown fields; responses from a trusted upstream ignore unknown ones so a new upstream field does not break us.
- [ ] Money arrives as a string and is parsed to integer minor units without `f64`: digits only, fraction padded to the precision (`{frac:0<2}`), more fractional digits than the precision rejected (not silently cut), sign and overflow handled with `checked_*`.
- [ ] Strings from outside are bounded in length and charset (ids: fixed length, alphanumeric; no NUL or control chars) before they reach a DB, a log or a map key; JSON bodies are size-limited at the server layer before the deserializer runs.

## 10. Ponytail lens
- [ ] Smallest correct diff: nothing speculative (no trait with one impl, no config for a constant, no unused generic).
- [ ] std and the crates already in `Cargo.toml` before any new dependency; every new dependency is well-known and pinned.
- [ ] For a full over-engineering pass, run the `ponytail:ponytail-review` skill too and list its findings with these.

## 11. Tests
- [ ] Names say the behaviour (`rejects_unknown_field`, `replay_is_a_no_op`); one behaviour per test or one table with a message per case (`assert_eq!(got, want, "case {name}")`).
- [ ] Time-based async tests use `#[tokio::test(start_paused = true)]` and `tokio::time::advance`, never real sleeps.
- [ ] Invariants that must hold for any input are `proptest`s (balances never negative, replay-order independence).
- [ ] Errors are matched by variant (`matches!(err, Error::Stale { .. })`), not by message string; no `#[should_panic]` for library errors.
- [ ] A real in-memory implementation (SQLite in memory, a channel) over a mock where it's cheap.

## Verdict
Approve only when every applicable item passes. Otherwise request changes and list the failed items by number (e.g. "5.2: this task borrows a local; what should it own instead?").
