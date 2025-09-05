# WIP
Current idea: maintain per-tick queues and schedule work aligned with epochs.

- Tick source: By default, a “tick” is an epoch number from [global-queue](./global-queue.md). Alternative tick sources could be added later.
- Scheduling: When a new epoch arrives, tasks targeting that tick are enqueued; the engine drains the current tick’s queue to completion, then advances.
- Future tasks: Tasks may target future ticks (e.g., t10), and will wait until that logical time is reached. Starvation prevention and fairness policies are TBA.

Example (logical time ticks t1, t2, t10):

|  t1  |  t2  |  t10 |
|------|------|------|
| task1| task2| task3|

`task3` is scheduled way after the current time; it will not run until epoch t10 is committed.
