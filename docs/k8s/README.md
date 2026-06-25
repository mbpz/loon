# k8s deployment

Single-replica `loon-server` for Phase 12.

## Prerequisites
- Kubernetes 1.27+
- `kubectl` configured for the target cluster
- Container image `loon-server:latest` built and pushed to a registry reachable by the cluster

## Setup
1. Create the secrets:
   ```bash
   kubectl create secret generic loon-secrets \
     --from-literal=openai-api-key=$OPENAI_API_KEY \
     --from-literal=auth-tokens=$LOON_AUTH_TOKENS
   ```
2. Apply the manifest:
   ```bash
   kubectl apply -f docs/k8s/loon-server-deployment.yaml
   ```
3. Port-forward to test:
   ```bash
   kubectl port-forward svc/loon-server 8800:8800
   curl localhost:8800/health
   ```

## Multi-replica

This manifest uses a single replica. For multi-replica:

- Use a shared `DistributedState` (Redis backend in `loon-persistence`)
  for session/message state. `loon-persistence` exposes this behind
  the `redis` feature flag.
- `livenessProbe`/`readinessProbe` already point at `/health`, which
  can scale to N replicas.
- Sessions must be routed to a specific replica based on a sticky
  session ID (e.g. via a service mesh). The current implementation
  uses in-memory stores; multi-replica requires all replicas to
  share a `DistributedState` backend.

## Out of scope (Phase 12 follow-ups)

- Redis backend integration in `loon-server::run()` (the
  `DistributedState` trait is exposed; wiring is future work)
- Service mesh / sticky session routing
- Horizontal Pod Autoscaler
- PersistentVolumeClaim for the `/data` mount
