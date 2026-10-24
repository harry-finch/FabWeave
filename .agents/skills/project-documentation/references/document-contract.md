# Project document contract

## STATUS.md

Answer quickly: current milestone, known-good commit, build/test status, blockers, and next action. Avoid historical detail.

## DEVLOG.md

Chronological and append-oriented. Record meaningful sessions: scope, subsystems investigated, changes, validations, discoveries, unresolved issues.

## ROADMAP.md

Stable plan, not a diary. Each milestone should have goal, included/excluded scope, acceptance criteria, and status.

## ARCHITECTURE.md

Describe stable boundaries and data flow. Avoid duplicating low-level implementation details that are obvious from code and likely to go stale.

## UPSTREAM_ANALYSIS.md

Track upstream subsystem, disposition (`KEEP`, `EXTEND`, `REPLACE`, `DEFER`), rationale, fork delta, and merge/rebase implications.

## CAM_VALIDATION.md

Use durable IDs for test/validation scenarios so discussions can refer to them unambiguously.
