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
exclusion list — scoped: inside a Surface scope, only seeds carrying
that surface's label count toward (or against) it.
_Avoid_: done, finished, release-ready

**Surface**:
A label that classifies a seed by the artifact it touches (`shipping`
= changes the shipped artifact or its runtime behavior; `neutral` =
tests, tooling, docs, unshipped code). The store's `surfaces` config
key declares which labels form the relevance-scoped pace groups.
_Avoid_: category, kind (the record's `type` field is taken)

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

**Threat Mass**:
The sum of priority weights over the seeds created inside the
observation window of one Surface scope — incoming pressure, not
backlog size. Fixed weights: P0=16, P1=8, P2=4, P3=1, P4=0.5; a seed
without a priority weighs as P4.
_Avoid_: severity score, risk points