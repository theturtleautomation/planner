import { useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';

export default function Dashboard() {
  const navigate = useNavigate();

  const handleNewSession = (): void => {
    void navigate('/session/new');
  };

  return (
    <Layout>
      <div style={{
        flex: 1,
        overflow: 'auto',
        padding: '32px 24px',
        display: 'flex',
        flexDirection: 'column',
        gap: '24px',
        maxWidth: '800px',
        margin: '0 auto',
        width: '100%',
      }}>
        {/* Section header */}
        <div style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderBottom: '1px solid var(--border)',
          paddingBottom: '12px',
        }}>
          <span style={{ color: 'var(--text-primary)', fontSize: '14px', fontWeight: 600 }}>
            sessions
          </span>
          <button
            onClick={handleNewSession}
            style={{
              background: 'var(--accent-cyan)',
              border: 'none',
              color: 'var(--bg-primary)',
              padding: '7px 18px',
              fontSize: '12px',
              fontWeight: 700,
              cursor: 'pointer',
              letterSpacing: '0.05em',
              textTransform: 'uppercase',
              borderRadius: '2px',
              fontFamily: 'inherit',
              transition: 'opacity 0.18s',
            }}
            onMouseEnter={(e) => { (e.currentTarget as HTMLButtonElement).style.opacity = '0.85'; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLButtonElement).style.opacity = '1'; }}
          >
            + new session
          </button>
        </div>

        {/* Empty state */}
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          padding: '60px 24px',
          border: '1px dashed var(--border)',
          borderRadius: '3px',
          gap: '12px',
        }}>
          <span style={{ color: 'var(--text-secondary)', fontSize: '13px' }}>
            no sessions yet
          </span>
          <span style={{ color: 'var(--text-secondary)', fontSize: '12px' }}>
            create a new session to start planning
          </span>
          <button
            onClick={handleNewSession}
            style={{
              marginTop: '8px',
              background: 'transparent',
              border: '1px solid var(--accent-cyan)',
              color: 'var(--accent-cyan)',
              padding: '8px 20px',
              fontSize: '12px',
              cursor: 'pointer',
              borderRadius: '2px',
              fontFamily: 'inherit',
              transition: 'background 0.18s',
            }}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLButtonElement).style.background = 'rgba(0,212,255,0.08)';
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLButtonElement).style.background = 'transparent';
            }}
          >
            start new session →
          </button>
        </div>

        {/* Info box */}
        <div style={{
          padding: '14px 16px',
          background: 'var(--bg-secondary)',
          border: '1px solid var(--border)',
          borderRadius: '3px',
          fontSize: '12px',
          color: 'var(--text-secondary)',
          lineHeight: 1.7,
        }}>
          <span style={{ color: 'var(--accent-cyan)', fontWeight: 600 }}>TIP</span>
          {' '}— Each session maintains its own conversation history and pipeline state.
          Sessions are isolated and can be resumed at any time.
        </div>
      </div>
    </Layout>
  );
}
