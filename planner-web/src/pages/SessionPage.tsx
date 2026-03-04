import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import Layout from '../components/Layout.tsx';
import ChatPanel from '../components/ChatPanel.tsx';
import PipelineBar from '../components/PipelineBar.tsx';
import MessageInput from '../components/MessageInput.tsx';
import ConvergenceBar from '../components/ConvergenceBar.tsx';
import BeliefStatePanel from '../components/BeliefStatePanel.tsx';
import SpeculativeDraftView from '../components/SpeculativeDraftView.tsx';
import { createApiClient } from '../api/client.ts';
import { useGetAccessToken } from '../auth/useAuthenticatedFetch.ts';
import { useSocraticWebSocket } from '../hooks/useSocraticWebSocket.ts';
import type { Session } from '../types.ts';

export default function SessionPage() {
  const { id: routeId } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const getToken = useGetAccessToken();

  const api = useMemo(() => createApiClient(getToken), [getToken]);

  // Core session state
  const [session, setSession] = useState<Session | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(
    routeId && routeId !== 'new' ? routeId : null,
  );
  const [initError, setInitError] = useState<string | null>(null);

  // Waiting phase state
  const [description, setDescription] = useState('');
  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);

  // Right panel toggle: show draft vs. belief state
  const [showDraft, setShowDraft] = useState(false);

  // Socratic WebSocket hook
  const socratic = useSocraticWebSocket({ sessionId, getToken });

  // Auto-show draft when it arrives
  useEffect(() => {
    if (socratic.speculativeDraft) {
      setShowDraft(true);
    }
  }, [socratic.speculativeDraft]);

  // Track whether we've triggered auto-connect for an existing session
  const autoConnectAttemptedRef = useRef(false);

  // ── Init: create or load session ──
  useEffect(() => {
    let cancelled = false;
    autoConnectAttemptedRef.current = false;

    const init = async (): Promise<void> => {
      try {
        if (!routeId || routeId === 'new') {
          // Create a new session — don't auto-connect WS
          const resp = await api.createSession();
          if (cancelled) return;
          const s = resp.session ?? resp;
          setSession(s as Session);
          setSessionId((s as Session).id);
          void navigate(`/session/${(s as Session).id}`, { replace: true });
        } else {
          // Load existing session
          const resp = await api.getSession(routeId);
          if (cancelled) return;
          const s = resp.session ?? resp;
          setSession(s as Session);
          setSessionId((s as Session).id);
        }
      } catch (err) {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : String(err);
        console.error('[SessionPage] init error:', msg);
        setInitError(msg);
      }
    };

    void init();
    return () => { cancelled = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [routeId]);

  // Auto-connect WS for existing sessions that are already in progress
  useEffect(() => {
    if (!session || !sessionId || autoConnectAttemptedRef.current) return;
    const phase = session.intake_phase;
    if (phase === 'interviewing' || phase === 'pipeline_running' || phase === 'complete') {
      autoConnectAttemptedRef.current = true;
      // Seed the description if available so sendDescription can reconnect
      const desc = session.project_description ?? '';
      socratic.sendDescription(desc);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session, sessionId]);

  // ── Description submission ──
  const handleStartInterview = useCallback(async (): Promise<void> => {
    if (!sessionId || !description.trim()) return;
    setIsStarting(true);
    setStartError(null);
    try {
      // 1. Create the server-side Socratic session
      await api.startSocratic(sessionId, description.trim());
      // 2. Connect WS and send initial description
      socratic.sendDescription(description.trim());
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      console.error('[SessionPage] startSocratic error:', msg);
      setStartError('Failed to start interview. Please try again.');
      setTimeout(() => setStartError(null), 6000);
    } finally {
      setIsStarting(false);
    }
  }, [sessionId, description, api, socratic]);

  // Textarea auto-grow ref
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    const el = textareaRef.current;
    if (el) {
      el.style.height = 'auto';
      el.style.height = Math.min(el.scrollHeight, 300) + 'px';
    }
  }, [description]);

  // ── Effective intake phase ──
  // Use the WS hook's phase once connected; fall back to the session's stored phase
  const effectivePhase = socratic.intakePhase !== 'waiting'
    ? socratic.intakePhase
    : (session?.intake_phase ?? 'waiting');

  // ── Right panel content ──
  const rightPanelContent = (() => {
    if (showDraft && socratic.speculativeDraft) {
      return (
        <SpeculativeDraftView
          draft={socratic.speculativeDraft}
          onBack={() => setShowDraft(false)}
          onReact={socratic.sendDraftReaction}
        />
      );
    }
    return (
      <BeliefStatePanel
        beliefState={socratic.beliefState}
        classification={socratic.classification}
        contradictions={socratic.contradictions}
        onDimensionEdit={socratic.sendDimensionEdit}
      />
    );
  })();

  // ── Error state ──
  if (initError) {
    const is404 = initError.includes('404');
    return (
      <Layout>
        <div style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '12px',
          padding: '24px',
        }}>
          <span style={{ color: 'var(--accent-red)', fontSize: '14px' }}>[ ERROR ]</span>
          <span style={{ color: 'var(--text-secondary)', fontSize: '13px', textAlign: 'center', maxWidth: '500px' }}>
            {is404 ? 'Session not found.' : initError}
          </span>
          <button
            onClick={() => void navigate('/')}
            style={{
              marginTop: '8px',
              background: 'transparent',
              border: '1px solid var(--border)',
              color: 'var(--text-secondary)',
              padding: '7px 16px',
              fontSize: '12px',
              cursor: 'pointer',
              fontFamily: 'inherit',
              borderRadius: '2px',
            }}
          >
            ← back to dashboard
          </button>
        </div>
      </Layout>
    );
  }

  // ── Loading state ──
  if (!sessionId && !initError) {
    return (
      <Layout>
        <div style={{
          flex: 1,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: 'var(--text-secondary)',
          fontSize: '13px',
        }}>
          initializing session…
        </div>
      </Layout>
    );
  }

  // ── Reconnect failure banner ──
  const reconnectBanner = socratic.reconnectFailed && (
    <div style={{
      padding: '8px 16px',
      background: 'rgba(255,68,68,0.10)',
      borderBottom: '1px solid var(--accent-red)',
      color: 'var(--accent-red)',
      fontSize: '12px',
      textAlign: 'center',
      flexShrink: 0,
    }}>
      Connection lost. Please refresh to reconnect.
    </div>
  );

  // ─────────────────────────────────────────────────────────────────
  // PHASE: waiting — show description form
  // ─────────────────────────────────────────────────────────────────
  if (effectivePhase === 'waiting') {
    return (
      <Layout sessionId={sessionId} isConnected={false}>
        <div style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
        }}>
          {reconnectBanner}
          <div style={{
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '24px',
            overflow: 'auto',
          }}>
            <div style={{
              width: '100%',
              maxWidth: '600px',
              background: 'var(--bg-secondary)',
              border: '1px solid var(--border)',
              borderRadius: '4px',
              padding: '28px 32px',
              display: 'flex',
              flexDirection: 'column',
              gap: '16px',
            }}>
              {/* Header */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <span style={{
                  fontSize: '11px',
                  fontWeight: 700,
                  letterSpacing: '0.1em',
                  textTransform: 'uppercase',
                  color: 'var(--accent-cyan)',
                }}>
                  Planner v2
                </span>
                <h2 style={{
                  margin: 0,
                  fontSize: '18px',
                  fontWeight: 700,
                  color: 'var(--text-primary)',
                  fontFamily: 'inherit',
                }}>
                  Describe your project
                </h2>
                <p style={{
                  margin: 0,
                  fontSize: '13px',
                  color: 'var(--text-secondary)',
                  lineHeight: '1.5',
                }}>
                  Give a brief overview — what you want to build, who it's for, and any important constraints. We'll ask focused questions to fill in the details.
                </p>
              </div>

              {/* Textarea */}
              <div style={{
                background: 'var(--bg-tertiary)',
                border: `1px solid ${description.trim() ? 'var(--accent-cyan)' : 'var(--border)'}`,
                borderRadius: '3px',
                padding: '10px 14px',
                transition: 'border-color 0.18s',
              }}>
                <textarea
                  ref={textareaRef}
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.shiftKey && !isStarting && description.trim()) {
                      e.preventDefault();
                      void handleStartInterview();
                    }
                  }}
                  disabled={isStarting}
                  placeholder="e.g. A multi-tenant SaaS dashboard for tracking equipment rentals, with role-based access for admins and field staff…"
                  rows={4}
                  aria-label="Project description"
                  style={{
                    width: '100%',
                    background: 'transparent',
                    border: 'none',
                    outline: 'none',
                    color: isStarting ? 'var(--text-secondary)' : 'var(--text-primary)',
                    fontSize: '13px',
                    lineHeight: '1.6',
                    resize: 'none',
                    minHeight: '90px',
                    maxHeight: '300px',
                    overflowY: 'auto',
                    fontFamily: 'inherit',
                    boxSizing: 'border-box',
                    cursor: isStarting ? 'not-allowed' : 'text',
                  }}
                />
              </div>

              {/* Character count */}
              <div style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
              }}>
                <span style={{
                  fontSize: '11px',
                  color: description.length > 2000 ? 'var(--accent-red)' : 'var(--text-secondary)',
                }}>
                  {description.length} chars
                  {description.length > 2000 && ' — try to keep it concise'}
                </span>
                <span style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>
                  Enter to submit
                </span>
              </div>

              {/* Error */}
              {startError && (
                <div style={{
                  padding: '8px 12px',
                  background: 'rgba(255,68,68,0.10)',
                  border: '1px solid var(--accent-red)',
                  borderRadius: '3px',
                  color: 'var(--accent-red)',
                  fontSize: '12px',
                }}>
                  {startError}
                </div>
              )}

              {/* Submit button */}
              <button
                onClick={() => void handleStartInterview()}
                disabled={!description.trim() || isStarting}
                style={{
                  alignSelf: 'flex-end',
                  background: !description.trim() || isStarting
                    ? 'transparent'
                    : 'var(--accent-cyan)',
                  border: `1px solid ${!description.trim() || isStarting
                    ? 'var(--border)'
                    : 'var(--accent-cyan)'}`,
                  color: !description.trim() || isStarting
                    ? 'var(--text-secondary)'
                    : 'var(--bg-primary)',
                  padding: '8px 20px',
                  fontSize: '13px',
                  fontWeight: 700,
                  fontFamily: 'inherit',
                  borderRadius: '3px',
                  cursor: !description.trim() || isStarting ? 'not-allowed' : 'pointer',
                  transition: 'background 0.18s, border-color 0.18s, color 0.18s',
                  letterSpacing: '0.03em',
                }}
              >
                {isStarting ? 'Starting…' : 'Start Interview →'}
              </button>
            </div>
          </div>

          {/* Disabled MessageInput for consistent chrome */}
          <MessageInput
            onSend={() => undefined}
            disabled={true}
            intakePhase="waiting"
          />
        </div>
      </Layout>
    );
  }

  // ─────────────────────────────────────────────────────────────────
  // PHASE: interviewing or pipeline_running or complete
  // All share the split-pane layout
  // ─────────────────────────────────────────────────────────────────
  const isInterviewing = effectivePhase === 'interviewing';
  const isPipelineRunning = effectivePhase === 'pipeline_running';
  const isComplete = effectivePhase === 'complete';

  return (
    <Layout sessionId={sessionId} isConnected={socratic.isConnected}>
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        overflow: 'hidden',
      }}>
        {reconnectBanner}

        {/* Top bar: ConvergenceBar (interviewing) or PipelineBar (pipeline_running / complete) */}
        {isInterviewing && (
          <ConvergenceBar
            convergencePct={socratic.convergencePct * 100}
            classification={socratic.classification}
          />
        )}
        {(isPipelineRunning || isComplete) && (
          <PipelineBar stages={socratic.stages} />
        )}

        {/* Split pane */}
        <div className="split-pane">
          {/* Left: Chat + Input */}
          <div className="pane-left">
            <ChatPanel messages={socratic.messages} />

            {/* Pipeline complete summary */}
            {isComplete && socratic.pipelineSummary && (
              <div style={{
                padding: '12px 16px',
                background: 'rgba(0,255,136,0.06)',
                borderTop: '1px solid var(--accent-green)',
                color: 'var(--accent-green)',
                fontSize: '12px',
                flexShrink: 0,
                lineHeight: '1.5',
              }}>
                <span style={{ fontWeight: 700, letterSpacing: '0.04em' }}>Pipeline complete — </span>
                {socratic.pipelineSummary}
              </div>
            )}

            <MessageInput
              onSend={(content) => socratic.sendResponse(content)}
              intakePhase={socratic.intakePhase}
              currentQuestion={socratic.currentQuestion}
              onSkip={socratic.skipQuestion}
              onDone={socratic.sendDone}
              disabled={!socratic.isConnected}
              pipelineRunning={isPipelineRunning}
              convergencePct={socratic.convergencePct * 100}
            />
          </div>

          {/* Right: BeliefStatePanel or SpeculativeDraftView */}
          <div className="pane-right">
            {/* Draft toggle hint when draft is available but we're viewing belief state */}
            {socratic.speculativeDraft && !showDraft && (
              <div style={{
                padding: '6px 14px',
                background: 'rgba(0,212,255,0.06)',
                borderBottom: '1px solid var(--border)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                flexShrink: 0,
              }}>
                <span style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>
                  Speculative draft available
                </span>
                <button
                  onClick={() => setShowDraft(true)}
                  style={{
                    background: 'transparent',
                    border: '1px solid var(--accent-cyan)',
                    borderRadius: '3px',
                    color: 'var(--accent-cyan)',
                    fontSize: '10px',
                    fontFamily: 'inherit',
                    letterSpacing: '0.04em',
                    padding: '3px 10px',
                    cursor: 'pointer',
                  }}
                >
                  View Draft
                </button>
              </div>
            )}
            {rightPanelContent}
          </div>
        </div>
      </div>
    </Layout>
  );
}
