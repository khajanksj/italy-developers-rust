# Italy Developers — MongoDB CMS

Server-rendered Rust/Actix website and content-management system for Italy Developers. It targets Italian small businesses and independent professionals seeking practical, affordable websites, e-commerce and automation.

## Included

- MongoDB-backed services, work, tech stack, about sections, insights, blog posts and contact leads.
- Role-aware admin dashboard at `/admin` with live validation, publishing controls, ordering, rich visual/HTML editing, SEO fields, image uploads and lead management.
- Expanded service, portfolio, insight, blog, tech stack, about and contact experiences.
- Per-entry title, description, keywords, Open Graph data and Schema.org structured data.
- Dynamic XML sitemap, robots policy, semantic server rendering and crawlable detail routes.
- Signed HTTP-only sessions, CSRF-protected public contact form, rate limiting, security headers and bounded uploads.
- Non-root, read-only application container with persistent MongoDB and upload volumes.

## Run with Docker

```bash
docker compose up --build -d
```

Open:

- Website: `http://localhost:8080`
- Admin: `http://localhost:8080/admin/login`

Create the first account from the project directory (passwords must be at least 12 characters):

```powershell
docker compose run --rm web create-superuser owner@example.com "use-a-strong-password"
docker compose run --rm web create-admin manager@example.com "use-a-strong-password"
docker compose run --rm web create-staff editor@example.com "use-a-strong-password"
```

- `superuser` and `admin` can publish, delete content and manage enquiries.
- `staff` can create and edit content, but cannot publish, delete or access enquiries.

Accounts sign in with email and password. Passwords are stored as bcrypt hashes. Change `APP_SECRET_KEY` before any shared or production deployment; Compose credentials are for local development only.

Useful commands:

```bash
docker compose logs -f web
docker compose ps
docker compose down
```

Use `docker compose down -v` only when you intentionally want to erase MongoDB content and uploaded images.

## Production settings

Configure `MONGODB_URL`, `MONGODB_DATABASE`, `APP_SECRET_KEY`, `PUBLIC_URL`, `COOKIE_SECURE=true`, `HOST`, `PORT`, `WEB_WORKERS`, `UPLOAD_DIR` and `RUST_LOG`.

Use TLS, a strong database user/password, encrypted backups, restricted network access, object storage or a persistent upload volume, secret rotation, monitoring and regular dependency updates.

## Production deployment

The production override enables authenticated MongoDB, secure cookies, readiness checks, automatic restarts, bounded container logs and loopback-only web binding for a TLS reverse proxy.

1. Copy `.env.production.example` to `.env.production` and replace every `CHANGE_ME` value. Keep this file off Git. Use URL-safe characters for the MongoDB password because it is embedded in the connection URL.
2. Point `PUBLIC_URL` at the final HTTPS domain. Configure Caddy, Nginx or your hosting provider to terminate TLS and proxy to `127.0.0.1:8080`.
3. Build and start the stack:

```bash
docker compose --env-file .env.production -f docker-compose.yml -f docker-compose.prod.yml up --build -d
docker compose --env-file .env.production -f docker-compose.yml -f docker-compose.prod.yml ps
```

4. Create the first administrator:

```bash
docker compose --env-file .env.production -f docker-compose.yml -f docker-compose.prod.yml run --rm web create-superuser owner@example.com "use-a-unique-12+-character-password"
```

5. Verify `https://your-domain/health/live`, `https://your-domain/health/ready`, the public pages and `/admin/login`.

Back up both named volumes: `mongo_data` contains content, users and enquiries; `uploads` contains admin-uploaded images. Test restoration before relying on a backup policy. For updates, pull the reviewed code and repeat the `up --build -d` command. Do not run `docker compose down -v` in production.
