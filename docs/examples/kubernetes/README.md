# Kubernetes + Longhorn — Production-Flavored Example

> **⚠️ Example only — not production-ready as-is.** No warranty or guarantee. Review every manifest. You are responsible for secrets management, NetworkPolicies, TLS, backup, resource tuning, and RBAC.

This example deploys the full RDataCore backend on Kubernetes with Longhorn-backed persistent storage for PostgreSQL and Redis.

## Prerequisites

- Kubernetes 1.27+
- [Longhorn](https://longhorn.io) installed, with a `StorageClass` named `longhorn` available
- An Ingress controller (the example uses `ingressClassName: nginx`)
- A working `kubectl` context with permission to create Namespaces, StatefulSets, Deployments, Jobs, Services, Secrets, ConfigMaps, Ingresses, PodDisruptionBudgets, and HorizontalPodAutoscalers
- Optional: [cert-manager](https://cert-manager.io) for automatic TLS (the Ingress annotation is commented out by default)
- A valid RDataCore license key

Verify Longhorn is your storage class:

```bash
kubectl get storageclass
# You should see "longhorn" listed
```

If your storage class has a different name, either edit the PVCs in `10-postgres.yaml` and `11-redis.yaml`, or rename your StorageClass.

## What gets deployed

| Manifest | Kind | Notes |
|----------|------|-------|
| `00-namespace.yaml` | Namespace | `rdatacore` |
| `01-configmap.yaml` | ConfigMap | Non-secret env (cron schedules, log levels, etc.) |
| `02-secret.yaml.example` | Secret (template) | Passwords, JWT secret, license key, DSNs |
| `10-postgres.yaml` | StatefulSet + Service | Postgres 18 with Longhorn PVC (20 GiB) |
| `11-redis.yaml` | StatefulSet + Service | Redis 8 with Longhorn PVC (5 GiB), appendonly |
| `20-migrations-job.yaml` | Job | Runs `run_migrations` once |
| `30-core.yaml` | Deployment + Service + PDB + HPA | API, 2 replicas, HPA 2–5 on 70% CPU |
| `31-worker.yaml` | Deployment | Workflow worker, 1 replica (stateless — scale as needed) |
| `32-maintenance.yaml` | Deployment | Cron singleton — `strategy: Recreate` |
| `40-ingress.yaml` | Ingress | nginx-class, TLS placeholder |

## Setup

**1. Create the Secret from the template:**

```bash
cp 02-secret.yaml.example 02-secret.yaml
# Edit 02-secret.yaml: replace the placeholder values with real secrets.
```

> **⚠️ Password coupling:** `DATABASE_URL` in the Secret embeds `POSTGRES_PASSWORD` directly. Any time you rotate the Postgres password you must update **both** `POSTGRES_PASSWORD` and the password portion of `DATABASE_URL` in the same Secret, then re-apply and roll the core/worker/maintenance Deployments. Forgetting one leaves the stack in a broken state. If you dislike this coupling, split `DATABASE_URL` into components (host, port, db, user, password) and compose it at runtime in your own wrapper.

Alternatively, create the Secret imperatively — see the header of `02-secret.yaml.example` for the `kubectl create secret` command. If you use the imperative route, you do NOT apply `02-secret.yaml` in the next step.

**Generate strong secrets:**

```bash
openssl rand -base64 32
```

**2. Adjust `01-configmap.yaml` and `40-ingress.yaml`:**

- Set `FRONTEND_BASE_URL` in `01-configmap.yaml` to your public API URL (e.g. `https://api.example.com`).
- Change `api.example.com` in `40-ingress.yaml` to your real hostname.
- If you use cert-manager, uncomment the `cert-manager.io/cluster-issuer` annotation and point it at your ClusterIssuer.

## Apply order

Apply manifests in filename order so dependencies are satisfied. Wait for Postgres and Redis to become Ready before applying the migrations Job.

```bash
kubectl apply -f 00-namespace.yaml
kubectl apply -f 01-configmap.yaml
kubectl apply -f 02-secret.yaml           # or use the imperative create-secret command

kubectl apply -f 10-postgres.yaml
kubectl apply -f 11-redis.yaml

# Wait for StatefulSets to be Ready
kubectl -n rdatacore rollout status sts/postgres --timeout=5m
kubectl -n rdatacore rollout status sts/redis    --timeout=5m

kubectl apply -f 20-migrations-job.yaml
kubectl -n rdatacore wait --for=condition=complete job/rdatacore-migrate --timeout=5m

kubectl apply -f 30-core.yaml
kubectl apply -f 31-worker.yaml
kubectl apply -f 32-maintenance.yaml
kubectl apply -f 40-ingress.yaml
```

## Verify

```bash
kubectl -n rdatacore get pods
kubectl -n rdatacore logs deploy/rdatacore-core
kubectl -n rdatacore exec -it sts/postgres -- psql -U rdatacore -d rdata -c '\dt'
```

Hit the health endpoint from inside the cluster (bypasses the Ingress):

```bash
kubectl -n rdatacore run curl --rm -it --image=curlimages/curl --restart=Never -- \
  curl -sf http://rdatacore-core.rdatacore.svc.cluster.local:8888/api/v1/health
```

Once your DNS points at the Ingress and TLS is configured:

```bash
curl -sf https://api.example.com/api/v1/health | jq
```

## Upgrade

```bash
# Pull new images (forces a rollout if you use :latest; otherwise bump the tag in each manifest)
kubectl -n rdatacore rollout restart deploy/rdatacore-core deploy/rdatacore-worker deploy/rdatacore-maintenance

# Re-run migrations
kubectl -n rdatacore delete job rdatacore-migrate --ignore-not-found
kubectl apply -f 20-migrations-job.yaml
kubectl -n rdatacore wait --for=condition=complete job/rdatacore-migrate --timeout=5m
```

## Tear down

```bash
kubectl delete namespace rdatacore
```

This deletes all Deployments/StatefulSets/Services. Longhorn PVs are governed by their reclaim policy — verify they are removed or retained as you expect.

## Caveats

- **`:latest` tags drift.** For any real deployment, pin each image to a specific version tag.
- **Single-replica stateful services.** Postgres and Redis run as 1-replica StatefulSets here. For HA, use a managed Postgres service and Redis Sentinel/Cluster — or a Postgres operator such as CloudNativePG.
- **Longhorn storage class name** must be `longhorn` as shipped, or you must edit the PVC specs.
- **CORS_ORIGINS=*** is permissive. Override in `01-configmap.yaml` to restrict to your frontend origins.
- **No NetworkPolicies, no RBAC lockdown.** Add them to suit your cluster's security model.
- **No backup** is configured for the Postgres PVC. Use `pgBackRest`, Velero, or another tool as appropriate.
- **Resource limits** are starting points. Tune based on your workload and node sizing.
- **Maintenance is a Deployment, not a CronJob**, because the maintenance binary owns its schedule. `strategy: Recreate` prevents two replicas running simultaneously during rollouts.
- **Only the core Deployment has a PodDisruptionBudget.** Worker and maintenance run as single replicas (stateless queue consumer and cron singleton respectively), so a PDB would just block node drains. Scale worker horizontally if you need resilience there, and add a PDB at that point.
