import type { ReactNode } from 'react';
import { useAuth0 } from '@auth0/auth0-react';
import { AUTH0_ENABLED } from '../config.ts';

interface LayoutProps {
  children: ReactNode;
  sessionId?: string | null;
  isConnected?: boolean;
}

// ─── Auth0 header user info ───────────────────────────────────────────────────
function UserInfoAuth0() {
  const { user, logout } = useAuth0();

  const displayName = user?.name ?? user?.email ?? 'user';
  const avatarLetter = displayName.charAt(0).toUpperCase();

  const handleLogout = (): void => {
    void logout({ logoutParams: { returnTo: window.location.origin } });
  };

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
      <span style={{
        width: '28px', height: '28px', borderRadius: '50%',
        background: 'var(--bg-tertiary)', border: '1px solid var(--border)',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontSize: '12px', color: 'var(--accent-cyan)', fontWeight: 700,
      }}>
        {avatarLetter}
      </span>
      <span style={{ fontSize: '12px', color: 'var(--text-secondary)' }}>{displayName}</span>
      <button
        onClick={handleLogout}
        aria-label="Log out"
        style={{
          background: 'transparent', border: '1px solid var(--border)',
          color: 'var(--text-secondary)', padding: '3px 10px', fontSize: '11px',
          cursor: 'pointer', borderRadius: '2px', fontFamily: 'inherit',
          transition: 'border-color 0.18s, color 0.18s',
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--accent-red)';
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--accent-red)';
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--border)';
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--text-secondary)';
        }}
      >
        logout
      </button>
    </div>
  );
}

function UserInfoDev() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
      <span style={{
        width: '28px', height: '28px', borderRadius: '50%',
        background: 'var(--bg-tertiary)', border: '1px solid var(--border)',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontSize: '11px', color: 'var(--accent-yellow)', fontWeight: 700,
      }}>
        D
      </span>
      <span style={{ fontSize: '12px', color: 'var(--accent-yellow)' }}>dev mode</span>
    </div>
  );
}

function UserInfo() {
  if (AUTH0_ENABLED) return <UserInfoAuth0 />;
  return <UserInfoDev />;
}

// ─── ASCII Banner ────────────────────────────────────────────────────────────
const ASCII_BANNER = `
 ██████╗ ██╗      █████╗ ███╗   ██╗███╗   ██╗███████╗██████╗
 ██╔══██╗██║     ██╔══██╗████╗  ██║████╗  ██║██╔════╝██╔══██╗
 ██████╔╝██║     ███████║██╔██╗ ██║██╔██╗ ██║█████╗  ██████╔╝
 ██╔═══╝ ██║     ██╔══██║██║╚██╗██║██║╚██╗██║██╔══╝  ██╔══██╗
 ██║     ███████╗██║  ██║██║ ╚████║██║ ╚████║███████╗██║  ██║
 ╚═╝     ╚══════╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝`.trimStart();

// ─── Layout ───────────────────────────────────────────────────────────────────
export default function Layout({ children, sessionId, isConnected }: LayoutProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: 'var(--bg-primary)' }}>
      {/* ── Banner ── */}
      <header
        role="banner"
        style={{
          borderBottom: '1px solid var(--border)',
          background: 'var(--bg-secondary)',
          flexShrink: 0,
        }}
      >
        {/* ASCII art — centered */}
        <pre
          aria-label="Planner"
          style={{
            color: 'var(--accent-cyan)',
            textAlign: 'center',
            fontSize: 'clamp(5px, 1.3vw, 12px)',
            lineHeight: 1.2,
            margin: '12px 0 4px 0',
            padding: 0,
            userSelect: 'none',
            overflow: 'hidden',
          }}
        >
          {ASCII_BANNER}
        </pre>

        {/* Status bar */}
        <div
          style={{
            display: 'flex', alignItems: 'center', justifyContent: 'space-between',
            padding: '0 20px', height: '32px',
            borderTop: '1px solid var(--border)',
          }}
        >
          {/* Left — subtitle + session */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <span style={{ color: 'var(--text-secondary)', fontSize: '11px', letterSpacing: '0.08em' }}>
              SOCRATIC LOBBY
            </span>
            <span style={{ color: 'var(--text-secondary)', fontSize: '11px' }}>v2</span>
            {sessionId && (
              <span style={{
                color: 'var(--text-secondary)', fontSize: '11px',
                background: 'var(--bg-tertiary)', padding: '2px 8px',
                borderRadius: '2px', border: '1px solid var(--border)',
              }}>
                session: {sessionId.slice(0, 8)}…
              </span>
            )}
          </div>

          {/* Right — connection + user */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            {sessionId !== undefined && (
              <span
                aria-label="Connection status"
                role="status"
                style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11px', color: 'var(--text-secondary)' }}
              >
                <span style={{
                  width: '8px', height: '8px', borderRadius: '50%', display: 'inline-block',
                  background: isConnected ? 'var(--accent-green)' : 'var(--accent-red)',
                  ...(isConnected ? {} : { animation: 'blink 1.5s ease infinite' }),
                }} />
                {isConnected ? 'connected' : 'disconnected'}
              </span>
            )}
            <UserInfo />
          </div>
        </div>
      </header>

      {/* ── Main content ── */}
      <main style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
        {children}
      </main>
    </div>
  );
}
