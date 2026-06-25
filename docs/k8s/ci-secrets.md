# CI secrets for external services

The `.github/workflows/e2e-external.yml` workflow spins up MongoDB,
Chroma, and Qdrant as service containers and runs the gated
integration tests against them.

## Gate variable

All jobs are guarded by:

```
vars.RUN_EXTERNAL_E2E == 'true'
```

This is a *repository variable* (not a secret). Set it under
**Settings -> Secrets and variables -> Actions -> Variables**.

When unset, every job short-circuits (`if:` condition is false) and
the workflow does nothing. This keeps the workflow from slowing down
default PR builds while still being available on demand.

## Per-job env vars

Each job sets the URI that the corresponding `loon-persistence`
e2e test consumes. None of these are secrets; they point at the
service container started by the job itself.

| Job       | Env var                | Container image             | Port |
|-----------|------------------------|-----------------------------|------|
| mongodb   | `LOON_TEST_MONGODB_URI`| `mongo:7`                   | 27017|
| chroma    | `LOON_TEST_CHROMA_URI` | `chromadb/chroma:0.4.24`    | 8000 |
| qdrant    | `LOON_TEST_QDRANT_URI` | `qdrant/qdrant:v1.9.0`      | 6334 |

The matching Rust tests:

- `cargo test --test e2e_mongodb` — reads `LOON_TEST_MONGODB_URI`
- `cargo test --workspace chroma` — reads `LOON_TEST_CHROMA_URI`
  (currently no `tests/e2e_chroma.rs`; the workflow is wired so that
  once one is added under `LOON_TEST_CHROMA_URI`, the job runs it)
- `cargo test --workspace qdrant` — reads `LOON_TEST_QDRANT_URI`
  (same situation as chroma)

Both chroma and qdrant jobs exist for forward compatibility; today
they will succeed trivially. As soon as a gated e2e test is added
for either backend the job will exercise it.

## Manual trigger

The workflow also runs on `workflow_dispatch`, so you can trigger it
from the Actions tab without waiting for the weekly cron schedule.

## Adding more jobs

To add another external service (e.g. Postgres), mirror the chroma
job: add a `services:` block, set the env var, and run the gated
test under `RUN_EXTERNAL_E2E`.
