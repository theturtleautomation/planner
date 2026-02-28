# Frontend Audit — planner-web

**Date:** 2026-02-28  
**Auditor:** Automated code review  
**Scope:** All 18 source files in `src/`, plus `package.json` and `vite.config.ts`

---

## Executive Summary

The frontend is **well above minimal scaffolding** — it is a thoughtful, cohesive React + TypeScript SPA with a clear design system, a functional WebSocket integration, and explicit dev/prod auth toggling. The code quality is consistent, readable, and free of `any` types. The primary gaps are operational concerns: no tests, no global state management, no responsive CSS, no runtime API schema validation, and several subtle bugs in state synchronisation. None are blockers, but several need attention before production traffic.

---

## 1. Per-File Assessment

### `src/main.tsx`
**Quality:** Production-grade  
- Wraps the entire tree in a class-based `ErrorBoundary` that catches render crashes and shows a recovery UI — good defensive practice.  
- Conditionally wraps with `Auth0ProviderWithNavigate` only when `AUTH0_ENABLED` is true; avoids rendering an unconfigured provider.  
- Guards `document.getElementById('root')` with a hard throw — correct.  
- Uses `StrictMode` — good for surfacing double-render issues in development.

### `src/App.tsx`
**Quality:** Production-grade  
- Clean route table: `/`, `/callback`, `/session/new`, `/session/:id`, and a catch-all `*`.  
- `CallbackPageAuth0` and `RootPageAuth0` each handle `isLoading` and `error` states explicitly.  
- Dev mode (`AUTH0_ENABLED=false`) routes directly to `Dashboard` without any auth wall — intentional and clearly communicated.

### `src/config.ts`
**Quality:** Good  
- All config is sourced from `import.meta.env` with safe fallbacks to empty string.  
- `AUTH0_ENABLED` computed from whether domain + client ID are set — clean approach.  
- **Gap:** `AUTH0_AUDIENCE` falls back to `''` but in `Auth0ProviderWithNavigate` it is coerced to `undefined` when empty. This is handled correctly downstream, but the fallback chain is subtle.

### `src/types.ts`
**Quality:** Production-grade  
- All domain types are strongly typed: `PipelineStageName` is a string union (not `string`), `StageStatus` is an enum-like union.  
- `ServerWsMessage` and `ClientWsMessage` are discriminated unions — enables exhaustive `switch` dispatch.  
- `ChatMessage.timestamp` is `string` (ISO); no `Date` objects. Acceptable, though worth documenting.

### `src/index.css`
**Quality:** Adequate for desktop; not responsive  
- Uses CSS custom properties (`--bg-primary`, `--accent-cyan`, etc.) — clean design token system.  
- Defines `pulse` and `blink` keyframes used by components.  
- **Gap:** Zero `@media` queries. The layout is entirely desktop-first (fixed heights, `flexDirection: column`, no viewport-aware breakpoints). On mobile (<600px), the pipeline bar will overflow horizontally and the chat panel will not resize properly.  
- Scrollbar styling is WebKit-only; no Firefox equivalent.

### `src/vite-env.d.ts`
**Quality:** Correct  
- Properly extends `ImportMetaEnv` with all four `VITE_` variables as `readonly string`. Consistent with `config.ts`.

### `src/auth/Auth0ProviderWithNavigate.tsx`
**Quality:** Production-grade  
- Implements `onRedirectCallback` using `react-router-dom`'s `useNavigate` — correct pattern to avoid full-page reload on Auth0 callback.  
- Falls back to `window.location.pathname` if `appState.returnTo` is missing.  
- `AUTH0_AUDIENCE` correctly converted to `undefined` when empty rather than passed as an empty string.

### `src/auth/ProtectedRoute.tsx`
**Quality:** Good  
- Separate `Auth0Route` / `DevRoute` components selected at render time rather than inside a hook — avoids conditional hook calls.  
- `Auth0Route` preserves `location` in navigate `state` so the user can be returned to their intended URL after login — standard PKCE redirect pattern.  
- **Minor:** The loading spinner is inline style only. If Auth0 hangs indefinitely (e.g., network timeout), there is no timeout escape.

### `src/auth/useAuthenticatedFetch.ts`
**Quality:** Clever, slightly fragile  
- Module-level conditional (`AUTH0_ENABLED ? hookA : hookB`) avoids React's rules-of-hooks violation at the call site. The `eslint-disable` comments acknowledge the deliberate rule-of-hooks exception.  
- `useGetAccessToken` silently returns `''` on token errors rather than re-throwing — prevents crashes but masks authentication failures. Callers cannot distinguish "no auth configured" from "token fetch failed silently".  
- **Gap:** When a token fetch fails, `getToken` returns `''`, the API request is sent without a `Bearer` header, and the server returns 401. That 401 is then thrown as a generic `Error` string (see `client.ts`) with no re-authentication trigger. The user sees a raw error message, not a login redirect.

### `src/api/client.ts`
**Quality:** Functional, missing 401/403 handling  
- `apiFetch` is a clean factory pattern: one generic function, typed return values, no `any`.  
- On non-2xx responses, extracts the response body for the error message — good DX.  
- **Critical Gap:** No special handling for HTTP 401 or 403. A 401 throws the same generic `Error` as a 500. There is no call to `loginWithRedirect()`, no token refresh attempt, and no user-visible prompt to re-authenticate. In production with short-lived tokens, a user with an expired session will see a raw error string.  
- `body: '{}'` on `createSession` is a hardcoded literal instead of `JSON.stringify({})` — technically identical but inconsistent with `sendMessage`.

### `src/hooks/useSessionWebSocket.ts`
**Quality:** Solid, with one notable limitation  
- Implements exponential backoff reconnection (1 s, 2 s, 4 s) with `MAX_RETRIES = 3`.  
- Cleans up properly on unmount: removes `onclose` handler before calling `close()` to prevent spurious reconnection attempts, clears the retry timer, sets `mountedRef.current = false`.  
- Uses `sessionIdRef` to safely access the current session ID inside async callbacks without stale closures.  
- On `sessionId` change, resets stages, `pipelineComplete`, and `pipelineSummary` — but **does not reset `messages`**. If a user navigates from one session to another without unmounting the hook, WS messages from the new session are appended to existing messages from the old session. (In practice, the page unmounts on navigation, so this is low-risk but worth noting.)  
- **Security note:** The Auth0 JWT is appended as a query-string parameter (`?token=…`) in the WebSocket URL. This is a common workaround for WebSocket protocols that cannot set headers, but it means the token appears in server access logs, browser history, and any network proxy logs. A safer pattern is to send the token as the first WebSocket message after the handshake.  
- After `MAX_RETRIES` is exhausted, `isConnected` remains `false` with no user-visible UI feedback beyond the header indicator dot.

### `src/components/Layout.tsx`
**Quality:** Good  
- Clean header with left/right flex layout: branding, session badge, connection indicator, user info.  
- Connection indicator correctly differentiates undefined (not a session page) from `false` (disconnected).  
- Hover effects on the logout button use inline `onMouseEnter`/`onMouseLeave` — works, but would be cleaner as CSS classes.  
- **Accessibility Gap:** No `aria-label` on the logout `<button>`. No `role` or `aria-live` on the connection status indicator. Keyboard focus styles rely on browser defaults only.

### `src/components/ChatPanel.tsx`
**Quality:** Good  
- Handles the empty state with a placeholder message — correct.  
- Auto-scrolls to the bottom on every `messages` change via `scrollIntoView({ behavior: 'smooth' })` — functional.  
- Planner messages are rendered through `ReactMarkdown` with custom renderers for `p`, `code`, `pre`, `ul`, `ol`, `li`, `strong`. Code blocks have syntax-styled backgrounds.  
- **Gap:** No user-scroll detection. If the user scrolls up to read earlier messages and a new message arrives, they are force-scrolled back to the bottom. This is poor UX for long conversations.  
- **Gap:** `ReactMarkdown` does not sanitise HTML by default in v10 (the `rehype-sanitize` plugin is absent). Planner responses containing raw HTML would be rendered. If the planner LLM can ever emit untrusted content, this is an XSS vector.  
- `formatTime` uses `toLocaleTimeString` without a locale parameter, so time format varies by user's browser locale — acceptable for an internal tool.

### `src/components/PipelineBar.tsx`
**Quality:** Production-grade  
- Receives `stages` as a prop (sourced from the WebSocket hook), updates purely reactively — correct.  
- `STATUS_COLORS` and `STATUS_BG` are exhaustive `Record<StageStatus, string>` — no fallthrough.  
- Running stages have the `pulse` CSS animation applied via a global class — clean.  
- `overflowX: 'auto'` on the bar container handles the 12-stage horizontal list correctly.  
- **Minor:** Stage chips have no tooltip or `title` attribute to explain what a stage does. On narrow screens the abbreviations may be ambiguous.

### `src/components/MessageInput.tsx`
**Quality:** Good  
- Correct Enter-to-send / Shift+Enter-for-newline UX.  
- Disabled state, pipeline-running state, and loading state all have distinct placeholder text.  
- `send` trims whitespace and short-circuits on empty — correct.  
- **Gap:** The `<textarea>` is set to `rows={1}` with a CSS `maxHeight: 120px` but **has no auto-grow logic**. It will not expand as the user types multiple lines; `overflowY: auto` means it silently scrolls instead. This is a visible UX regression for multi-line input.  
- **Gap:** No `aria-label` on the textarea or the send button. Screen readers will not announce the purpose of either element.  
- **Minor:** The `⚡` emoji in the pipeline-running hint is inconsistent with the otherwise emoji-free design system.

### `src/pages/LoginPage.tsx`
**Quality:** Good  
- Shared `LoginView` renders correctly for both Auth0 and dev-mode.  
- Shows a visible warning when running without Auth0.  
- **Minor:** The CTA button has no `type="button"` attribute. In a form context this would default to `submit`; here it is safe, but defensive coding would add it.

### `src/pages/Dashboard.tsx`
**Quality:** Minimal / placeholder  
- The dashboard is a **static empty state** — it always shows "no sessions yet" and never fetches existing sessions from the API.  
- There is no `GET /sessions` (list) call in `client.ts` and no such call in `Dashboard.tsx`. A returning user cannot see or resume past sessions.  
- This is the largest functional gap in the frontend.

### `src/pages/SessionPage.tsx`
**Quality:** Good, with some edge-case gaps  
- Handles three init paths cleanly: new session (create + navigate), existing session (load), and error (display + back button).  
- Loading state is correctly shown when `sessionId` is null and there is no error.  
- `allMessages` merges REST-loaded messages with WebSocket-delivered ones via `useMemo` — this is correct for initial load, but **can produce duplicates**: if `sendMessage` appends `user_message` + `planner_message` to `restMessages`, and the server also emits those same messages as WebSocket `message` events, they will appear twice in the chat.  
- `handleSend` errors are swallowed with `console.error` — the user receives no visual feedback when sending a message fails.  
- The `pipelineComplete` → `pipeline_running: false` sync uses `session` as a dependency in a `useEffect`, which causes the effect to re-run every time `session` changes. Since `session` is also updated inside the effect, this creates a re-render loop if multiple `pipelineComplete` events arrive. A safer guard would use a functional updater without `session` as a dep.  
- `useEffect` init dep array omits `api` (suppressed with `eslint-disable-next-line react-hooks/exhaustive-deps`) — safe because `api` is recreated only when `getToken` changes, and `getToken` is stable, but the suppression comment hides this reasoning.  
- When `routeId` exists but the session is not found (server returns 404), the error message from the API (`API GET /sessions/:id → 404: …`) is displayed verbatim to the user — not user-friendly.

---

## 2. Critical Questions — Answered

### Q1: Does the WebSocket hook handle reconnection? What happens on network failure?

**Partially.** The hook implements exponential backoff with up to 3 retries (delays: 1 s, 2 s, 4 s). After exhausting retries, it silently gives up — `isConnected` stays `false`, the header dot turns red/blinks, but no modal, toast, or in-chat message is shown to the user. There is no "retry" button. After a prolonged disconnect, the user has no way to manually reconnect without a full page reload.

**No handling for:** tab visibility changes (`visibilitychange` event), online/offline events (`navigator.onLine`), or server-side forced closes (e.g., 1008 Policy Violation when a token expires mid-session).

### Q2: Does the API client handle 401/403?

**No.** All HTTP errors, including 401 and 403, are thrown as generic `Error` objects with a status code embedded in the message string. There is no:
- Automatic redirect to `loginWithRedirect()`
- Token refresh attempt via `getAccessTokenSilently()`
- User-visible re-authentication prompt

In production, when a user's Auth0 token expires, the next API call will throw a raw error string like `"API POST /sessions/123/message → 401: Unauthorized"`, which is displayed in the `initError` UI (for session init) or silently logged to the console (for `sendMessage`).

### Q3: Is the chat panel functional?

**Yes, with caveats.**
- Markdown renders correctly via `react-markdown` v10 with custom component overrides.
- Auto-scroll-to-bottom works.
- Empty state is handled.
- **Not handled:** user-scroll preservation (scrolling up while messages arrive will force-scroll the user back), streaming/typing indicator (the user waits with no feedback while the API responds), and potential XSS from unvalidated markdown containing HTML (no `rehype-sanitize`).

### Q4: Does the Pipeline bar actually update from WebSocket events?

**Yes.** The `useSessionWebSocket` hook dispatches `stage_update` events to `setStages` using a functional updater, and `PipelineBar` is a pure component receiving `stages` as a prop. Each `stage_update` message maps to an immutable state update that re-renders only the affected `StageChip`. This is correct and efficient.

### Q5: Is there any global state management beyond local `useState`?

**No.** There is no `useContext`, `useReducer`, `createContext`, Zustand, Redux, Jotai, Recoil, or MobX anywhere in the source tree. All state is local to `SessionPage` or the `useSessionWebSocket` hook. This is fine for a single-page session view but means:

- A user cannot navigate away from a session and return to find it in the same WS state.
- There is no shared auth token cache across components (each hook call to `getAccessTokenSilently` is independent, though Auth0's SDK does handle caching internally).
- If the product grows to include multi-session views or cross-page navigation, state will need to be lifted or a store added.

### Q6: Are there React testing files?

**No.** There are zero test files in `src/`. No Vitest configuration, no Jest configuration, no React Testing Library setup, no Storybook. The only test infrastructure present is in `node_modules` (Zod's own test suite). This means:

- No unit tests for `useSessionWebSocket` reconnection logic
- No component tests for `ChatPanel`, `PipelineBar`, or `MessageInput`
- No integration tests for the Auth0 flow
- No snapshot tests

### Q7: Is the CSS responsive?

**No.** `index.css` contains zero `@media` queries. All component layouts use inline `style` props with fixed pixel values (e.g., header height `52px`, pipeline bar `48px`, message input padding). On a 375px-wide mobile viewport:

- The pipeline bar's 12 stage chips will overflow their container (handled by `overflowX: auto` but not optimised)
- The header will clip user info on the right
- The chat area will be usable but unoptimised
- Font size is fixed at `14px` (not `clamp`-based)

There is no viewport meta tag concern (that belongs in `index.html`, not audited here), but the CSS itself has no responsive adaptation.

### Q8: Does the session page handle edge cases?

| Edge Case | Handled? | Notes |
|-----------|----------|-------|
| Session not found (404) | Partially | Raw API error string shown, not user-friendly |
| Network error during init | Yes | Caught, `initError` set, error UI displayed |
| Pipeline error (`status: 'failed'`) | Visually yes | `PipelineBar` shows red failed stages; no user explanation |
| WebSocket disconnect after max retries | Visually only | Header dot turns red; no action prompt |
| `sendMessage` failure | No | Error is `console.error`-only; user sees no feedback |
| Duplicate messages (REST + WS) | No | `restMessages` + `wsMessages` merged without dedup |
| Navigating away mid-pipeline | Yes | Unmount cleans up WS correctly |
| Pipeline running on page load (existing session) | Partially | `pipeline_running` restored from session, but WS stages reset to `pending` on connect |

---

## 3. Bugs & Issues Catalogue

### Critical
| # | Location | Issue |
|---|----------|-------|
| C1 | `api/client.ts` | No 401/403 handling — expired tokens produce raw error strings with no re-auth trigger |
| C2 | `pages/SessionPage.tsx` L86 | Potential duplicate messages: `sendMessage` pushes to `restMessages`, WS may also emit the same messages via `type: 'message'` events |

### High
| # | Location | Issue |
|---|----------|-------|
| H1 | `hooks/useSessionWebSocket.ts` L80 | JWT sent as query-string parameter — visible in server logs and browser history |
| H2 | `pages/Dashboard.tsx` | No session listing — users cannot see or resume existing sessions |
| H3 | `components/ChatPanel.tsx` | No `rehype-sanitize` — planner HTML content could render as raw HTML (XSS vector if LLM output is untrusted) |
| H4 | `pages/SessionPage.tsx` L81–92 | `sendMessage` errors are silently logged; user receives no visual feedback on failure |

### Medium
| # | Location | Issue |
|---|----------|-------|
| M1 | `hooks/useSessionWebSocket.ts` | After max retries, no user-visible recovery UI (toast, modal, or in-chat message) |
| M2 | `components/MessageInput.tsx` | `<textarea>` does not auto-expand — multi-line input scrolls invisibly |
| M3 | `pages/SessionPage.tsx` L74–78 | `pipelineComplete` effect has `session` as dependency, causing re-runs on every session state update — potential loop |
| M4 | `hooks/useSessionWebSocket.ts` L44 | `messages` not reset on `sessionId` change — stale messages could leak into a new session if the component is not unmounted |
| M5 | All components | No `@media` queries — layout breaks on mobile |
| M6 | `pages/SessionPage.tsx` L98–113 | 404 error message is the raw API error string — not user-friendly |

### Low
| # | Location | Issue |
|---|----------|-------|
| L1 | All interactive elements | No `aria-label`, no `role` annotations, no `aria-live` regions — poor screen-reader experience |
| L2 | `components/ChatPanel.tsx` | No scroll-position preservation — force-scrolls to bottom on every new message |
| L3 | `auth/useAuthenticatedFetch.ts` L33–36 | `getAccessTokenSilently` failures silently return `''` — masks auth errors |
| L4 | `index.css` | WebKit-only scrollbar styling; no Firefox equivalent |
| L5 | Multiple files | Hover effects via `onMouseEnter`/`onMouseLeave` with `(e.currentTarget as HTMLButtonElement)` casts — fragile; CSS `:hover` would be cleaner |
| L6 | `api/client.ts` L44 | `body: '{}'` is a hardcoded literal — inconsistent style, should be `JSON.stringify({})` |
| L7 | `pages/SessionPage.tsx` L70 | `eslint-disable` on hooks dep array hides potentially missing `api` dep — add a comment explaining stability guarantee |

---

## 4. TypeScript Quality

**Zero `any` types found.** All inferred and explicit types are concrete. Notable positives:
- `ServerWsMessage` is a proper discriminated union — `switch` dispatch is type-safe.
- `ApiClient` is derived via `ReturnType<typeof createApiClient>` — stays in sync with implementation automatically.
- `vite-env.d.ts` declares all `VITE_` variables as `readonly string` — no implicit `undefined` from `import.meta.env`.
- One unsafe cast: `JSON.parse(event.data as string)` in `useSessionWebSocket.ts` L102 — `event.data` is already typed as `any` by the browser API, so this is effectively a no-op cast. Adding Zod validation here would eliminate the possibility of a crash on unexpected server messages.

---

## 5. Dependencies Assessment

```
react: ^19.2.0           — Latest major; correct
react-dom: ^19.2.0       — Matches react
react-router-dom: ^7.13.1 — v7 (Data Router API available but not used)
@auth0/auth0-react: ^2.15.0 — Current stable
react-markdown: ^10.1.0  — Current stable; no rehype-sanitize
```

**Missing:**
- No state management library (Zustand, Jotai, etc.)
- No form library (React Hook Form, etc.) — not needed yet
- No toast/notification library (react-hot-toast, sonner, etc.) — needed for error feedback
- No test framework (Vitest, Jest, React Testing Library) — critical gap
- No `rehype-sanitize` — security gap for markdown rendering
- No `react-error-boundary` (uses hand-rolled class component instead — fine)

---

## 6. Positive Highlights

1. **Auth toggle is clean.** The `AUTH0_ENABLED` flag is resolved at module load time and used consistently across all auth-related code. Dev mode and prod mode are explicit, not hacked.
2. **No `any` types anywhere.** Strict TypeScript throughout.
3. **WebSocket hook is well-structured.** Reconnection, cleanup on unmount, and `mountedRef` guard against post-unmount state updates are all correct.
4. **Error boundary at root.** Catastrophic render errors are caught and surfaced without a white screen.
5. **Design system is consistent.** CSS variables used uniformly; colour palette and typography are cohesive.
6. **`useMemo` for API client.** `createApiClient(getToken)` is memoised — prevents unnecessary re-creation.
7. **Cancellation token in `useEffect`.** The `cancelled` flag in `SessionPage`'s init effect prevents state updates after unmount.

---

## 7. Prioritised Recommendations

### Before production launch
1. **Add 401/403 handling in `api/client.ts`** — detect the status code, call `loginWithRedirect()` (or dispatch a logout event), and show a user-friendly re-authentication prompt.
2. **Add `rehype-sanitize` to `ChatPanel`** — prevents XSS if the planner LLM ever emits HTML markup.
3. **Send WS auth token as first message, not query string** — reduces token exposure in logs.
4. **Fix duplicate messages** — define a clear contract: either use REST response OR WebSocket events for message delivery, not both. Add a message ID dedup step to `allMessages`.
5. **Surface `sendMessage` errors to the user** — add a toast/notification library and show an inline error on failure.

### High priority
6. **Implement session listing in Dashboard** — add `GET /sessions` to the API client and fetch it on Dashboard mount.
7. **Add a "reconnected / reconnection failed" notification** — after max WS retries, show an in-chat system message and a "reconnect" button.
8. **Add textarea auto-grow** — use a `useEffect` that sets `textarea.style.height = 'auto'; textarea.style.height = textarea.scrollHeight + 'px'` on every value change.

### Medium priority
9. **Add Vitest + React Testing Library** — at minimum: unit tests for `useSessionWebSocket`, component tests for `ChatPanel` and `PipelineBar`, and an integration test for the auth flow.
10. **Add responsive CSS** — at minimum a `@media (max-width: 640px)` breakpoint that wraps the pipeline bar and adjusts the header.
11. **Fix `pipelineComplete` effect dependency** — use `setPipelineComplete` and a functional updater to remove `session` from the dep array.
12. **Replace 404 raw error string** — detect `404` in the status code and show "Session not found" with a navigation prompt.

### Low priority
13. **Add ARIA attributes** — `aria-label` on the send button and textarea, `aria-live="polite"` on the connection status indicator.
14. **Add scroll-position preservation to ChatPanel** — only auto-scroll to bottom if the user is already at or near the bottom.
15. **Replace inline hover handlers with CSS classes** — cleaner and more performant than `onMouseEnter`/`onMouseLeave` with type casts.
