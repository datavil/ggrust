# Notes index

Every markdown file outside the root contract files, `memory/`, `docs/` and
`benchmarks/results/` is listed here with a status. `make check-docs` fails when a file
under `notes/` is missing from this index. Root files stay at the root:
[AGENTS.md](../AGENTS.md) (contract), [plan.md](../plan.md) (roadmap),
[deviations.md](../deviations.md), [CHANGELOG.md](../CHANGELOG.md).
Lessons learned: [memory/](../memory/README.md).

Status: **live** = still drives work, **done** = finished, kept for the record,
**archived** = superseded.

| File | What it is | Status |
|---|---|---|
| [backlog.md](backlog.md) | Unscheduled ideas and gaps, merged from the ggplot-rs comparison, the user-asks survey, "consider later" and accessor proposals | live |
| [priorities.md](priorities.md) | Every backlog item ranked from 1.1 (highest) down | live |
| [plans/plan-letsplot-gaps.md](plans/plan-letsplot-gaps.md) | Gap analysis of ggrust vs lets-plot 4.11.0 (2026-10-31); source of plan.md Phase 8 | live |
| [plans/plan-lp5.md](plans/plan-lp5.md) | Detail for the `lp5.N` steps (lets-plot gaps, score 3) | live |
| [plans/extensions.md](plans/extensions.md) | Plan for ggplot2 extension features (19 steps done, 5 open) | live |
| [plans/accessors.md](plans/accessors.md) | Spec for read-only plot accessors; the agreed part is built | done |
| [cellestial/cellestial_workarounds.md](cellestial/cellestial_workarounds.md) | Maps ggrust features to cellestial workarounds they replace; add a row whenever a step removes one | live |
| [audits/dep_port_audit.md](audits/dep_port_audit.md) | Read-only audit of cellestial's third-party dependencies and what ggrust must port (2026-11-08) | done |
| [archive/](archive/) | Superseded docs go here (empty) | |

Benchmarks: [benchmarks/README.md](../benchmarks/README.md) and the results index
[benchmarks/results/README.md](../benchmarks/results/README.md).
