# Italy Developers — Rust / Actix Web / MongoDB

Production-oriented, server-rendered company website built with Rust, Actix Web, Askama and MongoDB. JavaScript enhances navigation without replacing crawlable server routes, so links, browser history and SEO work without client-side rendering.

Site content — services, case studies, testimonials, team, FAQs and insight articles — lives in MongoDB and is managed through a built-in admin panel, not hardcoded in templates.

## Security design

- Memory-safe Rust application with strict CSP and no inline JavaScript or CSS.
- HSTS, `nosniff`, clickjacking denial, restrictive Permissions Policy, COOP and CORP.
- Signed, HTTP-only, `SameSite=Strict`, `__Host-` session cookies.
- Session-bound CSRF tokens compared in constant time, on every public and admin form.
- Global IP rate limiting, 32 KiB form/JSON limits and bounded field validation.
- Honeypot spam field, normalized email input and parameterized MongoDB queries.
- Admin passwords hashed with Argon2; login runs the verify step against a dummy hash for non-existent usernames to reduce timing side-channels.
- Every admin route re-checks the session against the database on each request.
- Generic public errors, request IDs and structured logs without submitted form values.
- Non-root/read-only container and dropped Linux capabilities.
- Readiness/liveness endpoints and dependency audit in CI.

No web application can promise immunity from every attacker. Production still requires TLS at the proxy, secret rotation, least-privilege database credentials, backups, monitoring, WAF/DDoS controls, dependency updates and regular security review.

## Admin panel

Visit `/admin/login`. The first admin account is created automatically on startup from `ADMIN_USERNAME`/`ADMIN_PASSWORD` if the `admins` collection is empty; afterwards, manage further admin accounts from `/admin/admins`. From the panel you can:

- Review, filter, update the status of, and delete contact leads (`/admin/leads`).
- Edit services, work/case studies, testimonials, team members, FAQs and insight articles (`/admin/content/<type>`).
- Edit site-wide settings — contact details and headline stats (`/admin/settings`).

## Run locally

```bash
cp .env.example .env
# Replace APP_SECRET_KEY and ADMIN_PASSWORD; for local HTTP only keep COOKIE_SECURE=false.
docker compose up --build
```

Open `http://localhost:8080`. Indexes are created and starter content is seeded into MongoDB on first boot.

## Production variables

`MONGODB_URI`, `MONGODB_DB`, `APP_SECRET_KEY` (at least 64 random bytes), `ADMIN_USERNAME`/`ADMIN_PASSWORD` (bootstrap only — remove after the first admin exists), `COOKIE_SECURE=true`, `HOST`, `PORT`, `WEB_WORKERS` and `RUST_LOG`.

## Deployment

Deploy the Docker image to Fly.io, Render, Railway, AWS, Google Cloud Run or another container host with a managed MongoDB database (e.g. MongoDB Atlas). Vercel's normal Next.js deployment is not an Actix Web server runtime; use a container platform for this repository.
