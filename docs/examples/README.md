# Example Deployments

> **⚠️ Example only — not production-ready as-is.** These files are starting points for your own deployment. No warranty or guarantee is provided, express or implied. You are responsible for hardening security (secrets management, network policies, TLS, backup strategy, resource tuning, RBAC), and for adapting these examples to your infrastructure. Review every manifest before applying it to any environment.

This directory contains two runnable example deployments of RDataCore:

| Example | Location | Use when |
|---------|----------|----------|
| **Docker Compose** | [`docker-compose/`](./docker-compose/) | Single-host deployments, evaluation, small self-hosted setups |
| **Kubernetes + Longhorn** | [`kubernetes/`](./kubernetes/) | Cluster deployments with Longhorn persistent storage |

Both examples deploy the full backend stack: the API server, workflow worker, maintenance worker, PostgreSQL, and Redis. Neither example includes the admin frontend container, an SMTP server, or development tooling — those are expected to be provided externally in a real environment.

## What's not included (deliberately)

- Helm charts or Kustomize overlays — raw YAML is easier to read and adapt
- SealedSecrets / external-secrets integration — use your org's secret management
- cert-manager manifests — the Ingress has a TLS placeholder you can wire to your own issuer
- NetworkPolicies — scope them to your cluster's network model
- Backup/restore tooling for PostgreSQL — use your preferred backup solution
- Observability stack (metrics, logs, tracing)

## Related docs

- [Root README](../../README.md) — quick start with the in-repo development Docker Compose
- [DEVELOPMENT.md](../DEVELOPMENT.md) — local development environment setup
