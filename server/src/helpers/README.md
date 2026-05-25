# helpers/ — Miscellaneous utilities

## Files

### `common.rs`

```rust
pub fn return_error_if_true(...) { ... }
```

Legacy helper from the hiqlite era. Currently **unused** — the services use direct
`if/return Err(...)` expressions instead. Kept in the codebase but marked with a
dead-code warning. Safe to delete when confirmed no callers remain.

## Note

This module has no meaningful content after the migration. If new cross-cutting
utilities are needed (e.g., timestamp helpers, retry logic), add them here.
