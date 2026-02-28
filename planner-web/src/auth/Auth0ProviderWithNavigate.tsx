import { Auth0Provider } from '@auth0/auth0-react';
import type { AppState } from '@auth0/auth0-react';
import { useNavigate } from 'react-router-dom';
import type { ReactNode } from 'react';
import { AUTH0_DOMAIN, AUTH0_CLIENT_ID, AUTH0_AUDIENCE } from '../config.ts';

interface Props {
  children: ReactNode;
}

export default function Auth0ProviderWithNavigate({ children }: Props) {
  const navigate = useNavigate();

  const onRedirectCallback = (appState: AppState | undefined): void => {
    navigate(appState?.returnTo ?? window.location.pathname);
  };

  return (
    <Auth0Provider
      domain={AUTH0_DOMAIN}
      clientId={AUTH0_CLIENT_ID}
      authorizationParams={{
        redirect_uri: window.location.origin + '/callback',
        audience: AUTH0_AUDIENCE || undefined,
        scope: 'openid profile email',
      }}
      onRedirectCallback={onRedirectCallback}
    >
      {children}
    </Auth0Provider>
  );
}
