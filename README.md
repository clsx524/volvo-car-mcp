# VC Cli

VC Cli is a command-line interface for interacting with the Volvo Cars Connected Vehicle API.

This fork adds `vc-cli mcp`: an MCP server (streamable HTTP) that exposes the full command surface as tools, built for running in Docker. All CLI functionality is unchanged.

## MCP Server

`vc-cli mcp` serves every CLI operation as an MCP tool over streamable HTTP:

- Reads: `vehicle_list`, `vehicle_details`, `vehicle_windows`, `vehicle_doors`, `vehicle_warnings`, `vehicle_tyres`, `vehicle_statistics`, `vehicle_odometer`, `vehicle_fuel`, `vehicle_diagnostics`, `vehicle_engine_diagnostics`, `vehicle_engine_status`, `vehicle_brakes`, `energy_state`, `energy_capabilities`, `location_get`
- Remote commands (physical actuation — the tool descriptions say so): `lock`, `unlock`, `lock_reduced_guard`, `honk`, `flash`, `honk_flash`, `engine_start`, `engine_stop`, `climatization_start`, `climatization_stop`, plus `commands_list` and `commands_accessibility`
- Admin: `auth_whoami`, `set_default_vin`

Every VIN-optional tool falls back to the stored default VIN; tools share the same SQLite state and token refresh as the CLI.

### Environment Variables

| Variable | Required | Description |
|---|---|---|
| `VC_MCP_BEARER_TOKEN` | yes | Bearer token (e.g. a UUID, `uuidgen`). Every `/mcp` request must send `Authorization: Bearer <token>`; the server refuses to start without it. |
| `VC_MCP_HOST` / `VC_MCP_PORT` | no | Bind address, default `0.0.0.0:8811`. |
| `VC_MCP_ALLOWED_HOSTS` | no | Comma-separated `Host` header allowlist (DNS-rebinding guard). Unset disables Host validation; bearer auth remains the request gate. |
| `VOLVO_CONFIG_DIR` | no | State directory (config file + SQLite). Default `~/.config/vc-cli`; the Docker image sets `/data`. |
| `VCC_API_KEY`, `VOLVO_CLIENT_ID`, `VOLVO_CLIENT_SECRET` | no | Passed through to the API layer (same as the CLI). |
| `VOLVO_AUTH_ISSUER`, `VOLVO_API_HOST` | no | Region overrides (issuer defaults to the EU host). |

`GET /healthz` is unauthenticated and always returns `ok` for probes.

### Docker

```bash
docker build -t volvo-car-mcp .
```

docker-compose example:

```yaml
services:
  volvo-mcp:
    build: .
    ports:
      - "127.0.0.1:8811:8811"   # or your tailscale IP; avoid 0.0.0.0 on a LAN
    environment:
      VC_MCP_BEARER_TOKEN: "<uuidgen output>"
      VCC_API_KEY: "${VCC_API_KEY}"
      VOLVO_CLIENT_ID: "${VOLVO_CLIENT_ID}"
      VOLVO_CLIENT_SECRET: "${VOLVO_CLIENT_SECRET}"
    volumes:
      - volvo-config:/data
volumes:
  volvo-config:
```

### One-time Setup

```bash
# 1. Login (interactive; open the printed URL, paste the callback URL back).
#    Stop the server first, or rely on the SQLite busy timeout.
docker compose run --rm volvo-mcp auth login --headless

# 2. Verify session and vehicles.
docker compose run --rm volvo-mcp auth whoami
docker compose run --rm volvo-mcp vehicle list

# 3. If the account has several vehicles, pin a default VIN.
docker compose run --rm volvo-mcp vehicle vin default --vin "<VIN>"

# 4. Start the server.
docker compose up -d
```

Tokens refresh automatically (`ensure_fresh_session`); re-login is only needed if Volvo revokes the refresh token.

### Client Configuration

Claude Code:

```bash
claude mcp add --transport http volvo http://<host>:8811/mcp --header "Authorization: Bearer <token>"
```

Generic `mcpServers` entry:

```json
{
  "mcpServers": {
    "volvo": {
      "type": "http",
      "url": "http://<host>:8811/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

### Security Notes

- `lock`, `unlock`, `engine_start`, etc. actuate a real car. Do not expose `/mcp` unauthenticated: bearer token + loopback/tailnet binding is the minimum.
- Volvo rate limits apply: 10 command calls/min, 100 reads/min, 10000 calls/day per client ID. Exceeding them returns 429, which surfaces as a tool error.
- Tokens, client secret and API key live in the `VOLVO_CONFIG_DIR` volume — treat it as a credential store.

### Verifying without Volvo Credentials

`scripts/smoke-http.sh` (local build) and `scripts/smoke-container.sh` (Docker image) exercise bearer rejection, initialize, `tools/list` (30 tools + schemas) and tool-call error propagation end to end.

## Prerequisites

- Rust toolchain (stable)
- Volvo OAuth client credentials (`VOLVO_CLIENT_ID`, `VOLVO_CLIENT_SECRET`); prompted on first `auth login`
- Registered OAuth redirect URI; defaults to `http://127.0.0.1:1410/callback`
- API key for vehicle endpoints (`VCC_API_KEY`); prompted on first `auth login`

## Install and Run


Global npm install:

```bash
npm i -g @stefanosala/vc-cli
vc-cli --help
```

Install the skills for your agent:

```bash
npx skills add stefanosala/vc-cli
```

Agent-facing command guidance lives in `skills/vc-cli/SKILL.md` (the repo `@skills` entry for this CLI).

## Quick Start

1. Log in with Volvo OAuth:

```bash
vc-cli auth login
```

For headless hosts (agents/servers), use:

```bash
vc-cli auth login --headless
```

This prints an authorization URL. Open it in any browser, complete login, then paste the redirected callback URL back into the CLI.

If developer credentials are missing, the command shows the Volvo developer account URL and redirect URI to use, then waits for you to return and enter the API key, client ID, and client secret.

2. Discover your vehicles:

```bash
vc-cli vehicle list
```

3. Set a default VIN if more than one vehicle is available:

```bash
vc-cli vehicle vin default --vin "<VIN>"
```

4. Fetch vehicle data:

```bash
vc-cli vehicle windows get
```

If no default VIN is configured, VIN-based commands discover vehicles automatically. A single VIN is saved as the default; multiple VINs are shown as an interactive prompt.

## Common Commands

```bash
# List vehicles
vc-cli vehicle list

# Show active identity/session state
vc-cli auth whoami

# Get latest known location
vc-cli location get
```

## Agent Skills

If you are using an AI agent (for example via Cursor skills), install:

```bash
npx skills add stefanosala/vc-cli
```

Then reference `skills/vc-cli/SKILL.md` for command map, auth workflows, and safety notes (including headless login guidance).

## Notes

- Command output is JSON by default.
- Configuration is loaded from `~/.config/vc-cli/config` in env variable format before CLI parsing.
- Profile/session/VIN state is stored locally per profile in SQLite.
- `auth login` starts a temporary local HTTP listener for the OAuth redirect.
- `auth login --headless` skips browser/listener setup, prints the authorize URL, and prompts for the redirected callback URL.
- `auth login` prompts for missing `VCC_API_KEY`, `VOLVO_CLIENT_ID`, and `VOLVO_CLIENT_SECRET`, then saves them to `~/.config/vc-cli/config`.
- `auth login` requests all Connected Vehicle, Energy, and Location scopes by default. Use `--scopes` or `VOLVO_SCOPES` to override the requested scope set.

## License

MIT
