# Single sign-on (OIDC)

RDataCore can trust an external OpenID Connect provider — Keycloak, Auth0,
Entra ID, or anything else that speaks the specification — so that people sign
in with the account they already have.

Single sign-on is a third way to answer *who is this*. It is never a second
answer to *what may they do*: an OIDC sign-in populates the same claims a
password login does, and every existing permission check applies unchanged.

Two things are enabled separately, and you may want only the first:

- **Bearer tokens.** Set `RDC_OIDC_ISSUER` and `RDC_OIDC_AUDIENCE`, and any
  request carrying a valid token from that provider authenticates. This is
  what a machine caller such as the MCP server uses.
- **Browser sign-in.** Additionally set `RDC_OIDC_CLIENT_ID` and
  `RDC_OIDC_REDIRECT_URI`, and a "Sign in with…" button appears on the login
  page.

---

## Configuration

| Variable | Required | Default | What it does |
|---|---|---|---|
| `RDC_OIDC_ISSUER` | — | unset | The provider's issuer URL. **Setting this is what turns single sign-on on.** Leaving it unset disables the feature entirely. |
| `RDC_OIDC_AUDIENCE` | when the issuer is set | — | The audience RDataCore's tokens are minted for. Startup fails without it; see [Why the audience is required](#why-the-audience-is-required). |
| `RDC_OIDC_ROLE_MAP` | no | empty | `idp-group:rdc-role,other-group:other-role`. A malformed entry fails startup rather than being skipped. |
| `RDC_OIDC_ROLES_CLAIM` | no | `groups` | Which claim carries group membership. |
| `RDC_OIDC_DEFAULT_ROLE` | no | unset | Role for someone matching no mapping. **Unset means reject them**; see [Why unmapped users are rejected](#why-unmapped-users-are-rejected). |
| `RDC_OIDC_LINK_BY_EMAIL` | no | `false` | Whether a first sign-in may adopt an existing local account with the same address. See [Why email linking is off](#why-email-linking-is-off-by-default). |
| `RDC_OIDC_JWKS_TTL_SECS` | no | `3600` | How long a fetched signing-key set stays usable. |
| `RDC_OIDC_RESOLUTION_CACHE_SECS` | no | `60` | How long a resolved identity is reused before being looked up again. This is also how long a revocation takes to bite; see [The revocation window](#the-revocation-window). |
| `RDC_OIDC_CLIENT_ID` | for browser sign-in | unset | This application's client id at the provider. Setting it enables the browser flow. |
| `RDC_OIDC_REDIRECT_URI` | with `CLIENT_ID` | — | Where the provider sends the browser back. Startup fails if a client id is set without one. |
| `RDC_OIDC_CLIENT_SECRET` | no | unset | Omit for a public client; PKCE is mandatory either way. Never printed, including in debug output. |
| `RDC_OIDC_POST_LOGIN_PATH` | no | `/dashboard` | Route in the admin interface to land on after signing in. Resolved against `FRONTEND_BASE_URL` when the interface is on a different origin from the API. |

The redirect URI is always `<your base URL>/admin/api/v1/auth/oidc/callback`.
Register exactly that at the provider.

The sign-in button is labelled generically rather than with your provider's
name. `/admin/api/v1/system/capabilities` is public and unauthenticated, and
it reports only booleans — naming the provider there would tell any anonymous
caller which identity provider your organisation uses, which is more than a
feature flag and more than a button label is worth.

---

## Trying it locally

A working Keycloak, realm and users ship with the repo. Nothing to configure:

```bash
docker compose -f compose.yaml -f compose.sso.yaml up -d
```

That is the whole setup. Two one-shot services run as part of it: `sso-migrate`
applies migrations, and `sso-seed` creates the roles the realm's groups map
onto. Both exit when done and are safe to re-run.

They are not optional conveniences. The app does not migrate at boot by design,
and nothing else seeds roles — a role map naming a role that does not exist has
no effect, so every identity would map to nothing and be refused. That looks
like a broken guard rather than a missing fixture, which is a bad hour to
spend.

Then open `http://rdatacore.docker/admin` and use the SSO button.

| Who | Password | Group | Maps to | What should happen |
|---|---|---|---|---|
| `ada` | `ada` | `rdc-admins` | `sso-admin` | Signs in, broad permissions |
| `grace` | `grace` | `rdc-editors` | `sso-editor` | Signs in, read plus workflow edit |
| `mallory` | `mallory` | `rdc-nobodies` | *nothing* | **Refused** — this is default-deny working |

Keycloak's own console is at `http://auth.rdatacore.docker:8081` (`admin` /
`admin`). Neither those credentials nor anything in the realm is used by
RDataCore beyond the realm itself.

### Things worth trying

- **Sign in as `mallory`.** The refusal is the feature. Nothing is created for
  them — check `SELECT count(*) FROM admin_users WHERE username = 'mallory'`.
- **Sign in as `ada`, then deactivate her in the admin UI** and reload. Access
  ends within `RDC_OIDC_RESOLUTION_CACHE_SECS`, set to 10 in this stack so you
  do not have to wait a minute. The identity provider still considers her
  perfectly valid; RDataCore does not, and RDataCore wins.
- **Try to set a password on her account** through the user form. Refused: an
  SSO-provisioned account has no password path.
- **Try `mallory` again after adding her to `rdc-editors`** in the Keycloak
  console. She is admitted on the next sign-in, with no change to RDataCore.

### Why port 8081 rather than the usual `.docker` name on port 80

The issuer URL has to be byte-identical in two places that reach Keycloak by
different routes: your browser, via the host, and the `app` container, which
fetches the signing keys over the compose network. A token whose `iss` does not
match is rejected, and a key-set URL the server cannot reach fails every
sign-in.

Publishing 8081 on the host *and* listening on 8081 inside the container makes
`http://auth.rdatacore.docker:8081` correct from both sides. Port 80 would need
root inside the Keycloak image, which it deliberately does not have.

This is the single most common thing to get wrong when wiring a provider, local
or not: **whatever `iss` your provider puts in its tokens must be exactly what
`RDC_OIDC_ISSUER` says, and that URL must be reachable from the server.**

---

## Worked setups

Each of these grants the RDataCore role `editor` to members of one provider
group. Create that role in RDataCore first — a mapping naming a role that does
not exist is reported in the log and has no effect, which leaves everyone
rejected.

### Keycloak

1. In your realm, create a client: **Client ID** `r-data-core`, **Client
   authentication** on (confidential) or off (public — PKCE covers it).
2. Set **Valid redirect URIs** to
   `https://rdc.example.com/admin/api/v1/auth/oidc/callback`.
3. Add a **Group Membership** mapper on the client: **Token Claim Name**
   `groups`, **Full group path** off, and **Add to ID token** on.
4. Create a group, e.g. `rdc-editors`, and put a user in it.

```bash
RDC_OIDC_ISSUER=https://keycloak.example.com/realms/main
RDC_OIDC_AUDIENCE=r-data-core
RDC_OIDC_CLIENT_ID=r-data-core
RDC_OIDC_CLIENT_SECRET=<from the Credentials tab, if confidential>
RDC_OIDC_REDIRECT_URI=https://rdc.example.com/admin/api/v1/auth/oidc/callback
RDC_OIDC_ROLE_MAP=rdc-editors:editor
```

Keycloak does not put the client id in `aud` for ID tokens by default. If
sign-in fails with an audience error, add a hardcoded **Audience** mapper
naming `r-data-core`.

### Auth0

1. Create a **Regular Web Application**.
2. Set **Allowed Callback URLs** to
   `https://rdc.example.com/admin/api/v1/auth/oidc/callback`.
3. Auth0 does not emit a `groups` claim by default. Add an Action on the
   **Login** flow:

```js
exports.onExecutePostLogin = async (event, api) => {
    const groups = event.authorization?.roles ?? []
    api.idToken.setCustomClaim('https://rdatacore/groups', groups)
}
```

```bash
RDC_OIDC_ISSUER=https://your-tenant.eu.auth0.com/
RDC_OIDC_AUDIENCE=<the Client ID>
RDC_OIDC_CLIENT_ID=<the Client ID>
RDC_OIDC_CLIENT_SECRET=<the Client Secret>
RDC_OIDC_REDIRECT_URI=https://rdc.example.com/admin/api/v1/auth/oidc/callback
RDC_OIDC_ROLES_CLAIM=https://rdatacore/groups
RDC_OIDC_ROLE_MAP=rdc-editors:editor
```

### Microsoft Entra ID

1. Register an application. Under **Authentication**, add a **Web** redirect
   URI of `https://rdc.example.com/admin/api/v1/auth/oidc/callback`.
2. Under **Token configuration**, add a **groups** claim. Entra emits group
   **object ids**, not names, unless the groups are synced from on-premises AD
   — so the mapping keys are usually GUIDs.

```bash
RDC_OIDC_ISSUER=https://login.microsoftonline.com/<tenant-id>/v2.0
RDC_OIDC_AUDIENCE=<Application (client) ID>
RDC_OIDC_CLIENT_ID=<Application (client) ID>
RDC_OIDC_CLIENT_SECRET=<a client secret>
RDC_OIDC_REDIRECT_URI=https://rdc.example.com/admin/api/v1/auth/oidc/callback
RDC_OIDC_ROLE_MAP=8f4c1b2e-...:editor
```

A user in many groups causes Entra to send a `_claim_names` overage reference
instead of the groups themselves. RDataCore does not follow that reference, so
those users map to nothing and are rejected. Use an app-role claim, or narrow
which groups are emitted.

---

## How an account comes into being

The first time someone signs in, RDataCore resolves their roles **before**
creating anything. An identity that maps to no role is refused with no account
left behind, so a rejected sign-in leaves no trace to clean up.

If the identity does map to a role, an account is created and flagged as
SSO-provisioned. That flag closes every password path for it — see
[SSO accounts have no password](#sso-accounts-have-no-password).

Identity is keyed on `(issuer, subject)`, never on email address. Email is
mutable and re-assignable at most providers; a subject is not.

Roles are **not stored**. They are re-derived from the token's claims on each
sign-in, which is why the admin interface shows them read-only for such an
account: an editable control would silently discard the change. Change the
group membership at the provider instead.

---

## Security notes

Each of these defaults is deliberately inconvenient. Knowing why makes it
much less tempting to switch one off.

### Why unmapped users are rejected

Someone whose groups map to nothing is **refused**, not admitted with no
permissions.

Admitting them looks harmless — an account that can do nothing — but it is a
real account with a real session that merely happens to be powerless *today*.
The moment anyone configures a broad default role, or a mapping widens, every
one of those dormant accounts becomes live. Refusing keeps the set of people
who can sign in equal to the set someone deliberately granted access.

Set `RDC_OIDC_DEFAULT_ROLE` if you genuinely want everyone at your provider to
have a baseline role. Make it a narrow one.

### Why super_admin cannot be granted by claim mapping

A mapping onto a role with `super_admin` set has the role filtered out, and
the drop is logged.

A group name is a string controlled by whoever administers the identity
provider, who is frequently not the person who administers RDataCore. Allowing
that string to confer super-admin would hand over the entire instance to a
string match in someone else's directory. Grant super-admin inside RDataCore,
to a named person, deliberately.

If claim mapping produces *only* super-admin roles, the result is empty and
the sign-in is refused rather than admitted with nothing.

### Why email linking is off by default

`RDC_OIDC_LINK_BY_EMAIL` lets a first sign-in attach to an existing local
account with the same address. It is off, and even when on it additionally
requires the provider to have set `email_verified`.

Both conditions are necessary. Without the second, anyone able to set an
unverified email address at a trusted issuer could claim someone else's
account — including an administrator's. Turn it on only when you trust your
provider's address verification, and only while migrating.

### Why the audience is required

`RDC_OIDC_ISSUER` without `RDC_OIDC_AUDIENCE` fails startup.

Without an audience check, *any* token from that issuer is accepted, including
one minted for a completely different service that happens to share the
provider. Someone who can obtain a token for an unrelated application would be
able to use it here.

### SSO accounts have no password

An SSO-provisioned account is refused by every password path: login,
forgot-password, reset-password, and setting a password through the admin user
form.

The reason is forgot-password specifically. Without the guard, it becomes a
way to *set* a password on a federated account — turning single sign-on into a
local-account factory that bypasses the provider entirely. The refusal on
forgot-password is silent, exactly as it is for an unknown address, because
answering differently would tell a stranger which accounts are federated.

To convert someone to a local account, remove the identity link first.

### The revocation window

A resolved identity is cached for `RDC_OIDC_RESOLUTION_CACHE_SECS` (60 by
default). Deactivating an account in RDataCore therefore takes effect within
that window rather than instantly.

The token itself is verified on **every** request — only the database lookup
is cached — so an expired or revoked token stops working immediately. What the
window covers is a change made on the RDataCore side. Shorten it if you want
faster revocation and can afford a database round trip per request.

Note that an account deactivated in RDataCore is refused even while the
provider keeps issuing perfectly valid tokens. The provider says who someone
is; RDataCore says whether they may act.

### What the login flow protects against

- **PKCE is mandatory**, not negotiated. An intercepted authorization code is
  useless without the verifier, which never leaves the server.
- **`state` is single-use**, claimed atomically, so an intercepted callback
  cannot be replayed.
- **`state` is bound to the browser** that started the sign-in, via an
  HttpOnly cookie. Without that binding an attacker could start a sign-in,
  take the resulting `state`, and induce a victim's browser to complete it —
  leaving the victim signed in to the attacker's account.
- **Redirect targets must be paths inside this application.** An open redirect
  on a login endpoint is a phishing primitive, and here the response carries
  session tokens in its fragment, so it would hand those tokens away too.

---

## The MCP server

RDataCore's MCP server is a second resource server against this same issuer,
so an AI assistant authenticates its user the same way the admin interface
does. It uses **the same audience**: an MCP caller is an RDataCore
administrator, not a separate kind of principal, and the exchange endpoint
validates presented tokens against this instance's own `RDC_OIDC_AUDIENCE`.

It never forwards a caller's token to RDataCore. It presents it to
`POST /admin/api/v1/auth/oidc/exchange`, which answers with a short-lived
local token for that same person — so everything the assistant does is
attributed to the human who asked for it, and bounded by what they may do.

Deployment guide: [MCP.md](./MCP.md).

---

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| Startup fails: audience required | `RDC_OIDC_ISSUER` set without `RDC_OIDC_AUDIENCE`. |
| Startup fails: redirect URI required | `RDC_OIDC_CLIENT_ID` set without `RDC_OIDC_REDIRECT_URI`. |
| No button on the login page | Only the bearer half is configured. Set `RDC_OIDC_CLIENT_ID` and `RDC_OIDC_REDIRECT_URI`. |
| Everyone is rejected | The role map names a role that does not exist — check the log for a warning naming it — or no group matches and no default role is set. |
| `sso_error=unbound_state` | The sign-in was started in a different browser, or the cookie was dropped. Start again from the login page. Persistent failures behind a proxy usually mean the cookie is being stripped. |
| `sso_error=expired_or_replayed` | More than five minutes passed, or the callback was opened twice. |
| 401 with a token that works elsewhere | Wrong audience, or the token is an access token rather than an ID token at a provider that issues opaque access tokens. |
| 503 on a sign-in | The provider is unreachable, or the database is. Distinct from 403 on purpose — this is infrastructure, not permissions. |

Raise the log level to `debug` for the authentication path; refusals are
logged in full on the server while the client is told deliberately little.
