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
current Pace.
_Avoid_: deadline, forecast, completion date