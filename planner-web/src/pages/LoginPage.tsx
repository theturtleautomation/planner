import { useAuth0 } from '@auth0/auth0-react';
import { useNavigate } from 'react-router-dom';
import { AUTH0_ENABLED } from '../config.ts';

// ─── Auth0 version ────────────────────────────────────────────────────────────
function LoginPageAuth0() {
  const { loginWithRedirect } = useAuth0();

  const handleLogin = (): void => {
    void loginWithRedirect();
  };

  return <LoginView onLogin={handleLogin} />;
}

// ─── Dev mode version ─────────────────────────────────────────────────────────
function LoginPageDev() {
  const navigate = useNavigate();

  const handleEnter = (): void => {
    void navigate('/session/new');
  };

  return <LoginView onLogin={handleEnter} />;
}

// ─── Router ───────────────────────────────────────────────────────────────────
export default function LoginPage() {
  if (AUTH0_ENABLED) {
    return <LoginPageAuth0 />;
  }
  return <LoginPageDev />;
}

// ─── Shared view ─────────────────────────────────────────────────────────────
function LoginView({ onLogin }: { onLogin: () => void }) {
  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100vh',
      background: 'var(--bg-primary)',
      padding: '24px',
    }}>
      {/* Terminal window */}
      <div style={{
        width: '100%',
        maxWidth: '480px',
        border: '1px solid var(--border)',
        borderRadius: '4px',
        overflow: 'hidden',
      }}>
        {/* Title bar */}
        <div style={{
          background: 'var(--bg-tertiary)',
          padding: '10px 16px',
          borderBottom: '1px solid var(--border)',
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
        }}>
          <span style={{ width: '10px', height: '10px', borderRadius: '50%', background: 'var(--accent-red)', display: 'inline-block' }} />
          <span style={{ width: '10px', height: '10px', borderRadius: '50%', background: 'var(--accent-yellow)', display: 'inline-block' }} />
          <span style={{ width: '10px', height: '10px', borderRadius: '50%', background: 'var(--accent-green)', display: 'inline-block' }} />
          <span style={{ marginLeft: '8px', color: 'var(--text-secondary)', fontSize: '11px' }}>
            planner-v2 — socratic-lobby
          </span>
        </div>

        {/* Body */}
        <div style={{ padding: '32px 28px', background: 'var(--bg-secondary)' }}>
          <div style={{
            color: 'var(--accent-cyan)',
            fontSize: '22px',
            fontWeight: 700,
            letterSpacing: '0.04em',
            marginBottom: '8px',
            lineHeight: 1.3,
          }}>
            PLANNER v2
          </div>

          <p style={{
            color: 'var(--text-secondary)',
            fontSize: '13px',
            marginBottom: '28px',
            lineHeight: 1.8,
          }}>
            A Socratic AI planning tool. Ask questions, receive structured plans,
            and watch the pipeline transform your ideas into actionable outputs.
          </p>

          {/* Features */}
          <div style={{ marginBottom: '28px', display: 'flex', flexDirection: 'column', gap: '6px' }}>
            {[
              '→  real-time pipeline visualization',
              '→  socratic dialogue to refine plans',
              '→  12-stage compilation pipeline',
              '→  live WebSocket updates',
            ].map((line) => (
              <span key={line} style={{ color: 'var(--text-secondary)', fontSize: '12px' }}>{line}</span>
            ))}
          </div>

          {/* CTA */}
          <button
            onClick={onLogin}
            style={{
              width: '100%',
              padding: '12px',
              background: 'var(--accent-cyan)',
              border: 'none',
              color: 'var(--bg-primary)',
              fontSize: '13px',
              fontWeight: 700,
              cursor: 'pointer',
              letterSpacing: '0.06em',
              textTransform: 'uppercase',
              borderRadius: '2px',
              fontFamily: 'inherit',
              transition: 'opacity 0.18s',
            }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.opacity = '0.85'; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.opacity = '1'; }}
          >
            {AUTH0_ENABLED ? 'sign in' : 'enter  (dev mode)'}
          </button>

          {!AUTH0_ENABLED && (
            <p style={{ marginTop: '10px', color: 'var(--accent-yellow)', fontSize: '11px', textAlign: 'center' }}>
              Auth0 not configured — running without authentication
            </p>
          )}
        </div>
      </div>

      {AUTH0_ENABLED && (
        <p style={{ marginTop: '16px', color: 'var(--text-secondary)', fontSize: '11px' }}>
          secured by Auth0
        </p>
      )}
    </div>
  );
}
