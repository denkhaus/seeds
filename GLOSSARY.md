# Seeds

Domain of the seeds git-native issue-tracker format: seeds (issues), their
lifecycle, dependencies, plans, and the metrics derived from them.

## Language

**Lead Time**:
The span from a seed's creation to its closure (`createdAt` → `closedAt`),
measured per seed; reported as a median across closed seeds.
_Avoid_: cycle time, duration, age

**Closure Rate**:
The number of seeds closed per unit of time over a defined observation
window.
_Avoid_: velocity, throughput, speed

**Pace**:
The project's demonstrated backlog-drain rate, expressed as closures per
week over a rolling observation window.
_Avoid_: velocity, tempo, momentum

**Production Readiness**:
The target state in which no open seeds remain outside the configured
exclusion list.
_Avoid_: done, finished, release-ready

**ETA (project horizon)**:
The projected point in time when Production Readiness is reached at the
current net Drain Rate (closures minus creations per week); `null` with an
`eta_reason` while the window carries too little data or the backlog is
not shrinking.
_Avoid_: deadline, forecast, completion date

**Pace coefficient (k)**:
The ratio of closures to creations inside the observation window
(`k = closures / creations`). `k = 1` holds the backlog, `k > 1` drains
it, `k < 1` grows it; reported as `null` with `no_creations_in_window`
when the window saw no creations.
_Avoid_: friction score, burn rate, drain deficit

**Drain Rate**:
Net backlog reduction per week: closures minus creations inside the
observation window; the ETA's denominator.
_Avoid_: net velocity, throughput