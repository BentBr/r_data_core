# @rdatacore/mcp-server

The [RDataCore](https://github.com/BentBr/r_data_core) MCP server, so an AI
assistant can author, run and debug your data workflows.

```bash
npx @rdatacore/mcp-server
```

Installing downloads the binaries for your platform from the GitHub release
that matches **this package's version** (`mcp-v<version>`) — never "the
newest release" — and verifies each one against the `SHA256SUMS` shipped
inside this package before making it executable. A mismatch, a missing
checksum, or an unsupported platform fails the install with a message rather
than leaving a command that breaks later. Supported platforms: Linux x64 and
arm64, macOS x64 and arm64, Windows x64.

## Local use (stdio)

One person, one API key. Everything the assistant does happens as the owner of
that key.

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

## Hosted use (HTTP + OAuth)

Several people, each acting as themselves. The server holds no credential of
its own: every request carries a token belonging to the person who made it,
exchanged for a short-lived RDataCore token for that same person.

```bash
RDC_MCP_TRANSPORT=http \
RDC_BASE_URL=https://rdatacore.example.com \
RDC_MCP_BIND=0.0.0.0:8931 \
RDC_MCP_RESOURCE_URL=https://mcp.example.com \
RDC_OIDC_ISSUER=https://auth.example.com \
RDC_OIDC_AUDIENCE=rdc-mcp \
  npx @rdatacore/mcp-server
```

A client then needs only the URL — it reads
`https://mcp.example.com/.well-known/oauth-protected-resource` and configures
itself.

Setting `RDC_API_KEY` alongside HTTP transport is **refused at startup**: one
shared key would authenticate every caller as the same account and collapse
the per-user permissions the design rests on.

## Environment

| Variable | Purpose |
|---|---|
| `RDC_BASE_URL` | Your RDataCore instance. Required. |
| `RDC_API_KEY` | Credential for stdio mode. Required there, refused over HTTP. |
| `RDC_MCP_TRANSPORT` | `stdio` (default) or `http`. |
| `RDC_MCP_BIND` | Listen address for HTTP mode. Defaults to `127.0.0.1:8931`. |
| `RDC_MCP_RESOURCE_URL` | This server's public URL, for OAuth discovery. Required for HTTP. |
| `RDC_OIDC_ISSUER` / `RDC_OIDC_AUDIENCE` | The provider to trust, and the audience to require. Required for HTTP. |
| `RDC_MCP_ALLOWED_ORIGINS` | Browser origins permitted to reach the server. Defaults to the resource URL's origin. |
| `RDC_MCP_TIMEOUT_SECS` | Outbound HTTP timeout. Defaults to 30. |
| `RDC_MCP_HEADER_TIMEOUT_SECS` | HTTP mode: seconds a client gets to finish its request headers. Defaults to 10. |
| `RDC_MCP_IDLE_TIMEOUT_SECS` | HTTP mode: seconds a connection may stay open in total. Defaults to 120. |
| `RDC_MCP_MAX_CONNECTIONS` | HTTP mode: open connections accepted at once; the rest are refused. Defaults to 1024. |
| `RDATACORE_MCP_VERSION` | Install a specific release tag instead of this package's own version. Normally unnecessary. |
| `GITHUB_TOKEN` | Optional at install time, and only to raise the GitHub API rate limit. The repository is public, so no token is needed. |

Full deployment guide: [`docs/MCP.md`](https://github.com/BentBr/r_data_core/blob/main/docs/MCP.md).
