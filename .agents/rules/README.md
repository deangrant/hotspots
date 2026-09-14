# Agent rules

Repository constraints under [`.agents/rules/`](./). Cursor loads them via
[`.cursor/rules`](../../.cursor/rules) → `../.agents/rules`.

| Rule | Scope |
| ---- | ----- |
| [gate-contract](./gate-contract/) | Always-on — run `/verify`; never relax gates or pins |
| [rust-complexity-budget](./rust-complexity-budget/) | Glob `**/*.rs` — clippy / file size / CC≤8 dogfood |
| [hotspots-domain](./hotspots-domain/) | Glob product crates — log/grain/UX invariants |
| [ai-slop-mitigation](./ai-slop-mitigation/) | Opt-in — concrete diffs, no filler |
