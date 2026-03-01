# planner-web

The React frontend for Planner v2. A TypeScript + Vite single-page application that provides an Auth0-authenticated planning dashboard with real-time WebSocket chat, pipeline visualization, and a session listing view.

---

## Prerequisites

- **Node.js 18 or later** — [nodejs.org](https://nodejs.org/)
- `planner-server` running locally (for the API and WebSocket backend)

```bash
node --version   # must be 18+
npm --version
```

---

## Development Setup

```bash
# From the planner-web/ directory
npm install
npm run dev
```

The Vite dev server starts at `http://localhost:5173` with hot module replacement enabled.

The app proxies API calls to `http://localhost:3100` (the default `planner-server` port). Start the backend in a separate terminal:

```bash
# From the workspace root
cargo run --bin planner-server
```

Auth0 is **not required for local development**. When the `VITE_AUTH0_DOMAIN` environment variable is absent, the app runs in dev mode: the login page is bypassed, and the server accepts requests with a synthetic `dev|local` user. See [Auth0 Configuration](#auth0-configuration) if you need to test authenticated flows.

---

## Building for Production

```bash
npm run build
```

Output is written to `dist/`. Serve it via `planner-server`:

```bash
cargo run --bin planner-server -- --static-dir ./planner-web/dist
```

Or point any static file server at the `dist/` directory.

---

## Running Tests

```bash
# Watch mode (re-runs on file changes — default for development)
npm test

# Single run (for CI)
npm run test -- --run

# With coverage report
npm run test -- --run --coverage
```

Tests use [Vitest](https://vitest.dev/) and [React Testing Library](https://testing-library.com/docs/react-testing-library/intro/). The Vitest configuration is in `vite.config.ts`.

**Test files:**

| File | What it tests |
|---|---|
| `src/api/__tests__/client.test.ts` | `ApiError` class, typed fetch wrappers, error propagation |
| `src/components/__tests__/ChatPanel.test.tsx` | Message rendering, scroll preservation, deduplication |
| `src/components/__tests__/Layout.test.tsx` | App shell rendering, nav links |
| `src/components/__tests__/MessageInput.test.tsx` | Auto-grow textarea, submit on Enter, empty-input guard |
| `src/components/__tests__/PipelineBar.test.tsx` | Stage visualization, active/complete state rendering |
| `src/pages/__tests__/LoginPage.test.tsx` | Auth0 redirect, dev-mode bypass |

Total: **97 tests**, 0 failures.

---

## Auth0 Configuration

Auth0 is optional for development but required for production deployments. Full setup instructions are in [../AUTH0_SETUP.md](../AUTH0_SETUP.md).

**Quick summary:** create a `.env` file in this directory with three values:

```env
VITE_AUTH0_DOMAIN=your-tenant.us.auth0.com
VITE_AUTH0_CLIENT_ID=your-client-id
VITE_AUTH0_AUDIENCE=https://planner-api
```

Then rebuild: `npm run build`.

When these variables are absent the app automatically enters dev mode — no Auth0 account is needed.

---

## Component Architecture

```
src/
├── main.tsx                        # App entry point, Auth0Provider mount
├── App.tsx                         # Root router (React Router v6)
├── config.ts                       # Runtime env var configuration
├── types.ts                        # Shared TypeScript types (Session, Message, PipelineStage, etc.)
│
├── api/
│   └── client.ts                   # ApiError class + typed fetch wrappers for all /api/v1 endpoints
│
├── auth/
│   ├── Auth0ProviderWithNavigate.tsx   # Auth0Provider wired to React Router navigation
│   ├── ProtectedRoute.tsx             # Redirects unauthenticated users to /login
│   └── useAuthenticatedFetch.ts       # Hook that injects Authorization header via getAccessTokenSilently
│
├── components/
│   ├── ChatPanel.tsx               # Scrollable message list; rehype-sanitize XSS prevention; deduplication
│   ├── Layout.tsx                  # Persistent app shell with nav
│   ├── MessageInput.tsx            # Auto-grow textarea; Enter to send; Shift+Enter for newline
│   └── PipelineBar.tsx             # 12-stage pipeline visualization bar with live status
│
├── hooks/
│   └── useSessionWebSocket.ts      # WebSocket client with exponential-backoff reconnection
│
├── pages/
│   ├── Dashboard.tsx               # Lists all sessions for the current user; create new session button
│   ├── LoginPage.tsx               # Auth0 Universal Login redirect; dev-mode bypass
│   └── SessionPage.tsx             # Full session view: ChatPanel + PipelineBar + MessageInput
│
└── test/
    └── setup.ts                    # Vitest global setup; Auth0 mock; WebSocket stub
```

### Key Design Decisions

- **`client.ts` is the API boundary.** All HTTP calls go through this module. Components never call `fetch` directly. This makes mocking trivial in tests.
- **Auth0 mock in `setup.ts`.** Auth0's React SDK is mocked at the module level in `setup.ts`, so every test that imports an auth-dependent component gets a clean, controllable auth state without requiring a real Auth0 tenant.
- **`rehype-sanitize` in `ChatPanel`.** All server-returned message content passes through rehype-sanitize before being rendered. This prevents XSS from malicious LLM output.
- **`useSessionWebSocket` reconnection.** The hook uses exponential backoff (up to 30 s) and exposes `readyState` so the UI can show connection status. It also deduplicates messages by ID to handle reconnect replays.
- **ARIA accessibility.** All interactive elements have `aria-label` attributes. The pipeline bar uses `role="progressbar"` with `aria-valuenow`.

---

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `VITE_AUTH0_DOMAIN` | No | *(unset — dev mode)* | Auth0 tenant domain |
| `VITE_AUTH0_CLIENT_ID` | No | *(unset — dev mode)* | Auth0 application client ID |
| `VITE_AUTH0_AUDIENCE` | No | *(unset — dev mode)* | Auth0 API audience identifier |
| `VITE_API_BASE_URL` | No | `http://localhost:3100` | Base URL for the planner-server API |

All variables are prefixed with `VITE_` so Vite embeds them at build time. They are **not secrets** — avoid putting anything sensitive here.
