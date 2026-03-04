import { lazy, Suspense } from 'react';
import type { ReactNode } from 'react';
import { AUTH0_ENABLED } from '../config.ts';

interface Props {
  children: ReactNode;
}

// ─── Dev mode: no auth ────────────────────────────────────────────────────────
function DevRoute({ children }: Props) {
  return <>{children}</>;
}

// ─── Auth0 mode: lazy-loaded to avoid importing @auth0/auth0-react ───────────
const LazyAuth0Route = lazy(() =>
  import('./Auth0Route.tsx').then((m) => ({ default: m.default }))
);

// ─── Export ───────────────────────────────────────────────────────────────────
export default function ProtectedRoute({ children }: Props) {
  if (AUTH0_ENABLED) {
    return (
      <Suspense fallback={
        <div style={{
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          height: '100vh', background: '#0a0a0f', color: '#8888a0',
          fontFamily: 'monospace', fontSize: '13px',
        }}>
          authenticating…
        </div>
      }>
        <LazyAuth0Route>{children}</LazyAuth0Route>
      </Suspense>
    );
  }
  return <DevRoute>{children}</DevRoute>;
}
