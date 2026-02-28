# Auth0 Setup Guide for Planner v2

## What is Auth0?

Auth0 is a managed authentication service. It handles user login, signup, password reset, social logins (Google, GitHub, etc.), and JWT token issuance so you don't have to build any of that yourself.

**Auth0 is free** for up to 25,000 monthly active users — more than enough for development and early production. No credit card required to sign up.

### Pricing Summary

| Plan | Cost | Users | Notes |
|---|---|---|---|
| **Free** | $0/month | Up to 25,000 MAU | All you need for dev + early production |
| Essentials | $35/month | 500+ MAU | Custom domain, MFA, RBAC |
| Professional | $240/month | 1,000+ MAU | Advanced security, enterprise MFA |
| Enterprise | Custom | Unlimited | SLA, premium support |

You will almost certainly stay on the Free tier for a long time. It includes passwordless auth, social connections (Google, GitHub, etc.), and custom domains.

---

## Step 1: Create an Auth0 Account

1. Go to [https://auth0.com/signup](https://auth0.com/signup)
2. Sign up with your email or GitHub account
3. Choose a **Tenant Name** — this becomes your Auth0 domain (e.g., `planner-v2.us.auth0.com`)
4. Select your region (US, EU, or AU)

Once signed up, you'll land on the Auth0 Dashboard.

---

## Step 2: Create a Single Page Application

This is the Auth0 "application" that represents your React frontend.

1. In the Auth0 Dashboard, click **Applications** → **Applications** in the left sidebar
2. Click **+ Create Application**
3. Name it: `Planner v2 Web`
4. Select **Single Page Web Applications**
5. Click **Create**

### Configure the Application

On the application's **Settings** tab, scroll down to **Application URIs** and set:

**For local development:**

| Field | Value |
|---|---|
| Allowed Callback URLs | `http://localhost:5173/callback, http://localhost:3100/callback` |
| Allowed Logout URLs | `http://localhost:5173, http://localhost:3100` |
| Allowed Web Origins | `http://localhost:5173, http://localhost:3100` |

> **Note:** `localhost:5173` is the Vite dev server, `localhost:3100` is the Axum server serving the built React app.

**For production**, add your production domain to each of these fields (comma-separated with the localhost values).

Scroll down and click **Save Changes**.

### Copy Your Credentials

From the **Settings** tab, note these two values — you'll need them in Step 4:

- **Domain** — e.g., `planner-v2.us.auth0.com`
- **Client ID** — e.g., `aBcDeFgHiJkLmNoPqRsTuVwXyZ123456`

---

## Step 3: Create an API

This tells Auth0 about your backend server so it issues tokens with the right audience.

1. In the Auth0 Dashboard, click **Applications** → **APIs** in the left sidebar
2. Click **+ Create API**
3. Fill in:
   - **Name:** `Planner v2 API`
   - **Identifier:** `https://planner-api` (this is the "audience" — it can be any URI, it doesn't need to be a real URL)
   - **Signing Algorithm:** RS256 (leave as default)
4. Click **Create**

Note the **Identifier** value — this is your `AUTH0_AUDIENCE`.

---

## Step 4: Configure Planner v2

### Frontend (planner-web)

Create a `.env` file in the `planner-web/` directory:

```bash
cd planner-web
cp .env.example .env
```

Edit `.env` with your Auth0 values:

```env
VITE_AUTH0_DOMAIN=planner-v2.us.auth0.com
VITE_AUTH0_CLIENT_ID=aBcDeFgHiJkLmNoPqRsTuVwXyZ123456
VITE_AUTH0_AUDIENCE=https://planner-api
```

Then rebuild the frontend:

```bash
npm run build
```

### Backend (planner-server)

Set environment variables before starting the server:

```bash
export AUTH0_DOMAIN=planner-v2.us.auth0.com
export AUTH0_AUDIENCE=https://planner-api
```

Or create a `.env` file / shell script you source before running.

Then start the server:

```bash
cargo run --bin planner-server
```

---

## Step 5: Verify It Works

1. Start the server: `cargo run --bin planner-server`
2. Open `http://localhost:3100` in your browser
3. You should see the Planner v2 login page
4. Click **Sign In** — you'll be redirected to Auth0's Universal Login
5. Create an account or log in
6. You'll be redirected back to the Planner dashboard
7. Create a new session and send a message

---

## Dev Mode (No Auth0 Required)

If you don't set any Auth0 environment variables, both the frontend and backend run in **dev mode**:

- **Frontend:** Skips Auth0, login page goes directly to session creation
- **Backend:** Injects a synthetic `dev|local` user — all endpoints work without tokens

This means you can develop and test locally without touching Auth0 at all. Just run:

```bash
# No env vars needed
cargo run --bin planner-server
```

---

## Optional: Enable Social Logins

By default, Auth0 provides email/password login. To add Google, GitHub, etc.:

1. In Auth0 Dashboard, go to **Authentication** → **Social**
2. Click **+ Create Connection**
3. Choose a provider (e.g., Google, GitHub)
4. Follow the setup wizard (you'll need OAuth credentials from the provider)
5. Enable the connection for your `Planner v2 Web` application

Free tier includes unlimited social connections.

---

## Optional: Customize the Login Page

Auth0 provides a hosted "Universal Login" page. To customize its look:

1. In Auth0 Dashboard, go to **Branding** → **Universal Login**
2. Choose a theme and customize colors/logo to match the Planner dark theme
3. Click **Save**

---

## Environment Variable Reference

| Variable | Where | Required | Description |
|---|---|---|---|
| `VITE_AUTH0_DOMAIN` | planner-web `.env` | For auth | Your Auth0 tenant domain |
| `VITE_AUTH0_CLIENT_ID` | planner-web `.env` | For auth | Application client ID from Step 2 |
| `VITE_AUTH0_AUDIENCE` | planner-web `.env` | For auth | API identifier from Step 3 |
| `AUTH0_DOMAIN` | planner-server env | For auth | Same domain as frontend |
| `AUTH0_AUDIENCE` | planner-server env | For auth | Same audience as frontend |
| `AUTH0_SECRET` | planner-server env | No | HS256 signing secret (dev/testing only) |

**None of these are required for local development.** Dev mode works without any Auth0 configuration.

---

## Troubleshooting

### "Callback URL mismatch" error
Your browser URL doesn't match the **Allowed Callback URLs** in Auth0. Add the exact URL (including port) to the Auth0 application settings.

### "Unauthorized" on API calls
- Check that `AUTH0_DOMAIN` and `AUTH0_AUDIENCE` match between frontend and backend
- Verify the API identifier in Auth0 Dashboard matches your `AUTH0_AUDIENCE`

### "Login works but I get 401 on the server"
The server validates the JWT's issuer and audience. Make sure:
- `AUTH0_DOMAIN` is set on the server (without `https://` prefix)
- `AUTH0_AUDIENCE` matches the API identifier in Auth0

### Token expired
Auth0 tokens expire after a configurable period (default 24 hours). The React SDK handles token refresh automatically via `getAccessTokenSilently()`.

---

## Security Notes

- **Never commit `.env` files** — they contain secrets. The `.gitignore` already excludes them.
- **RS256 is the default** — Auth0 signs tokens with RS256 (asymmetric). In production, the server should fetch the JWKS (JSON Web Key Set) from `https://<domain>/.well-known/jwks.json` to validate tokens. The current implementation supports this path but also has a dev fallback.
- **CORS is tightened** when auth is enabled — only `localhost:5173` and `localhost:3100` are allowed origins. Update this when deploying to production.
