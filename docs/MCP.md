# MCP server

RDataCore ships an [MCP](https://modelcontextprotocol.io) server, so an AI
assistant can author, run and debug your data workflows — the part of the
system with the most options and the most tedium.

There are two deployment modes, and the difference between them is not
performance. It is *who the server acts as*.

| | stdio | HTTP + OAuth |
|---|---|---|
| Runs | On the user's machine | On a server |
| Authenticates with | One static API key | Each caller's own identity |
| Acts as | The key's owner, always | Whoever made the call |
| Suitable for | One person | A team |

**stdio is a single-user mode.** Everything the assistant does happens as the
owner of the configured key, and the audit trail says so. That is fine on your
own laptop with your own key.

The key is not sent to the admin API — that requires a JWT. The server
presents the key once at `/admin/api/v1/auth/api-key/token` and uses the
short-lived admin token it receives, which carries the key owner's roles and
nothing more. Revoking the key stops the next exchange, so access ends within
the token's lifetime rather than whenever a refresh token would have expired.

**HTTP mode holds no credential of its own.** Each request carries its
caller's token, which the server exchanges for a short-lived RDataCore token
minted for that same person. It can therefore never exceed what that person
may do, and the audit trail names them. Setting `RDC_API_KEY` alongside HTTP
transport is refused at startup for exactly this reason: one shared key would
authenticate every caller as the same account and collapse the whole model.

---

## Installing

Prebuilt binaries are published for five platforms:

| Platform | Target |
|---|---|
| Linux x86-64 | `x86_64-unknown-linux-gnu` |
| Linux arm64 | `aarch64-unknown-linux-gnu` |
| macOS Intel | `x86_64-apple-darwin` |
| macOS Apple silicon | `aarch64-apple-darwin` |
| Windows x86-64 | `x86_64-pc-windows-msvc` |

Anything else builds from source:

```bash
cargo build --release -p r_data_core_mcp
```

The installer refuses an unsupported platform rather than downloading a
binary that cannot run, and names the triple it looked for.

### Linking a download

Every release publishes a manifest describing itself, at a fixed address:

```
https://bentbr.github.io/r_data_core/mcp-latest.json
```

```json
{
  "version": "0.1.0",
  "tag": "mcp-v0.1.0",
  "published_at": "...",
  "npm": "@rdatacore/mcp-server",
  "downloads": [
    { "asset": "...", "sha256": "...", "url": "..." }
  ]
}
```

Link that from a download page rather than a release URL. Release URLs carry
the tag, so they go stale at the next release — and `releases/latest` in this
repository resolves to the *core* release, which carries none of these
assets. The same manifest is attached to each release as `mcp-latest.json`,
so a given version's asset list stays verifiable after it is superseded.

---

## Local (stdio)

1. Create an API key in RDataCore: **Settings → API keys**.
2. Add the server to your assistant's configuration:

```json
{
    "mcpServers": {
        "rdatacore": {
            "command": "npx",
            "args": ["-y", "@rdatacore/mcp-server"],
            "env": {
                "RDC_BASE_URL": "https://rdatacore.example.com",
                "RDC_API_KEY": "your-api-key"
            }
        }
    }
}
```

That is the whole setup. The server resolves the key's permissions at startup
and offers only the tools that key can actually use, so the assistant is not
led into a wall of 403s.

---

## Using it from Claude Code

`.mcp.json` is committed, so there is nothing to write. It points at a
locally built binary and reads the key from your environment — the file holds
a `${RDC_MCP_API_KEY}` reference, never a literal key, because it is in git.

```bash
cargo build --release -p r_data_core_mcp
export RDC_MCP_API_KEY=<a key from Settings → API keys>
```

Then restart Claude Code in this directory and run `/mcp`; `rdatacore` should
be listed as connected.

This is the stdio transport, so it needs neither Keycloak nor the `mcp`
compose profile — only the API reachable at `RDC_BASE_URL`. The key is
exchanged for a short-lived admin token at `/admin/api/v1/auth/api-key/token`,
so the key's owner needs the workflow permissions you expect the assistant to
use; the server resolves them at startup and offers only those tools.

---

## Hosted (HTTP + OAuth)

The server is an OAuth 2.1 **resource server**. A client that knows only its
URL reads the protected-resource metadata, discovers the authorization server,
obtains a token, and connects — no manual configuration.

### 1. Register a client at your identity provider

The *assistant* is the OAuth client, not this server. Register a public client
with PKCE and whatever redirect URI your assistant documents.

### 2. Set the audience

The MCP server requires `RDC_OIDC_AUDIENCE` and refuses to start without it.
Without an audience check, any token from that issuer would be accepted —
including one minted for an unrelated service that happens to share the
provider.

**Use the same audience as RDataCore, and set it to this server's resource
URL.** Both halves matter:

- *The same as RDataCore's* because an MCP caller is an RDataCore
  administrator — the server exists to manage an instance on behalf of a
  privileged user, and the exchange endpoint validates the presented token
  against RDataCore's own `RDC_OIDC_AUDIENCE`. A separate MCP audience would
  simply be refused there.
- *Equal to `RDC_MCP_RESOURCE_URL`* because the protected-resource metadata
  advertises that URL as the resource identifier. A client following the
  specification asks its provider for a token scoped to what was advertised;
  if this server then checks a different audience, the client does everything
  right and is rejected with a 401 that explains nothing. The server logs a
  warning at startup when the two differ.

### 3. Configure and run

```bash
RDC_MCP_TRANSPORT=http
RDC_BASE_URL=https://rdatacore.example.com
RDC_MCP_BIND=0.0.0.0:8931
RDC_MCP_RESOURCE_URL=https://mcp.example.com
RDC_OIDC_ISSUER=https://auth.example.com
RDC_OIDC_AUDIENCE=https://mcp.example.com
RDC_MCP_ALLOWED_ORIGINS=https://claude.ai
```

`RDC_MCP_RESOURCE_URL` is this server's own public URL, and it must be the one
clients reach — it goes into the discovery document and the authentication
challenge verbatim. It must be `https` outside localhost: bearer tokens must
not cross plaintext, and the server refuses to start otherwise.

RDataCore itself must trust the same issuer, so that the exchange works. See
[SSO.md](./SSO.md).

### 4. Deploy behind TLS

Terminate TLS at your proxy and forward to `RDC_MCP_BIND`. Endpoints:

| Path | Auth | Purpose |
|---|---|---|
| `/.well-known/oauth-protected-resource` | none | Discovery. Must stay unauthenticated — a client cannot authenticate until it has read this. |
| `/mcp` | bearer | The MCP endpoint. |
| `/health` | none | Liveness. |

### 5. Point the assistant at the URL

`https://mcp.example.com/mcp`. It should discover everything else.

---

## Running the HTTP transport locally

Set `COMPOSE_PROFILES=mcp` in `.env` and bring the stack up:

```bash
docker compose up -d
```

That adds Keycloak, the two seed jobs and the MCP server. The server is
published on `http://localhost:8931` — loopback rather than a
`.docker` hostname behind the proxy, because the resource URL must be https
or loopback and the server refuses to start otherwise. Bearer tokens must not
cross plaintext; localhost is the one exemption, and it is the right one for
a developer machine.

Check it answers:

```bash
curl -fsS http://localhost:8931/.well-known/oauth-protected-resource
```

You should see `"resource": "http://localhost:8931"` and an
`authorization_servers` entry naming the local realm. An unauthenticated call
to the endpoint itself should be refused with a challenge, not a 500:

```bash
curl -sS -i -X POST http://localhost:8931/mcp \
  -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | head -20
```

The realm's `rdc-mcp` client mints `http://localhost:8931` as the audience,
matching the resource URL. If you change the published port, change both or
every token is refused with a 401 that says nothing useful.

This is the hosted transport only. A local MCP client uses stdio, runs the
binary itself, and needs none of this.

---

## What the assistant gets

### Tools

Eighteen, filtered by what the caller may actually do. There is no delete
tool — the capability does not exist in this binary, so no amount of
prompting reaches it.

| Group | Tools |
|---|---|
| Instance | `system_info` |
| Discover | `list_workflows`, `get_workflow`, `list_entity_definitions`, `get_entity_definition`, `query_entities`, `dsl_options` |
| Author | `validate_dsl`, `create_workflow`, `update_workflow` |
| Versions | `list_workflow_versions`, `get_workflow_version`, `restore_workflow_version` |
| Execute | `test_workflow`, `run_workflow`, `list_runs`, `get_run_logs` |
| Schedule | `preview_cron` |

`system_info` reports which instance the server is pointed at, its version,
its enabled features, and what the current caller may do. It is the one tool
offered to every caller regardless of permissions: an assistant that cannot
tell which deployment it is connected to will write to the wrong one quite
confidently. The deployed component versions inside its response need
`system:read` and are omitted without it.

### Resources

| URI | Contents |
|---|---|
| `rdatacore://dsl/reference` | Every DSL type this instance accepts, with fields, enumerated values and worked examples |
| `rdatacore://dsl/validation-rules` | Ordering and typing constraints the catalogue cannot express |

The reference is generated per-instance from the live `/dsl/*/options`
catalogue, **not** from `docs/DSL.md`. That distinction is the point: the
markdown documents roughly a third of the language, so an assistant taught
from it would believe most of the DSL does not exist. The catalogue is built
from the same structs the executor consumes and therefore cannot drift.

Because it is generated on read, it costs three requests to the instance. A
client typically fetches it once per session.

---

## Environment

| Variable | Required | Default | Purpose |
|---|---|---|---|
| `RDC_BASE_URL` | yes | — | Your RDataCore instance |
| `RDC_MCP_TRANSPORT` | no | `stdio` | `stdio` or `http` |
| `RDC_API_KEY` | stdio only | — | The credential stdio mode acts as. **Refused with HTTP transport.** |
| `RDC_MCP_BIND` | no | `127.0.0.1:8931` | Listen address. The loopback default means an unconfigured server is not reachable from the network. |
| `RDC_MCP_RESOURCE_URL` | http | — | This server's public URL. Must be https outside localhost. |
| `RDC_OIDC_ISSUER` | http | — | The provider to trust |
| `RDC_OIDC_AUDIENCE` | with issuer | — | The audience to require. Set it to `RDC_MCP_RESOURCE_URL`, and to the same value RDataCore uses |
| `RDC_MCP_ALLOWED_ORIGINS` | no | the resource URL's origin | Browser origins permitted to connect |
| `RDC_MCP_ALLOWED_HOSTS` | no | the resource URL's authority | `Host` values permitted |
| `RDC_MCP_TIMEOUT_SECS` | no | `30` | Outbound HTTP timeout |

Configuration refuses unsafe combinations at startup rather than at first use:
stdio without a credential, HTTP without an issuer, HTTP with a static key, an
issuer without an audience, and a plaintext resource URL that is not localhost.

---

## Security notes

**No standing credential in HTTP mode.** There is no service account and no
"if the bearer is missing, use the configured key" path. With no valid caller
the server produces no outbound credential at all. That is what makes a 403
from RDataCore authoritative: no bug in the MCP server can grant more than the
human on whose behalf it is acting.

**The caller's token is never forwarded.** It is exchanged. Passing a received
token to a downstream service is the pattern the MCP authorization
specification names and disallows — and it would also require multi-audience
tokens, which Keycloak issues easily and Entra and Auth0 largely do not.

**Sessions are off.** A session outliving one caller's token would mean
whoever obtained the session id could act as the caller who opened it. The
server runs statelessly instead, which costs a cached permission lookup per
request.

**Origin is validated before anything else**, including the unauthenticated
endpoints. This is the DNS-rebinding protection the specification requires,
and discovery metadata is exactly what such an attack would want to read.
Requests carrying no `Origin` pass, because non-browser clients send none.

**There is no delete tool.** Not a guarded one, not a disabled one — the
capability does not exist in the binary, so no amount of prompting reaches it.

**An API key reaches the admin API only through one door.** The admin API
requires a JWT. `POST /admin/api/v1/auth/api-key/token` is the single endpoint
that accepts a key, and it answers with a short-lived token carrying the key
owner's roles. Every other admin route still requires a JWT, so the key's
reach is one explicit, auditable exchange rather than a second authentication
path spread across every handler.

**Tool filtering is ergonomics, not security.** Hiding `create_workflow` from
a read-only caller stops the assistant walking into a 403 it could not have
predicted. RDataCore performs the real check on every call, and a tool invoked
despite being hidden still fails there.

---

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| Startup: "http transport requires `RDC_OIDC_ISSUER`" | HTTP mode with no identity provider would serve every caller unauthenticated. Use stdio for single-user access. |
| Startup: shared credential over HTTP | Remove `RDC_API_KEY`. See the table above. |
| Startup: resource URL must use https | Bearer tokens must not cross plaintext. localhost is exempt for development. |
| Client cannot discover auth | `RDC_MCP_RESOURCE_URL` does not match the URL clients actually reach, so the challenge points somewhere they cannot fetch. |
| 401 with a token that works elsewhere | Wrong audience — the token was minted for a different resource. |
| 403 on the metadata endpoint | The `Origin` header is not in `RDC_MCP_ALLOWED_ORIGINS`. |
| 503 | RDataCore or the identity provider is unreachable. Not a permissions problem. |
| Few tools offered | The caller's permissions are narrow, or the permission lookup failed — the server offers nothing rather than everything in that case, deliberately. Check the server log. |
