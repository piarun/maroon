# Repository Guidelines

## Repo Map
- Top-level crates: 
    - `maroon` core node
    - `gateway` entry point for external clients
    - `runtime` fiber runtime, executes Maroon assembler 
    - `dsl` Maroon IR, Maroon assembler codegen, DSL prototype
    - `generated` auto-generated Maroon assembler; do not edit
    - `protocol` lib-p2p based types and functions for maroon-gateway communication
    - `types` shared core domain types
    - `common` libraries without specified domain
    - `epoch_coordinator` logic of emitting epochs(etcd based)
    - `tests/integration` local-runnable integration tests
    - `metrics` OTLP/Prometheus/Grafana local stacks
    - `docs` high-level description of key components and principles
    - `scripts` scripts for simplifying local development/running test scenarios
    - PoC
        - `cpp_ir/rust` preprocessor-based DSL and toolchain to prototype Maroon IR
        - `state` Maroon assembler PoC
    - experimental maroon-gateway work visualisation
        - `schema` events definition   
        - `util` html+js visualisation
        - `state_log` redis storage

## Per-Crate Guides & deep create maps
- `maroon/AGENTS.md`
- `runtime/AGENTS.md`

## Commit & Pull Request Guidelines
- Commit messages: imperative mood, concise summary, optional scope (e.g., "gateway: add request validation").
- PRs: clear description, linked issues, steps to test, and notes on metrics/config changes. Update `docs/` when behavior changes.
- before pushing run: 
    - `make test`
    - `make fmt`
    - `make integtest`
    - `make integtest-dockerized`

## Local cli agent development
- run `make test` for a quick verification
- before reporting finished task run:
    - `make test`
    - `make fmt`
    - `make integtest`
    - `make integtest-dockerized`
- after task is finished - update `docs/` if the change is significant and should be reflected
