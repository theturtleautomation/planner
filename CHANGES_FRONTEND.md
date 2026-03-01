# Frontend Completeness & Security Fixes — Change Log

All changes applied to `/home/user/workspace/planner/planner-web/src/`.
TypeScript compilation: **0 errors** (`npx tsc --noEmit`).

---

## 1. `api/client.ts` — ApiError class + listSessions + 401/403 handling

- **Removed** inline `throw new Error(...)` for HTTP failures.
- **Throws `ApiError`** (imported from `types.ts`) instead — includes `status: number` field.
- **Exported `ApiError`** and **`isAuthError(e: Error): boolean`** — checks for `status 401|403` on `ApiError`, or falls back to checking the message string.
- **Added `listSessions()`** method: `GET /api/sessions` → `ListSessionsResponse { sessions: Session[] }`.
- **Added `ListSessionsResponse`** interface export.

---

## 2. `hooks/useSessionWebSocket.ts` — Token delivery + message reset + reconnect UI

- **Imported `WS_PROTOCOL`** from `config.ts` instead of recomputing it inline.
- **Dual-mode auth**: query-string token kept for backward compat; after `ws.onopen`, also sends `{ type: 'auth', token }` as first JSON message.
- **Message reset**: `setMessages([])` added to the `sessionId` change effect, so switching sessions clears old messages.
- **`reconnectFailed` state** (`boolean`): exported in the return object. Set to `true` when `MAX_RETRIES` is exhausted; reset to `false` when `sessionId` changes.

---

## 3. `components/ChatPanel.tsx` — rehype-sanitize + scroll preservation

- **Added `rehype-sanitize` import** and passed `rehypePlugins={[rehypeSanitize]}` to `<ReactMarkdown>` to prevent XSS in planner Markdown output.
- **Scroll preservation**:
  - Added `containerRef` on the scroll container with an `onScroll` handler.
  - `userScrolled` ref tracks whether the user has scrolled up (scrollTop + clientHeight < scrollHeight - 50).
  - Auto-scroll (`scrollIntoView`) only fires when `!userScrolled.current`.
  - Scrolling back to within 50px of bottom resets `userScrolled.current = false`.

---

## 4. `components/MessageInput.tsx` — Auto-grow textarea + ARIA

- **Auto-grow**: added `textareaRef` and a `useEffect` that sets `el.style.height = 'auto'` then `Math.min(el.scrollHeight, 200) + 'px'` on every `value` change. Max height raised to `200px` (matching the cap).
- **ARIA**: `aria-label="Message input"` on `<textarea>`, `aria-label="Send message"` on `<button>`.
- **Removed ⚡ emoji** from the pipeline-running hint text.
- Height resets to `auto` after `send()` clears the value.

---

## 5. `pages/Dashboard.tsx` — Implement session listing

- Replaced static empty state with a fully dynamic session list:
  - **`useGetAccessToken` + `createApiClient`** wired up via `useMemo`.
  - On mount, calls `api.listSessions()` and stores results.
  - **Loading state**: spinner text while fetching.
  - **Error state**: red error banner if fetch fails.
  - **Empty state**: original "no sessions yet" UI preserved.
  - **Session list**: `SessionCard` component renders session ID (truncated), created-at timestamp (from first message), message count, and pipeline status. Cards are clickable and navigate to `/session/:id`.
- "New Session" button navigates to `/session/new` (unchanged behavior).

---

## 6. `pages/SessionPage.tsx` — Dedup messages + error display + pipeline fix

- **Dedup by message ID** in `allMessages` useMemo: uses a `Set<string>` to filter duplicate IDs from merged `restMessages + wsMessages`.
- **Inline send error**: `sendError` state; set to `'Failed to send message. Please try again.'` on catch; auto-clears after 5 seconds; rendered as a red banner above `<MessageInput>`.
- **404 error display**: if `initError` contains `'404'`, shows `"Session not found."` instead of the raw API error string. The "back to dashboard" button is present in both cases.
- **Pipeline completion re-render loop fix**: added `pipelineCompletedHandled` ref — the effect only calls `setSession` once per completion event; the ref resets when `pipelineComplete` goes back to `false`.
- **Imported `ApiError`** from `api/client.ts` for future use.

---

## 7. `components/Layout.tsx` — ARIA attributes

- `role="banner"` added to `<header>`.
- `flexWrap: 'wrap'` added to header style for mobile reflow.
- `aria-label="Connection status"` and `role="status"` added to the connection indicator `<span>`.
- `aria-label="Log out"` added to the logout `<button>`.

---

## 8. `src/index.css` — Responsive CSS

Added at end of file:

```css
@media (max-width: 640px) {
  :root {
    --header-height: 44px;
    --pipeline-height: auto;
  }
  body { font-size: 13px; }
}

@media (max-width: 768px) {
  .pipeline-bar { flex-wrap: wrap; gap: 4px; }
}
```

---

## 9. `auth/useAuthenticatedFetch.ts` — Token error tracking

- Added **module-level `let lastTokenError: Error | null = null`**.
- Exported **`getLastTokenError(): Error | null`** function.
- Auth0 token hook now sets `lastTokenError = error` on failure (and clears it to `null` on success).
- Existing silent-failure behavior (return `''`) preserved — callers can now check `getLastTokenError()` to inspect the cause.

---

## 10. `config.ts` — WS_PROTOCOL export

Added:
```ts
export const WS_PROTOCOL = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
```
Used by `useSessionWebSocket.ts` to build the WebSocket URL.

---

## 11. `types.ts` — ApiError class

Added at top of file:
```ts
export class ApiError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}
```
Re-exported from `api/client.ts` for backward compat.

---

## 12. `package.json` — rehype-sanitize dependency

Added to `dependencies`:
```json
"rehype-sanitize": "^6.0.0"
```

---

## TypeScript Verification

```
cd /home/user/workspace/planner/planner-web
npx tsc --noEmit
# Exit code: 0 — no errors
```
