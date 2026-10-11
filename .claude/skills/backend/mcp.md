# MCP crate (`crates/mcp`)

An HTTP client of the RDataCore admin API, exposed to AI assistants over the
Model Context Protocol. Package `r_data_core_mcp`; binary `r-data-core-mcp`
(kebab-case, matching the other binaries).

## Layering

Depends on `core` (shared DTOs, OIDC token validation), `workflow` (request
types and DSL types) and `oidc-http` (discovery + JWKS over HTTP), and
**nothing else**. It must never reach `api`, `persistence` or `services`:
it is a client of the running server, not a part of it. `tests/architecture.rs`
enforces this.

## The three invariants

**No deletion.** There is no `delete_workflow`, no entity deletion, no
definition deletion. The capability is absent from the binary rather than
guarded by a check a model could argue with, and a test asserts no tool name
contains `delete` or `remove`.

**Never acts as anyone but the caller.** No service account, no elevated
fallback. `AuthBackend::outbound_headers` produces a credential belonging to
the caller or fails; a 403 from RDataCore is authoritative. Configuration
refuses HTTP transport with a static key for this reason — one shared
credential over a network-facing server collapses per-user permissions.

**Never teaches from `docs/DSL.md`.** The reference served to models is
rendered from the live `/dsl/*/options` catalogue, which is generated from the
same structs the executor consumes. The markdown has drifted before and will
again.

## Layout

| Path | Role |
|---|---|
| `config.rs`, `config/` | Environment parsing (`RDC_BASE_URL`, `RDC_API_KEY`, `RDC_MCP_TRANSPORT`, `RDC_MCP_BIND`, `RDC_MCP_TIMEOUT_SECS`, `RDC_MCP_RESOURCE_URL`, `RDC_MCP_ALLOWED_HOSTS`, `RDC_MCP_ALLOWED_ORIGINS`, `RDC_MCP_HEADER_TIMEOUT_SECS`, `RDC_MCP_IDLE_TIMEOUT_SECS`, `RDC_MCP_MAX_CONNECTIONS`, `RDC_OIDC_*`), and the combinations it refuses; `credential.rs` (static key vs OIDC), `numbers.rs` (positive-integer parsing), `limits.rs` (HTTP server limits) |
| `auth/` | `AuthBackend` trait, `CallerContext`, `Permissions`; `api_key` (stdio: key to local token), `oidc` (HTTP: caller's bearer to local token, per-caller cache), `metadata` (RFC 9728 protected-resource metadata) |
| `client/` | Typed calls: `workflows`, `dsl`, `entities`, plus `error` |
| `tools/` | `discover`, `author`, `execute`, `system`; registry and permission filtering (`handler.rs`) |
| `resources/` | Renders the DSL catalogue for models |
| `prompts/` | The authoring and debugging sequences |
| `http.rs`, `http/` | The streamable-HTTP transport: `server` (+ `server/lifetime`, connection lifetime and limits), `routes`, `guards` (bearer validation, Host/Origin checks) |
| `bin/stdio.rs` | The stdio server (API-key backend) |
| `bin/serve.rs` | The HTTP server (OIDC backend) |

The HTTP transport takes a per-request identity from the OIDC backend: the
caller's bearer token is validated, exchanged for a local token and cached per
caller. Static-key configuration is refused for it.

## Things that will bite you

**`schemars` must be a direct dependency**, pinned to the version `rmcp`
resolves. The `JsonSchema` derive expands to absolute `::schemars::` paths, so
taking it from `rmcp`'s re-export does not compile, and two versions in the
graph produce derive errors naming neither.

**`rmcp` is pinned exactly.** Macro and handler shapes differ between majors.
The content type is `ContentBlock`, not `Content`; the prompt role is `Role`,
not `PromptMessageRole`; `ServerInfo` and `Implementation` are
`#[non_exhaustive]` and must be built by mutation.

**Tool failures are `Ok(CallToolResult::error(..))`, never `Err(ErrorData)`.**
MCP renders the latter opaquely, which discards the message written to help a
model correct itself. Reserve `Err` for genuine protocol faults.

**Permission strings must match the server exactly** — namespaces from
`ResourceNamespace::as_str`, actions lowercase from
`generate_workflow_permissions`. Running a workflow needs `workflows:execute`,
not `update`. A typo hides a tool from everyone and produces no error anywhere,
so the vocabularies are pinned by test.

**Tool bodies never construct a `CallerContext`.** They go through
`self.caller()`, so the HTTP transport supplies a per-request identity without
touching a single tool.

## Checks

```bash
cargo test -p r_data_core_mcp
cargo +nightly clippy -p r_data_core_mcp --all-targets --all-features -- \
  -D clippy::all -D warnings -D clippy::pedantic -D clippy::nursery
```

Workspace CI already covers the crate through `--workspace`; no per-crate job
is needed. Integration tests against a real server live in `tests/mcp/`.
