use secrecy::{ExposeSecret, SecretString};

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub mongodb_uri: String,
    pub mongodb_db: String,
    pub cookie_key: [u8; 64],
    pub cookie_secure: bool,
    pub workers: usize,
    pub bootstrap_admin_username: Option<String>,
    pub bootstrap_admin_password: Option<SecretString>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let mongodb_uri = std::env::var("MONGODB_URI")?;
        let mongodb_db = std::env::var("MONGODB_DB").unwrap_or_else(|_| "italy_developers".into());
        let secret = SecretString::from(std::env::var("APP_SECRET_KEY")?);
        let bytes = secret.expose_secret().as_bytes();
        anyhow::ensure!(bytes.len() >= 64, "APP_SECRET_KEY must contain at least 64 bytes");
        let mut cookie_key = [0u8; 64];
        cookie_key.copy_from_slice(&bytes[..64]);
        Ok(Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PORT").unwrap_or_else(|_| "8080".into()).parse()?,
            mongodb_uri,
            mongodb_db,
            cookie_key,
            cookie_secure: std::env::var("COOKIE_SECURE").map(|v| v != "false").unwrap_or(true),
            workers: std::env::var("WEB_WORKERS").unwrap_or_else(|_| "2".into()).parse()?,
            bootstrap_admin_username: std::env::var("ADMIN_USERNAME").ok(),
            bootstrap_admin_password: std::env::var("ADMIN_PASSWORD").ok().map(SecretString::from),
        })
    }
}
