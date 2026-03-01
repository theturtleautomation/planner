# Deployment Guide — Planner v2

This document covers everything needed to run Planner v2 in local development and production environments.

---

## Table of Contents

- [Prerequisites](#prerequisites)
- [Local Development](#local-development)
- [Production Deployment](#production-deployment)
- [Environment Variables Reference](#environment-variables-reference)
- [LLM CLI Installation](#llm-cli-installation)
- [Docker](#docker)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Rust Toolchain

Install via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
rustc --version   # should be 1.75 or later
```

### Node.js (frontend only)

Node.js 18 or later is required to build or develop the React frontend.

```bash
# Check current version
node --version   # must be 18+

# Install via nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash
nvm install 18
nvm use 18
```

### LLM CLI Tools

At least one of the following CLI tools must be installed and authenticated for the pipeline to run. All three are needed for full fidelity (parallel AR review, Factory Worker, Scenario Validator).

See [LLM CLI Installation](#llm-cli-installation) below for detailed setup instructions.

| Binary | Provider |
|---|---|
| `claude` | Anthropic |
| `gemini` | Google |
| `codex` | OpenAI |

### Git

Required for the Git Projection pipeline stage:

```bash
# macOS
brew install git

# Debian/Ubuntu
sudo apt install git
```

---

## Local Development

This setup runs everything locally with no Auth0 account required. Auth is in dev mode — the server accepts all requests with a synthetic `dev|local` user.

### Step 1: Clone and Build the Rust Workspace

```bash
git clone https://github.com/theturtleautomation/planner.git
cd planner
cargo build --workspace
```

### Step 2: Start the Backend Server

```bash
# No environment variables needed for dev mode
cargo run --bin planner-server
```

The server starts on `http://localhost:3100`. Confirm it's running:

```bash
curl http://localhost:3100/api/health
# → {"status":"ok"}
```

### Step 3: Start the React Dev Server (optional)

If you want to work on the frontend with hot module replacement:

```bash
cd planner-web
npm install
npm run dev
```

The Vite dev server runs at `http://localhost:5173` and proxies API calls to `localhost:3100`.

If you only need the built frontend, skip this step — `planner-server` already serves the pre-built `planner-web/dist` at `http://localhost:3100`.

### Step 4: Run the Pipeline

```bash
# Full pipeline
./target/debug/planner-core "Build me a task tracker widget"

# Front office only (no code generation)
./target/debug/planner-core --fo "Build me a pomodoro timer"
```

### Step 5: Run All Tests

```bash
# Rust tests
cargo test --workspace

# Frontend tests
cd planner-web && npm test -- --run
```

---

## Production Deployment

### Step 1: Build the Rust Binaries

```bash
cargo build --release
```

Binaries are written to `./target/release/`:
- `planner-core` — pipeline runner
- `planner-server` — HTTP + WebSocket server
- `planner-tui` — terminal UI

Copy them to a location on your `$PATH` or deploy them directly:

```bash
sudo cp target/release/planner-server /usr/local/bin/
sudo cp target/release/planner-core /usr/local/bin/
```

### Step 2: Build the React Frontend

```bash
cd planner-web

# Create the environment file with your Auth0 credentials
cp .env.example .env
# Edit .env — see Auth0 Configuration below

npm install
npm run build
```

Output is written to `planner-web/dist/`.

### Step 3: Configure Auth0

Follow the full instructions in [AUTH0_SETUP.md](./AUTH0_SETUP.md). The short version:

1. Create a free Auth0 account at [auth0.com](https://auth0.com/signup)
2. Create a **Single Page Application** named `Planner v2 Web`
3. Create an **API** with identifier `https://planner-api`
4. Set the callback, logout, and origin URLs for your production domain
5. Copy the Domain and Client ID values

Then populate `planner-web/.env`:

```env
VITE_AUTH0_DOMAIN=your-tenant.us.auth0.com
VITE_AUTH0_CLIENT_ID=your-client-id
VITE_AUTH0_AUDIENCE=https://planner-api
```

Rebuild the frontend after editing `.env`:

```bash
cd planner-web && npm run build
```

### Step 4: Set Backend Environment Variables

```bash
export AUTH0_DOMAIN=your-tenant.us.auth0.com
export AUTH0_AUDIENCE=https://planner-api
export RUST_LOG=planner_server=info,planner_core=info
```

For a persistent deployment, add these to a systemd unit file or your hosting platform's environment configuration.

### Step 5: Run the Server

```bash
planner-server \
  --port 3100 \
  --static-dir /path/to/planner-web/dist
```

For a reverse proxy setup (nginx, Caddy), proxy all requests to `localhost:3100`. WebSocket upgrade (`Upgrade: websocket`) must be forwarded — example nginx block:

```nginx
location / {
    proxy_pass         http://localhost:3100;
    proxy_http_version 1.1;
    proxy_set_header   Upgrade $http_upgrade;
    proxy_set_header   Connection "upgrade";
    proxy_set_header   Host $host;
    proxy_set_header   X-Real-IP $remote_addr;
}
```

### Step 6: Verify

1. Open your production domain in a browser
2. You should see the Planner v2 login page
3. Sign in via Auth0
4. Create a session and confirm WebSocket connectivity (the pipeline bar should update in real time)

---

## Environment Variables Reference

### Backend (`planner-server`)

| Variable | Required | Default | Description |
|---|---|---|---|
| `AUTH0_DOMAIN` | No | *(dev mode)* | Auth0 tenant domain (e.g., `planner-v2.us.auth0.com`). Omit to run without auth. |
| `AUTH0_AUDIENCE` | No | *(dev mode)* | Auth0 API identifier (e.g., `https://planner-api`). Must match `VITE_AUTH0_AUDIENCE`. |
| `AUTH0_SECRET` | No | *(none)* | HS256 signing secret for dev/testing token issuance only. Not used with RS256 in production. |
| `RUST_LOG` | No | `info` | Log filter using `tracing` env-filter syntax (e.g., `planner_server=debug`). |

### Frontend (`planner-web`)

All frontend variables are embedded at build time by Vite. They are not secrets.

| Variable | Required | Default | Description |
|---|---|---|---|
| `VITE_AUTH0_DOMAIN` | No | *(dev mode)* | Auth0 tenant domain. Omit to run without auth. |
| `VITE_AUTH0_CLIENT_ID` | No | *(dev mode)* | Auth0 application client ID from your SPA application. |
| `VITE_AUTH0_AUDIENCE` | No | *(dev mode)* | Auth0 API identifier. Must match `AUTH0_AUDIENCE` on the backend. |
| `VITE_API_BASE_URL` | No | `http://localhost:3100` | Base URL of the `planner-server` instance. Change for production. |

### Dev Mode Behavior

When Auth0 environment variables are **absent**:

- **Frontend**: The login page is bypassed; users land directly on the dashboard.
- **Backend**: A synthetic `dev|local` user is injected into every request; all endpoints respond without a JWT.

This is intentional — Auth0 is not needed for local development or running tests.

---

## LLM CLI Installation

Planner v2 routes LLM calls through native CLI tools rather than HTTP API keys. Install the ones you have subscriptions for.

### `claude` — Anthropic CLI

Requires a Claude Max or Claude Pro subscription.

1. Install via pip:
   ```bash
   pip install anthropic-claude-cli
   ```
   Or follow the latest instructions at [claude.ai/download](https://claude.ai/download) or the [Anthropic CLI documentation](https://docs.anthropic.com/claude/cli).

2. Authenticate:
   ```bash
   claude login
   ```

3. Verify:
   ```bash
   claude --version
   echo "hello" | claude
   ```

### `gemini` — Google Gemini CLI

Requires a Google account with Gemini Advanced/Pro access.

1. Install the Google GenAI CLI. The official tool is distributed as part of the Google Cloud SDK or as a standalone binary:
   ```bash
   # Via npm
   npm install -g @google/generative-ai-cli

   # Or follow instructions at
   # https://ai.google.dev/gemini-api/docs/quickstart
   ```

2. Authenticate:
   ```bash
   gemini login
   # Or: gcloud auth login (if using the Google Cloud path)
   ```

3. Verify:
   ```bash
   gemini --version
   echo "hello" | gemini
   ```

### `codex` — OpenAI Codex CLI

Requires a ChatGPT Pro subscription.

1. Install via npm:
   ```bash
   npm install -g @openai/codex
   ```
   Or follow the instructions at [github.com/openai/codex](https://github.com/openai/codex) or [platform.openai.com/docs/codex](https://platform.openai.com/docs/codex).

2. Authenticate:
   ```bash
   codex login
   ```

3. Verify:
   ```bash
   codex --version
   echo "hello" | codex
   ```

### Confirming CLI Detection

After installing at least one CLI, run:

```bash
planner-core --help
```

The output lists which CLIs were detected on `$PATH`. If a required CLI is missing for a specific stage, that stage is skipped (AR panel) or falls back to simulation mode (Factory Worker).

---

## Docker

Docker support is planned but not yet implemented. A multi-stage `Dockerfile` that builds the Rust binaries and React app in separate build stages, then packages them into a minimal runtime image, is on the roadmap.

To track or contribute to this work, see the [Docker issue on GitHub](https://github.com/theturtleautomation/planner/issues) and the relevant entry in [CONTRIBUTING.md](./CONTRIBUTING.md#known-limitations--areas-for-contribution).

Until Docker support lands, the recommended deployment approach is the manual process described in [Production Deployment](#production-deployment) above, optionally behind a process supervisor like systemd or supervisord.

---

## Troubleshooting

### Server fails to start — "address already in use"

Port 3100 is occupied. Either stop the conflicting process or use a different port:

```bash
planner-server --port 8080
```

Find the conflicting process:

```bash
lsof -i :3100
```

### `cargo build` fails with "linker error" on Linux

Install the C linker and OpenSSL dev headers:

```bash
sudo apt install build-essential pkg-config libssl-dev
```

### `planner-core` exits with "no LLM CLI found"

None of `claude`, `gemini`, or `codex` are on `$PATH`. Install at least one — see [LLM CLI Installation](#llm-cli-installation).

Confirm the binary is accessible:

```bash
which claude
which gemini
which codex
```

### WebSocket connection drops immediately

- Confirm `planner-server` is running and reachable.
- If using a reverse proxy, verify WebSocket upgrade headers are forwarded (see the nginx example in [Production Deployment](#production-deployment)).
- Check the browser console for the close code. Code `1008` indicates a policy violation (usually an expired JWT).

### "Callback URL mismatch" after Auth0 login

The redirect URL after login doesn't match any entry in Auth0's **Allowed Callback URLs**. Add your exact origin (including port) to the Auth0 application settings. See [AUTH0_SETUP.md](./AUTH0_SETUP.md#configure-the-application).

### "401 Unauthorized" on API calls after successful login

- Confirm `AUTH0_DOMAIN` and `AUTH0_AUDIENCE` are set on the server and match the frontend's `VITE_AUTH0_DOMAIN` and `VITE_AUTH0_AUDIENCE`.
- Check that `AUTH0_DOMAIN` does **not** include `https://` — pass the bare domain only.
- Verify the token hasn't expired (default Auth0 expiry is 24 hours; the React SDK refreshes silently via `getAccessTokenSilently`).

### Rate limit errors (429) in development

The server enforces 100 requests/minute per IP. During development, automated test scripts or rapid manual testing can hit this limit. If you need a higher limit locally, increase the constant in `planner-server/src/rate_limit.rs` and rebuild.

### Frontend build fails — "VITE_AUTH0_DOMAIN is required"

If the frontend was built with a stricter config that validates env vars at build time, create `planner-web/.env` with the required values before running `npm run build`. For dev mode builds, you can leave all `VITE_AUTH0_*` variables unset.

### `npm test` fails with "Cannot find module '@auth0/auth0-react'"

Run `npm install` in the `planner-web/` directory first. The Auth0 SDK and all test dependencies must be installed before the test suite can run.
