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

## Releasing

A release is cut by release-please on `main`; its `mcp-tag` job pushes the
matching `mcp-v*` tag with the **release App**'s token, which starts
`mcp-release.yml`. That workflow refuses any tag whose commit is not a
release commit on `main`, or is not the commit the run was started for
(`.github/scripts/check-release-tag.sh`), builds five targets, attests their
provenance, uploads them once (an existing release's assets are never
overwritten), and publishes the npm package. The package carries the
release's `SHA256SUMS` inside the tarball, so an install verifies against
sums covered by npm's provenance rather than by the release it is verifying.

That guard stops a mistaken or misdirected tag. It is not what stops a
malicious one — it constrains which commit is published, not who may start a
release. The boundary is the GitHub configuration below: the tag ruleset, the
release App's key being readable only from `main` (an environment secret of
`release-tag`), and the `release` environment that the publishing jobs run
in being limited to `mcp-v*` tags with required reviewers.

Settings that live on GitHub and npm rather than in this repository have to be
in place for any of this to mean anything. **Both environments, and the two
secrets in `release-tag`, must exist before this change merges.** If
`release-tag` is missing, GitHub creates it on first use with no secrets, and
the next core release fails to tag the MCP (it fails closed, but it fails).

1. **Create the release App** with repository permission `contents: write`
   and nothing else, and install it on this repository. Create the
   environment **`release-tag`**: deployment branches `main` only, **no**
   required reviewers (tagging runs unattended). Store the App's ID and
   private key as **environment secrets of `release-tag`** named
   `RELEASE_APP_ID` and `RELEASE_APP_PRIVATE_KEY` — never as repository
   secrets, which any branch's workflow can read. Only `release-please.yml`'s
   `mcp-tag` job, which declares `environment: release-tag`, may name them
   (`check-app-secret-scope.sh` fails CI otherwise).
2. **Reduce the PR-management App** (`PR_APP_*`, used by `pr-management.yml`
   on every branch push) to `pull-requests: write`, `contents: read` and
   `metadata: read`, so it can no longer push a tag. The workflow also asks
   for only those permissions when it mints the token.
3. **Tag ruleset.** A repository ruleset on `refs/tags/mcp-v*` and
   `refs/tags/v*` restricting creation and update to the release App.
4. **Immutable releases.** Enable them in the repository's release settings,
   so a published asset cannot be replaced even by a maintainer.
5. **`release` environment.** Create it with required reviewers and
   deployment refs limited to tags matching `mcp-v*` (the `publish`, `pages`
   and `npm` jobs use it), and register the npm trusted publisher for
   `@rdatacore/mcp-server` with workflow `mcp-release.yml` **and**
   environment `release`.

`check-app-secret-scope.sh` is a drift detector, not a boundary: it greps for
the name of the key, so it does not catch `secrets: inherit` or
`toJSON(secrets)`. What makes the key unreadable from a branch is step 1's
environment and its `main`-only deployment policy.

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

`.mcp.json` is committed and runs the server **in a container** — there is no
host Rust toolchain to install and nothing to `cargo build`. It references
`${RDC_MCP_API_KEY}` rather than holding a key, because the file is in git.

```bash
# 1. Build the image (once, and after changing the MCP crate)
docker compose --profile mcp build mcp

# 2. Create an API key: admin UI → Settings → API keys
export RDC_MCP_API_KEY=<the key, shown once>
```

Then restart Claude Code in this directory and run `/mcp`; `rdatacore` should
be listed as connected.

Behind that, the client runs:

```bash
docker compose run --rm --no-deps -T mcp-stdio
```

`-T` is what makes it work: stdio mode *is* the protocol stream, so the
client needs a pipe rather than a TTY. `--no-deps` keeps it from starting the
rest of the stack, and `--rm` means no container survives the session. The
`mcp-stdio` service sits behind a `cli-only` profile so `docker compose up`
never starts it — `run` reaches it anyway.

The container talks to `http://rdatacore.docker` over the compose network, so
the API must be up (`docker compose up -d`). It needs neither Keycloak nor the
`mcp` profile: those are for the HTTP transport.

Check it without a client:

```bash
RDC_MCP_API_KEY=wrong docker compose run --rm --no-deps -T mcp-stdio
```

A credential error naming the admin panel means the container reached the API
and the only thing missing is a valid key. `MissingCredential` means the
variable never arrived.

Logs go to stderr by design — a stray line on stdout corrupts the JSON-RPC
stream — and the service sets `RUST_LOG=warn` to keep that quiet.

### Building from source instead

If you would rather run the binary directly:

```bash
cargo build --release -p r_data_core_mcp
```

then point `.mcp.json`'s `command` at `./target/release/r-data-core-mcp` and
set `RDC_BASE_URL` and `RDC_API_KEY` in its `env`.

---

## Several instances at once

One checkout, several RDataCore deployments — local, demo, production. Each
`mcpServers` entry is its own process with its own environment, so run one
per instance and name them for the instance they reach:

```bash
claude mcp add rdatacore-demo --scope user \
    --env RDC_BASE_URL=https://demo.rdatacore.eu \
    --env 'RDC_API_KEY=${RDC_MCP_API_KEY_DEMO}' \
    -- npx -y @rdatacore/mcp-server

claude mcp add rdatacore-prod --scope user \
    --env RDC_BASE_URL=https://production.rdatacore.eu \
    --env 'RDC_API_KEY=${RDC_MCP_API_KEY_PROD}' \
    -- npx -y @rdatacore/mcp-server
```

Single quotes matter: the `${...}` has to reach the config file unexpanded,
so the client resolves it per launch. A key written literally there is a key
committed to a dotfile.

`--scope user` puts them in `~/.claude.json`, where they are available in
every directory. Leave the local server in this repository's `.mcp.json`
instead: it is tied to this checkout's compose stack and means nothing
elsewhere.

### The variable names

The binary reads **`RDC_API_KEY`** and nothing else. The `RDC_MCP_*` spelling
is a host-side convention, mapped in the server entry the same way
`compose.yaml` maps it for the container:

| Instance | Shell variable | Mapped to |
|---|---|---|
| local (compose) | `RDC_MCP_API_KEY` | `RDC_API_KEY`, by `compose.yaml` |
| demo | `RDC_MCP_API_KEY_DEMO` | `RDC_API_KEY`, by the server entry |
| production | `RDC_MCP_API_KEY_PROD` | `RDC_API_KEY`, by the server entry |

So keep exporting `RDC_MCP_*` and let the mapping do the renaming:

```bash
# ~/.zshrc -- then restart the client so it inherits them
export RDC_MCP_API_KEY='...'        # local
export RDC_MCP_API_KEY_DEMO='...'   # demo
export RDC_MCP_API_KEY_PROD='...'   # production
```

A key belongs to one instance; there is no shared credential across
deployments.

### Knowing which instance you are talking to

The assistant names the server when it calls a tool, and that name is the
only signal distinguishing demo from production — identical tools, identical
arguments, different data. Hence `rdatacore-prod` rather than `rdatacore2`.

Everything the assistant does happens as the owner of that key, with no
confirmation step inside the MCP layer. Give the production key the
narrowest role that is useful; a read-only key is a reasonable default, and
an assistant that can author workflows there can also run them. When only
one instance should be reachable in a session, remove the others rather than
relying on care:

```bash
claude mcp remove rdatacore-prod --scope user
```

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

## For a customer's users

A customer runs RDataCore in their own network and their people want to reach
it from Claude or ChatGPT. Nothing above fits them: they will not clone a
repository, run a container, or paste an API key into a JSON file.

### What gets deployed, and where

**The MCP server is part of the customer's installation, not a service we
host.** It runs beside their RDataCore, behind their TLS, on their hostname,
against their identity provider. There is nothing for their users to download
and no address of ours for them to point at.

It has to be theirs, because it is theirs that it talks to: the server
validates every token against `RDC_OIDC_ISSUER` — the customer's provider —
and then exchanges it at the customer's own RDataCore. A server we hosted
would have neither, and could only fall back to one shared key, which is the
thing this design exists to avoid.

The npm package and the published binaries are for developers running stdio
on their own machine. A customer's users never meet them.

### The flow, once it is deployed

```
user's browser          Claude                MCP server            RDataCore
     |                     |                       |                     |
     |                     |--- GET /mcp --------->|                     |
     |                     |<-- 401 + challenge ---|                     |
     |                     |--- GET metadata ----->|                     |
     |                     |<-- authz server ------|                     |
     |<-- sign in (PKCE) --|                       |                     |
     |--- work account --->| (customer's IdP)      |                     |
     |                     |--- /mcp + bearer ---->|                     |
     |                     |                       |-- verify (JWKS)     |
     |                     |                       |-- POST exchange --->|
     |                     |                       |<-- token for *them* |
     |                     |<-- result ------------|--- API as them ---->|
```

The exchange is the point of the whole arrangement. The provider token is
never forwarded to RDataCore as-is — the MCP authorization specification
names that pattern and disallows it. What comes back is a short-lived token
RDataCore minted for that same person, carrying the roles their IdP groups
map to through `RDC_OIDC_ROLE_MAP`. No service account exists anywhere along
the path, and nothing long-lived is stored.

### What has to be reachable

**The assistant's servers open the connection, not the user's machine.** So
`https://mcp.customer.example/mcp` must resolve and answer from the public
internet. For an instance deployed inside a corporate network this is the
only hard part of the whole exercise, and it is worth raising with the
customer early rather than late.

Two things make it a smaller ask than it first sounds:

- **Only the MCP endpoint is exposed.** RDataCore itself stays internal; the
  MCP server reaches it over the internal network.
- **The exposed component stores nothing worth stealing.** It validates
  tokens and forwards none of them. Its unauthenticated surface is a
  discovery document and a health check.

### When the network forbids it

Some customers will not expose anything, and then remote connectors are out —
for ChatGPT as much as for Claude, since both connect the same way.

The fallback is per-user stdio. It runs on the user's own machine, which is
already inside the network, so it reaches the internal instance without
anything being published. The price is real and should be stated rather than
discovered:

| | remote connector | per-user stdio |
|---|---|---|
| Needs a public endpoint | yes | no |
| User installs something | no | yes |
| What each person holds | nothing; a token per request | a long-lived API key |
| Acts as | the person | the key's owner |
| Removing one person | disable them at the provider | rotate the key for everyone |
| "Who changed that workflow?" | answerable | not answerable |

### What the customer's administrator configures

The server ships as a container image, published on the core release cycle
alongside the API, worker and maintenance images:

```
ghcr.io/bentbr/r-data-core-mcp:0.4.18
```

Multi-arch (`linux/amd64`, `linux/arm64`), so it runs on a Graviton node as
readily as an Intel one. Its entrypoint is the HTTP transport; the stdio
binary ships in the same image for anyone who wants to exec it. `:latest`
tracks the newest release, but pin the version in anything you deploy — the
tag the rest of the stack uses is the tag this should use.

Nothing is built on the customer's side, and nothing is downloaded by their
users. For Kubernetes it is an ordinary `Deployment` plus a `Service`: one
replica is enough to start with, the process is stateless, and `/health`
answers unauthenticated for a liveness probe.

Configure it with the same values as **Hosted (HTTP + OAuth)** above:

```bash
RDC_MCP_TRANSPORT=http
RDC_BASE_URL=https://rdatacore.internal.customer.example
RDC_MCP_BIND=0.0.0.0:8931
RDC_MCP_RESOURCE_URL=https://mcp.customer.example
RDC_OIDC_ISSUER=https://id.customer.example/realms/main
RDC_OIDC_AUDIENCE=https://mcp.customer.example
RDC_MCP_ALLOWED_ORIGINS=https://claude.ai
```

`RDC_MCP_BIND` must be `0.0.0.0` in a container: the default binds loopback,
which inside a pod means nothing outside it can connect, including the
ingress.

`RDC_OIDC_AUDIENCE` must equal `RDC_MCP_RESOURCE_URL`, and RDataCore must
trust the same issuer ([SSO.md](./SSO.md)). If the two disagree, a client that
does everything correctly is rejected with a 401 explaining nothing; the
server warns at startup, so read its first lines after any change here.

Route the whole host to the container, not just `/mcp`. The discovery
document sits at the root, and an ingress rule matching only `/mcp` leaves
`/.well-known/oauth-protected-resource` answering with the ingress's own 404
— at which point clients cannot discover the authorization server and fail
with nothing useful to show for it.

Their provider also has to let the assistant register as a public client with
PKCE. Providers supporting dynamic client registration handle that on their
own; the rest need the client created by hand, with the redirect URI the
assistant documents.

### What the end user does

> 1. Open Claude → Settings → Connectors → *Add custom connector*.
> 2. Paste the URL your administrator gave you, ending in `/mcp`.
> 3. Sign in with your usual work account when the browser opens.
>
> If sign-in is refused, your account is probably not in the RDataCore group
> at your identity provider yet. Ask an administrator — retrying will not
> help.

Nothing is issued to them and nothing is installed, and the server is
upgraded centrally by whoever runs the instance. Availability of custom
connectors varies by Claude plan, so confirm that before promising it to a
team.

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
a developer machine. The port is bound to loopback only; other machines on the
network cannot reach it, which is the condition under which plaintext is
acceptable.

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
| `RDC_MCP_HEADER_TIMEOUT_SECS` | no | `10` | Seconds a client gets to finish its request headers |
| `RDC_MCP_IDLE_TIMEOUT_SECS` | no | `120` | Seconds a connection may stay open in total; at the limit it finishes the request in flight (up to 5 s more) and closes |
| `RDC_MCP_MAX_CONNECTIONS` | no | `1024` | Open connections accepted at once; the rest are refused |

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
