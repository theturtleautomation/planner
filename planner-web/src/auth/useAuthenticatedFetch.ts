import { useAuth0 } from '@auth0/auth0-react';
import { useCallback } from 'react';
import { AUTH0_ENABLED, AUTH0_AUDIENCE } from '../config.ts';

type GetTokenFn = () => Promise<string>;

// ─── Module-level token error tracking ───────────────────────────────────────

let lastTokenError: Error | null = null;

/** Returns the last error encountered when fetching an access token, if any. */
export function getLastTokenError(): Error | null {
  return lastTokenError;
}

/**
 * Hook that returns a function to get the Auth0 access token.
 *
 * When AUTH0_ENABLED is false (dev mode), the token is always an empty string.
 * This hook is safe to call unconditionally — when Auth0 is disabled, we still
 * call useAuth0() but it will be wrapped in the Auth0Provider only when enabled.
 *
 * IMPORTANT: When AUTH0_ENABLED=false, do NOT call useAuth0(). Instead we
 * expose a no-op via a separate exported function selected at module level.
 */

// Module-level selection: export the right hook based on compile-time flag.
// This avoids conditional hook calls at the component level.
export const useGetAccessToken: () => GetTokenFn = AUTH0_ENABLED
  ? function useGetAccessTokenAuth0(): GetTokenFn {
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const { getAccessTokenSilently } = useAuth0();
      // eslint-disable-next-line react-hooks/rules-of-hooks
      return useCallback(async (): Promise<string> => {
        try {
          const token = await getAccessTokenSilently({
            authorizationParams: {
              audience: AUTH0_AUDIENCE || undefined,
              scope: 'openid profile email',
            },
          });
          lastTokenError = null;
          return token;
        } catch (err) {
          const error = err instanceof Error ? err : new Error(String(err));
          console.warn('[useGetAccessToken] token error', error);
          lastTokenError = error;
          return '';
        }
      }, [getAccessTokenSilently]);
    }
  : function useGetAccessTokenDev(): GetTokenFn {
      // eslint-disable-next-line react-hooks/rules-of-hooks
      return useCallback(async (): Promise<string> => '', []);
    };

/**
 * Hook that returns an authenticated fetch function.
 * Attaches Authorization: Bearer <token> to every request.
 */
export function useAuthenticatedFetch() {
  const getToken = useGetAccessToken();

  const authFetch = useCallback(
    async (input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> => {
      const token = await getToken();
      const headers = new Headers(init.headers);
      if (token) headers.set('Authorization', `Bearer ${token}`);
      headers.set('Content-Type', 'application/json');
      return fetch(input, { ...init, headers });
    },
    [getToken],
  );

  return { authFetch, getToken };
}
