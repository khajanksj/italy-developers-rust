# Italy Developers — Rust / Actix Web

Production-oriented, server-rendered company website built with Rust, Actix Web, Askama and PostgreSQL. JavaScript enhances navigation without replacing crawlable server routes, so links, browser history and SEO work without client-side rendering.

## Security design

- Memory-safe Rust application with strict CSP and no inline JavaScript or CSS.
- HSTS, `nosniff`, clickjacking denial, restrictive Permissions Policy, COOP and CORP.
- Signed, HTTP-only, `SameSite=Strict`, `__Host-` session cookies.
- Session-bound CSRF tokens compared in constant time.
- Global IP rate limiting, 32 KiB form/JSON limits and bounded field validation.
- Honeypot spam field, normalized email input and parameterized SQLx queries.
- Generic public errors, request IDs and structured logs without submitted form values.
- PostgreSQL role hardening in the migration, non-root/read-only container and dropped Linux capabilities.
- Readiness/liveness endpoints and dependency audit in CI.

No web application can promise immunity from every attacker. Production still requires TLS at the proxy, secret rotation, least-privilege database credentials, backups, monitoring, WAF/DDoS controls, dependency updates and regular security review.

## Run locally

```bash
cp .env.example .env
# Replace APP_SECRET_KEY; for local HTTP only keep COOKIE_SECURE=false.
docker compose up --build
```

Open `http://localhost:8080`. SQLx applies migrations during startup.

## Production variables

`DATABASE_URL`, `APP_SECRET_KEY` (at least 64 random bytes), `COOKIE_SECURE=true`, `HOST`, `PORT`, `WEB_WORKERS`, `DB_MAX_CONNECTIONS` and `RUST_LOG`.

## Deployment

Deploy the Docker image to Fly.io, Render, Railway, AWS, Google Cloud Run or another container host with a managed PostgreSQL database. Vercel's normal Next.js deployment is not an Actix Web server runtime; use a container platform for this repository.
