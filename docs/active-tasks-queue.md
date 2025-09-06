# WIP
Current idea: maintain per-tick queues and schedule work aligned with epochs.

- Tick source: By default, a “tick” is an epoch number from [global-queue](./global-queue.md). Alternative tick sources could be added later.
- Scheduling: When a new epoch arrives, tasks targeting that tick are enqueued; the engine drains the current tick’s queue to completion, then advances.
- Future tasks: Tasks may target future ticks (e.g., t10), and will wait until that logical time is reached. Starvation prevention and fairness policies are TBA.
    - Scheduled tasks: for problems such as the distributed task queue with dependencies, and to support sleep()-s and timeouts overall, another [priority] queue that the system maintains are that futures should be resolved automatically at the beginning of processing which future ticks.

Example (logical time ticks t1, t2, t10):

|  t1  |  t2  |  t10 |
|------|------|------|
| task1| task2| task3|
| task5|      | task4|
| task6|      |      |

- `t1` is a current time and this queue is in execution right now.
- `task3` and `task4` are scheduled way after the current time; it will not run until epoch t10 is committed.