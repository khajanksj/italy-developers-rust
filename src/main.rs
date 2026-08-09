mod config;
mod error;
mod handlers;
mod security;

use std::io;
use actix_files::Files;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_session::{config::PersistentSession, storage::CookieSessionStore, SessionMiddleware};
use actix_web::{cookie::{Key, SameSite}, middleware::{Compress, DefaultHeaders, NormalizePath}, web, App, HttpServer};
use mongodb::Client;
use tracing_actix_web::TracingLogger;

use config::Config;

#[actix_web::main]
async fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().json().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let config = Config::from_env().map_err(io::Error::other)?;
    let client = Client::with_uri_str(&config.mongodb_url).await.map_err(io::Error::other)?;
    let database = client.database(&config.mongodb_database);
    database.run_command(mongodb::bson::doc!{"ping":1}).await.map_err(io::Error::other)?;
    let args=std::env::args().collect::<Vec<_>>();
    if args.len()>1 { handlers::user_command(&database,&args[1..]).await.map_err(io::Error::other)?; return Ok(()) }
    std::fs::create_dir_all(&config.upload_dir)?;
    let limiter = GovernorConfigBuilder::default().seconds_per_request(1).burst_size(100).finish().expect("valid governor config");
    let bind = (config.host.clone(), config.port);
    let workers = config.workers;
    let cookie_name = if config.cookie_secure { "__Host-session" } else { "session" }.to_string();
    tracing::info!(host=%bind.0, port=bind.1, "starting server");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(database.clone()))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::JsonConfig::default().limit(32 * 1024).error_handler(error::json_error))
            .app_data(web::FormConfig::default().limit(32 * 1024).error_handler(error::form_error))
            .wrap(TracingLogger::default())
            .wrap(security::RequestId)
            .wrap(Governor::new(&limiter))
            .wrap(Compress::default())
            .wrap(NormalizePath::trim())
            .wrap(SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&config.cookie_key)).cookie_name(cookie_name.clone()).cookie_secure(config.cookie_secure).cookie_http_only(true).cookie_same_site(SameSite::Strict).cookie_path("/".into()).session_lifecycle(PersistentSession::default().session_ttl(actix_web::cookie::time::Duration::hours(2))).build())
            .wrap(DefaultHeaders::new()
                .add(("Content-Security-Policy", security::CSP))
                .add(("Strict-Transport-Security", "max-age=63072000; includeSubDomains; preload"))
                .add(("X-Content-Type-Options", "nosniff"))
                .add(("X-Frame-Options", "DENY"))
                .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=(), payment=(), usb=()"))
                .add(("Cross-Origin-Opener-Policy", "same-origin"))
                .add(("Cross-Origin-Resource-Policy", "same-origin"))
                .add(("X-Permitted-Cross-Domain-Policies", "none")))
            .service(Files::new("/static", "static").prefer_utf8(true).use_etag(true).use_last_modified(true))
            .service(Files::new("/uploads", "uploads").prefer_utf8(true).use_etag(true).use_last_modified(true))
            .configure(handlers::routes)
    }).workers(workers).bind(bind)?.run().await
}
