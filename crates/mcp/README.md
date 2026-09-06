# RDataCore MCP server

Lets an AI assistant author, run and debug RDataCore workflows over the Model
Context Protocol.

Writing a workflow means knowing the DSL's legal shapes, the target entity's
field names, and the runtime type-casting rules — then iterating against real
runs to find the mistakes structural validation cannot catch. This server
exposes exactly those affordances, so that loop becomes a conversation.

## What it can and cannot do

It can read workflows, entity definitions and sample entity data; author and
update workflows; roll back to a previous version; dry-run a program without
side effects; execute one for real; and read run logs.

It **cannot delete anything** — no workflow, entity or definition. The
capability is absent from the binary rather than guarded by a check, so no
amount of prompting reaches it. Entities and entity definitions are read-only:
schema changes and data writes stay in human hands.

It also cannot exceed your own permissions. It holds no credential of its own
and carries yours on every call, so RDataCore's answer is authoritative. Tools
you lack permission for are not advertised, and calls to them are rejected.

## Setup

### 1. Create an API key

In the admin panel under **API Keys**, create a key and assign it the roles the
assistant should act with. The key's permissions are the ceiling on what the
assistant can do — grant it read-only access if you only want help reading.

### 2. Build

```bash
cargo build --release -p r_data_core_mcp
```

The binary is `target/release/r-data-core-mcp`.

### 3. Point your client at it

Claude Code — project `.mcp.json`:

```json
{
  "mcpServers": {
    "rdatacore": {
      "command": "/absolute/path/to/target/release/r-data-core-mcp",
      "env": {
        "RDC_BASE_URL": "https://your-instance.example.com",
        "RDC_API_KEY": "your-api-key"
      }
    }
  }
}
```

### 4. Check it works

Ask: *"List my RDataCore workflows."* You should get them back. If the key
lacks workflow permissions the tool will not be offered at all — that is the
filtering working, not a fault.

## Configuration

| Variable | Required | Description |
|---|---|---|
| `RDC_BASE_URL` | yes | Base URL of your RDataCore instance |
| `RDC_API_KEY` | stdio | The API key to act as |
| `RDC_MCP_TIMEOUT_SECS` | no | Outbound HTTP timeout, default 30 |
| `RDC_MCP_TRANSPORT` | no | `stdio` (default) or `http` |

An empty value counts as unset, so `RDC_API_KEY=` in a compose file produces a
clear refusal at startup rather than a puzzling 401 later.

## stdio is single-user

Everything runs as the owner of the configured key. That is right for a
developer on their own machine and wrong for anything shared, which is why
`http` transport refuses a static key outright: one shared credential over a
network-facing server would authenticate every caller as the same account and
collapse per-user permissions. Hosted deployment uses OAuth instead — see
`docs/MCP.md`.

## Authoring a workflow with it

The tools are designed around one loop, and the server's instructions tell an
assistant to follow it:

1. `dsl_options` for the vocabulary — generated from the running engine, so it
   is accurate for your instance in a way documentation cannot be.
2. `get_entity_definition` for field names, if the workflow touches entities.
   Never guess them.
3. Draft, then `validate_dsl`.
4. `test_workflow` — executes for real against an in-memory overlay and
   persists nothing. Iterate here freely.
5. `create_workflow` (disabled by default), `run_workflow` once to confirm,
   then enable.

Steps 3 and 4 are the ones worth insisting on. Validation catches structural
errors; only the dry-run catches the mapping and type errors that appear at
runtime.
