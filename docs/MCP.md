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

## Hosted (HTTP + OAuth)

The server is an OAuth 2.1 **resource server**. A client that knows only its
URL reads the protected-resource metadata, discovers the authorization server,
obtains a token, and connects — no manual configuration.

### 1. Register a client at your identity provider

The *assistant* is the OAuth client, not this server. Register a public client
with PKCE and whatever redirect URI your assistant documents.

### 2. Decide the audience

The MCP server requires `RDC_OIDC_AUDIENCE` and refuses to start without it.
Without an audience check, any token from that issuer would be accepted —
including one minted for an unrelated service that happens to share the
provider.

The MCP audience **may differ** from RDataCore's own. It usually should: they
are two resources, and a token for one should not be a token for the other.
The exchange endpoint accepts a token for the MCP audience and answers with a
token for RDataCore, which is what keeps them separable.

### 3. Configure and run

```bash
RDC_MCP_TRANSPORT=http
RDC_BASE_URL=https://rdatacore.example.com
RDC_MCP_BIND=0.0.0.0:8931
RDC_MCP_RESOURCE_URL=https://mcp.example.com
RDC_OIDC_ISSUER=https://auth.example.com
RDC_OIDC_AUDIENCE=rdc-mcp
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

## Environment

| Variable | Required | Default | Purpose |
|---|---|---|---|
| `RDC_BASE_URL` | yes | — | Your RDataCore instance |
| `RDC_MCP_TRANSPORT` | no | `stdio` | `stdio` or `http` |
| `RDC_API_KEY` | stdio only | — | The credential stdio mode acts as. **Refused with HTTP transport.** |
| `RDC_MCP_BIND` | no | `127.0.0.1:8931` | Listen address. The loopback default means an unconfigured server is not reachable from the network. |
| `RDC_MCP_RESOURCE_URL` | http | — | This server's public URL. Must be https outside localhost. |
| `RDC_OIDC_ISSUER` | http | — | The provider to trust |
| `RDC_OIDC_AUDIENCE` | with issuer | — | The audience to require |
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
