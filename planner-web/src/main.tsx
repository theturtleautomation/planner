import { StrictMode, Component } from 'react';
import type { ReactNode, ErrorInfo } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import './index.css';
import App from './App.tsx';
import Auth0ProviderWithNavigate from './auth/Auth0ProviderWithNavigate.tsx';
import { AUTH0_ENABLED } from './config.ts';

// ─── Error Boundary ───────────────────────────────────────────────────────────

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class ErrorBoundary extends Component<{ children: ReactNode }, ErrorBoundaryState> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error('[ErrorBoundary]', error, info);
  }

  render(): ReactNode {
    if (this.state.hasError) {
      return (
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          height: '100vh',
          background: '#0a0a0f',
          color: '#ff4444',
          fontFamily: 'monospace',
          gap: '16px',
          padding: '24px',
        }}>
          <div style={{ fontSize: '18px' }}>[ FATAL ERROR ]</div>
          <div style={{ color: '#e0e0e8', fontSize: '13px', maxWidth: '600px', textAlign: 'center' }}>
            {this.state.error?.message ?? 'An unexpected error occurred.'}
          </div>
          <button
            onClick={() => window.location.reload()}
            style={{
              background: 'transparent',
              border: '1px solid #ff4444',
              color: '#ff4444',
              padding: '8px 20px',
              cursor: 'pointer',
              fontFamily: 'monospace',
            }}
          >
            reload
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

// ─── Mount ─────────────────────────────────────────────────────────────────────

const rootEl = document.getElementById('root');
if (!rootEl) throw new Error('Root element not found');

createRoot(rootEl).render(
  <StrictMode>
    <ErrorBoundary>
      <BrowserRouter>
        {AUTH0_ENABLED ? (
          <Auth0ProviderWithNavigate>
            <App />
          </Auth0ProviderWithNavigate>
        ) : (
          <App />
        )}
      </BrowserRouter>
    </ErrorBoundary>
  </StrictMode>,
);
