# Frontend Tests — Changes Summary

## Vitest Setup

Configured Vitest with React Testing Library for the `planner-web` React + TypeScript + Vite frontend.

### Dependencies Added (dev)
- `vitest` v4
- `@testing-library/react`
- `@testing-library/jest-dom`
- `@testing-library/user-event`
- `jsdom`

### Configuration Files

| File | Purpose |
|------|---------|
| `vite.config.ts` | Added `test` block: globals, jsdom environment, setup file |
| `src/test/setup.ts` | Global test setup: Auth0 mock, `scrollIntoView` mock, `matchMedia` mock |
| `package.json` | Added `"test": "vitest run"` and `"test:watch": "vitest"` scripts |

---

## Test Results

```
Test Files: 6 passed (6)
     Tests: 97 passed (97)
  Duration: ~7.7s
```

All tests pass. `npx tsc --noEmit` also passes with zero errors.

---

## Test Files Created

### `src/components/__tests__/MessageInput.test.tsx` — 21 tests
- Renders textarea and send button
- Send button disabled when input is empty
- Send button disabled when `disabled` prop is true
- Send button disabled when `pipelineRunning` is true
- Calls `onSend` with trimmed input value on button click
- Clears input after sending
- Sends on Enter key (without Shift)
- Does not send on Shift+Enter
- Shows pipeline hint text when `pipelineRunning` is true
- ARIA labels on textarea (`"Message input"`) and button (`"Send message"`)
- Shows `"Waiting for response…"` placeholder when `isLoading`
- Shows pipeline placeholder when `pipelineRunning`
- Button text is `"…"` when loading, `"send"` otherwise
- Textarea disabled when `pipelineRunning` or `isLoading`
- Textarea ref attached (auto-grow behavior)

### `src/components/__tests__/PipelineBar.test.tsx` — 14 tests
- Renders all stage names
- Stage names rendered with `text-transform: uppercase` style
- Empty bar renders without stage content
- Arrow (`›`) separators between stages (n−1 for n stages)
- No separator after last stage
- `.pulse` CSS class applied to running stage dot indicator
- No `.pulse` class on non-running stages
- All 12 pipeline stages render correctly (including "AR Review")
- Single stage renders without separator
- Running stage has `fontWeight: 700`; pending/complete/failed have `fontWeight: 400`

### `src/components/__tests__/ChatPanel.test.tsx` — 15 tests
- Empty message list shows `"no messages yet — send one to begin"`
- Renders user, planner, and system message content
- Role labels rendered as lowercase DOM text with CSS `text-transform: uppercase`
- Multiple messages all render
- Planner messages rendered via ReactMarkdown (`**bold**` → `<strong>`)
- Code blocks rendered via ReactMarkdown
- User messages rendered as plain text (no markdown processing)
- System messages rendered as italic plain text
- Timestamps rendered for messages
- Non-empty message list hides empty state

### `src/components/__tests__/Layout.test.tsx` — 13 tests
- Renders children in `<main>`
- Header displays `"PLANNER v2"` and `"— Socratic Lobby"`
- `<header>` has `role="banner"`
- Session ID badge shows when `sessionId` prop provided
- No session badge when `sessionId` omitted
- Connection status indicator shown when `sessionId` is set
- `"connected"` / `"disconnected"` text based on `isConnected` prop
- Status indicator has `aria-label="Connection status"` and `role="status"`
- Dev mode shows `"dev mode"` label (when `AUTH0_ENABLED=false`)

### `src/api/__tests__/client.test.ts` — 22 tests
- `getSession` → GET `/api/sessions/{id}`
- `createSession` → POST `/api/sessions` with `{}` body
- `listSessions` → GET `/api/sessions`
- `sendMessage` → POST `/api/sessions/{id}/message` with `{content}` payload
- `health` → GET `/api/health`
- Sets `Authorization: Bearer <token>` header
- Sets `Content-Type: application/json` header
- Throws `ApiError` on non-OK responses (404, 500)
- `ApiError` carries `.status` code and message
- `isAuthError` returns `true` for 401/403 `ApiError` instances
- `isAuthError` returns `true` for generic `Error` with "401"/"403" in message
- `isAuthError` returns `false` for other status codes / messages

### `src/pages/__tests__/LoginPage.test.tsx` — 12 tests
- Renders `"PLANNER v2"` heading and terminal title bar
- Renders login button (not disabled)
- Button text is `"enter  (dev mode)"` when `AUTH0_ENABLED=false`
- Shows `"Auth0 not configured"` dev mode notice
- Feature list items rendered (pipeline, WebSocket, Socratic dialogue)
- Description text rendered
- Auth0 mock is accessible with `loginWithRedirect` function
- Button click does not throw

---

## Mocking Strategy

| Dependency | Strategy |
|-----------|---------|
| `@auth0/auth0-react` | `vi.mock` factory with `vi.fn()` for `useAuth0` (enables `vi.mocked`) |
| `fetch` | `vi.spyOn(global, 'fetch')` per test in client tests |
| `window.matchMedia` | `Object.defineProperty` in global setup |
| `Element.prototype.scrollIntoView` | Assigned `vi.fn()` in global setup (jsdom omits it) |
| `react-router-dom` | `MemoryRouter` wrapper in page tests |
| WebSocket | Not tested directly (hook-level; covered by integration path) |

---

## Notes

- No source files were modified — only test files and configuration were added.
- CSS `text-transform: uppercase` is applied via inline styles; DOM text remains lowercase (`"user"`, `"planner"`, `"system"`, `"Intake"`, etc.). Tests assert on DOM text content, not visual rendering.
- Auth0 is disabled in the test environment (no `VITE_AUTH0_DOMAIN`/`VITE_AUTH0_CLIENT_ID` env vars), so `AUTH0_ENABLED=false` and dev-mode code paths are exercised.
