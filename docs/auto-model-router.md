# Automatic model router

This fork can choose a model and reasoning effort for each new interactive TUI turn while keeping
the same Codex task.

| Prompt shape | Route |
| --- | --- |
| Small mechanical edit, lookup, typo, or rename | Sol with low reasoning |
| Moderate analysis or scoped debugging | Terra with low or medium reasoning |
| Code implementation, integration, or refactoring | Astra with low reasoning, or medium when broad or difficult |
| Architecture, concurrency, destructive work, repeated failures, or ambiguous production work | Astra with medium reasoning |

High reasoning is reserved for prompts that combine severe risk with maximum ambiguity. The router
does not select xhigh, max, or ultra by default.

## Use it

Automatic routing is enabled by default. Open `/model` and select **Auto Router** to turn it back
on after making a manual model or reasoning selection. Selecting a specific model or reasoning
level turns automatic routing off.

To configure it directly:

```toml
[tui]
auto_model_routing = true
```

Set the value to `false` to keep one manually selected model.

When the route changes, the TUI adds a short message such as:

```text
Auto-routed: Astra · medium
```

The current model indicator then reflects the routed model and effort. Input sent while a turn is
already active stays on that turn's route because steering cannot replace active-turn settings.

## Tune the policy

The classifier and fallback policy live in `codex-rs/tui/src/auto_router.rs`. It is deterministic,
local, and does not spend model tokens. It scores prompt complexity, scope, ambiguity, and risk,
then resolves the requested family against the model catalog available to the signed-in account.

If a requested family or reasoning level is unavailable, the resolver selects the nearest router
family and supported effort. If none of Sol, Terra, or Astra is present, Codex reports the problem
once and uses the manually selected model.

The routing examples and fallback rules are covered in
`codex-rs/tui/src/auto_router_tests.rs`. Per-turn integration coverage is in
`codex-rs/tui/src/chatwidget/tests/composer_submission.rs`.

## Build

Follow the repository's source setup in `docs/install.md`, then run from `codex-rs`:

```bash
cargo build --release --bin codex
cargo run --release --bin codex
```

The release binary is written below `codex-rs/target/release`.
