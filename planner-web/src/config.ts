export const AUTH0_DOMAIN = import.meta.env.VITE_AUTH0_DOMAIN || '';
export const AUTH0_CLIENT_ID = import.meta.env.VITE_AUTH0_CLIENT_ID || '';
export const AUTH0_AUDIENCE = import.meta.env.VITE_AUTH0_AUDIENCE || '';
export const API_BASE = import.meta.env.VITE_API_BASE || '/api';

export const AUTH0_ENABLED =
  AUTH0_DOMAIN !== '' && AUTH0_CLIENT_ID !== '';
