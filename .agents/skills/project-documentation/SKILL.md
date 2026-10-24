---
name: project-documentation
description: Update the ivaCAM fork's roadmap, STATUS, DEVLOG, architecture docs, ADRs, validation records, or milestone handoff documentation.
---

# Project documentation

Use this skill for persistent project documentation and architectural decision records.

Documentation is part of the deliverable, but keep each document optimized for its purpose rather than duplicating the same narrative everywhere.

## Document roles

- `STATUS.md`: current state and next action.
- `DEVLOG.md`: chronological engineering history.
- `ROADMAP.md`: milestones, scope, acceptance criteria.
- `ARCHITECTURE.md`: stable architecture relevant to the fork.
- `UPSTREAM_ANALYSIS.md`: what comes from ivaCAM and fork delta.
- `CAM_VALIDATION.md`: validation cases and status.
- `FUTURE_PROCESSES.md`: explicitly deferred process requirements such as ProtoMAX.
- `adr/`: significant architectural decisions.

Read `references/document-contract.md` before reorganizing project docs.
Use `assets/ADR-TEMPLATE.md` for new ADRs.

## Rules

Record verified facts as facts, mark hypotheses or unknowns clearly, prefer file/module references for technical claims, keep STATUS short, log meaningful decisions rather than terminal noise, and update docs in the same change as the behavior they describe.

## Milestone handoff

At a hard stop, ensure persistent docs contain enough context that a fresh Codex session can resume without relying on chat history.
