import { useAuth0 } from '@auth0/auth0-react';

export default function UserInfoAuth0() {
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
        background: 'var(--color-surface-2)', border: '1px solid var(--color-border)',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontSize: '12px', color: 'var(--color-primary)', fontWeight: 700,
      }}>
        {avatarLetter}
      </span>
      <span style={{ fontSize: '12px', color: 'var(--color-text-muted)' }}>{displayName}</span>
      <button
        onClick={handleLogout}
        aria-label="Log out"
        style={{
          background: 'transparent', border: '1px solid var(--color-border)',
          color: 'var(--color-text-muted)', padding: '3px 10px', fontSize: '11px',
          cursor: 'pointer', borderRadius: '2px', fontFamily: 'inherit',
          transition: 'border-color 0.18s, color 0.18s',
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--color-error)';
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--color-error)';
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLButtonElement).style.borderColor = 'var(--color-border)';
          (e.currentTarget as HTMLButtonElement).style.color = 'var(--color-text-muted)';
        }}
      >
        logout
      </button>
    </div>
  );
}
