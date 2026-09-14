# Verify

Read and follow the **verify-gates** skill at
[`.agents/skills/verify-gates/SKILL.md`](../skills/verify-gates/SKILL.md).

From the repo root, run the ordered gate: `./scripts/check.sh`, then llvm-cov,
then `./scripts/crap-gate.sh` (see the skill for exact commands).

Print failures clearly. Fix root causes, then re-run until green.
