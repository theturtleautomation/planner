import { lazy, Suspense } from 'react';
import type { ReactNode } from 'react';
import { useLocation, Link } from 'react-router-dom';
import { AUTH0_ENABLED } from '../config.ts';
import { useTheme } from '../hooks/useTheme.tsx';

interface LayoutProps {
  children: ReactNode;
  sessionId?: string | null;
  isConnected?: boolean;
}

// Lazy-load Auth0-dependent component
const UserInfoAuth0 = lazy(() => import('./UserInfoAuth0.tsx'));

function UserInfoDev() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
      <span style={{
        width: '24px', height: '24px', borderRadius: '50%',
        background: 'var(--color-surface-dynamic)', border: '1px solid var(--color-border)',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontSize: '10px', color: 'var(--color-gold)', fontWeight: 700,
      }}>
        D
      </span>
      <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-gold)' }}>dev</span>
    </div>
  );
}

function UserInfo() {
  if (AUTH0_ENABLED) {
    return (
      <Suspense fallback={<span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>…</span>}>
        <UserInfoAuth0 />
      </Suspense>
    );
  }
  return <UserInfoDev />;
}

type NavSection = 'primary' | 'secondary' | 'utility';

interface NavItem {
  label: string;
  path: string;
  icon: string;
  section: NavSection;
}

const NAV_ITEMS: NavItem[] = [
  { label: 'Home', path: '/', icon: 'home', section: 'primary' },
  { label: 'Projects', path: '/projects', icon: 'folder', section: 'primary' },
  { label: 'Knowledge', path: '/knowledge', icon: 'book', section: 'primary' },
  { label: 'Sessions', path: '/sessions', icon: 'clock', section: 'secondary' },
  { label: 'Events', path: '/events', icon: 'activity', section: 'secondary' },
  { label: 'Admin', path: '/admin', icon: 'terminal', section: 'secondary' },
  { label: 'Discovery', path: '/discovery', icon: 'search', section: 'utility' },
  { label: 'Blueprint', path: '/blueprint', icon: 'globe', section: 'utility' },
];

function isActiveNavItem(pathname: string, item: NavItem): boolean {
  if (item.path === '/') return pathname === '/';
  if (item.path === '/projects') return pathname.startsWith('/projects');
  if (item.path === '/sessions') {
    return pathname === '/sessions'
      || pathname === '/session/new'
      || pathname.startsWith('/session/');
  }
  if (item.path === '/knowledge') return pathname.startsWith('/knowledge');
  if (item.path === '/events') return pathname.startsWith('/events');
  return pathname === item.path || pathname.startsWith(`${item.path}/`);
}

function SidebarIcon({ name }: { name: string }) {
  const stroke = 'currentColor';
  const props = { width: 14, height: 14, viewBox: '0 0 24 24', fill: 'none', stroke, strokeWidth: 2, strokeLinecap: 'round' as const, strokeLinejoin: 'round' as const };

  switch (name) {
    case 'home':
      return <svg {...props}><path d="M3 10l9-7 9 7"/><path d="M9 21V12h6v9"/></svg>;
    case 'folder':
      return <svg {...props}><path d="M3 6h5l2 2h11v10a2 2 0 01-2 2H5a2 2 0 01-2-2z"/></svg>;
    case 'clock':
      return <svg {...props}><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>;
    case 'globe':
      return <svg {...props}><circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 000 20M2 12h20"/></svg>;
    case 'book':
      return <svg {...props}><path d="M4 19.5A2.5 2.5 0 016.5 17H20"/><path d="M4 4.5A2.5 2.5 0 016.5 2H20v20H6.5A2.5 2.5 0 014 19.5v-15z"/></svg>;
    case 'terminal':
      return <svg {...props}><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>;
    case 'activity':
      return <svg {...props}><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>;
    case 'search':
      return <svg {...props}><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>;
    default:
      return <svg {...props}><circle cx="12" cy="12" r="4"/></svg>;
  }
}

function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();

  return (
    <button
      className="theme-toggle"
      onClick={toggleTheme}
      aria-label={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}
    >
      {theme === 'dark' ? (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <circle cx="12" cy="12" r="5"/>
          <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
        </svg>
      ) : (
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
      )}
    </button>
  );
}

export default function Layout({ children, sessionId, isConnected }: LayoutProps) {
  const location = useLocation();
  const primaryItems = NAV_ITEMS.filter((item) => item.section === 'primary');
  const secondaryItems = NAV_ITEMS.filter((item) => item.section === 'secondary');
  const utilityItems = NAV_ITEMS.filter((item) => item.section === 'utility');

  const renderSection = (label: string, items: NavItem[]) => (
    <div className="sidebar-section">
      <div className="sidebar-label">{label}</div>
      {items.map((item) => {
        const isActive = isActiveNavItem(location.pathname, item);
        return (
          <Link
            key={item.path}
            to={item.path}
            className={`sidebar-item${isActive ? ' active' : ''}`}
            style={{ textDecoration: 'none' }}
          >
            <span className="icon">
              <SidebarIcon name={item.icon} />
            </span>
            {item.label}
          </Link>
        );
      })}
    </div>
  );

  return (
    <div className="app-shell">
      {/* Sidebar */}
      <aside className="sidebar">
        <div className="sidebar-brand">
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" aria-label="Planner logo">
            <rect x="2" y="2" width="20" height="20" rx="4" stroke="currentColor" strokeWidth="1.5"/>
            <path d="M7 8h10M7 12h7M7 16h4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
            <circle cx="18" cy="16" r="2" fill="var(--color-primary)"/>
          </svg>
          <span className="sidebar-wordmark">Planner</span>
        </div>

        {renderSection('Primary', primaryItems)}
        {renderSection('Secondary', secondaryItems)}
        {renderSection('Utility', utilityItems)}

        <div className="sidebar-spacer" />

        {/* Bottom: status + user info + theme toggle */}
        <div className="sidebar-section" style={{ marginTop: 0 }}>
          {sessionId !== undefined && (
            <div style={{
              display: 'flex', alignItems: 'center', gap: 'var(--space-2)',
              padding: 'var(--space-1) var(--space-2)',
              marginBottom: 'var(--space-2)',
            }}>
              <span style={{
                width: '8px', height: '8px', borderRadius: '50%', display: 'inline-block',
                background: isConnected ? 'var(--color-success)' : 'var(--color-error)',
                ...(isConnected ? {} : { animation: 'blink 1.5s ease infinite' }),
              }} />
              <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
                {isConnected ? 'connected' : 'disconnected'}
              </span>
            </div>
          )}
          <div style={{
            display: 'flex', alignItems: 'center', justifyContent: 'space-between',
            padding: 'var(--space-1) var(--space-2)',
          }}>
            <UserInfo />
            <ThemeToggle />
          </div>
        </div>
      </aside>

      {/* Main content */}
      <main style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
        {children}
      </main>
    </div>
  );
}
