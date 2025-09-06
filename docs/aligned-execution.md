# Aligned Execution

Definition: All correct nodes apply the exact same transactions, in the exact same order, at the exact same logical time (epochs), resulting in identical state without any additional coordination beyond observing epochs.

Assumptions

- Crash-fault model. Network may drop/reorder/duplicate; epochs are durably stored.
- Deterministic application: Transitions depend only on transaction input and epoch context.
- Idempotency: Each transaction has a unique `UniqueU64BlobId` and can be re-seen without changing the result.
- MUST avoid nondeterminism in state transitions: no wall-clock reads, random numbers, or external side effects unless it goes through [global-queue](./global-queue.md).