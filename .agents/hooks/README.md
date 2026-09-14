# Agent hooks

Shell hooks under [`.agents/hooks/`](./). Wired from
[`.cursor/hooks.json`](../../.cursor/hooks.json).

| Hook | Script | Behavior |
| ---- | ------ | -------- |
| `afterFileEdit` | [rustfmt.sh](./rustfmt.sh) | Format edited `*.rs` (fail-open) |
| `sessionStart` | [session-context.sh](./session-context.sh) | Inject short workspace context (fail-open) |
| `stop` | [verify-on-stop.sh](./verify-on-stop.sh) | On relevant dirty trees, run [run-verify.sh](./run-verify.sh) (`loop_limit` 3, fail-open) |

`run-verify.sh` matches [verify-gates](../skills/verify-gates/SKILL.md):
`check.sh` → `dry-gate.sh` → llvm-cov → `crap-gate.sh`.

Hooks do not replace an explicit `/verify` before claiming done.
