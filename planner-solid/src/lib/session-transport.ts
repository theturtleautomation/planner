import type {
  ClientPromptResponseMessage,
  ClientStartSocraticMessage,
  PromptBankResponse,
} from "./types";
import { completeMockSessionPrompt, startMockSessionInterview } from "./mock/store";
import { isFrontendMockEnabled } from "./mock/runtime";

export const SESSION_TRANSPORT_CONNECTING = 0;
export const SESSION_TRANSPORT_OPEN = 1;
export const SESSION_TRANSPORT_CLOSING = 2;
export const SESSION_TRANSPORT_CLOSED = 3;

export interface SessionTransportMessageEvent {
  data: string;
}

export interface SessionTransport {
  readyState: number;
  onopen: (() => void) | null;
  onerror: (() => void) | null;
  onclose: (() => void) | null;
  onmessage: ((event: SessionTransportMessageEvent) => void) | null;
  send(data: string): void;
  close(): void;
}

function buildLiveSocraticWebSocketUrl(sessionId: string): string {
  const url = new URL(window.location.origin);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.pathname = `/api/sessions/${encodeURIComponent(sessionId)}/socratic/ws`;
  return url.toString();
}

class MockSessionTransport implements SessionTransport {
  readyState = SESSION_TRANSPORT_CONNECTING;
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onmessage: ((event: SessionTransportMessageEvent) => void) | null = null;

  constructor(private readonly sessionId: string) {
    queueMicrotask(() => {
      if (this.readyState !== SESSION_TRANSPORT_CONNECTING) {
        return;
      }
      this.readyState = SESSION_TRANSPORT_OPEN;
      this.onopen?.();
    });
  }

  send(data: string): void {
    if (this.readyState !== SESSION_TRANSPORT_OPEN) {
      return;
    }

    const payload = JSON.parse(data) as ClientStartSocraticMessage | ClientPromptResponseMessage;
    if (payload.type === "start_socratic") {
      const bank = startMockSessionInterview(this.sessionId, payload.description);
      this.emit({ type: "prompt_bank", bank });
      return;
    }

    if (payload.type === "prompt_response") {
      completeMockSessionPrompt(this.sessionId, payload.prompt_id, payload.answers);
      this.emit({ type: "converged" });
    }
  }

  close(): void {
    if (this.readyState === SESSION_TRANSPORT_CLOSED) {
      return;
    }
    this.readyState = SESSION_TRANSPORT_CLOSING;
    queueMicrotask(() => {
      this.readyState = SESSION_TRANSPORT_CLOSED;
      this.onclose?.();
    });
  }

  private emit(payload: { type: string; bank?: PromptBankResponse }) {
    queueMicrotask(() => {
      this.onmessage?.({ data: JSON.stringify(payload) });
    });
  }
}

export function createSessionTransport(sessionId: string): SessionTransport {
  if (isFrontendMockEnabled()) {
    return new MockSessionTransport(sessionId);
  }
  return new WebSocket(buildLiveSocraticWebSocketUrl(sessionId)) as SessionTransport;
}
