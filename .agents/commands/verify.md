# Verify

Read and follow the **verify-gates** skill at
[`.agents/skills/verify-gates/SKILL.md`](../skills/verify-gates/SKILL.md).

From the repo root, run the ordered gate: `./scripts/check.sh`, then
`./scripts/dry-gate.sh`, then llvm-cov, then `./scripts/crap-gate.sh` (see the
skill for exact commands).

Print failures clearly. Fix root causes, then re-run until green.

If the user says **report only**, run the gates and summarize pass/fail only —
do not change code to fix failures.
