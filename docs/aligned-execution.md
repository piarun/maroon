Aligned execution means every node applies the same transactions in the same order at the same logical time, requiring no extra synchronization.

- Logical time: The epoch sequence number-tight
    - TODO: need more details on how logcial time is correlated with actual time
- Determinism: Application code must be deterministic with respect to transaction input and epoch context. Avoid randomness, and nondeterministic IO during state transitions unless derived from transaction/epoch data.
