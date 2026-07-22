use secrecy::{ExposeSecret, SecretString};

#[derive(Clone)]
pub struct Config { pub host:String, pub port:u16, pub mongodb_url:String, pub mongodb_database:String, pub cookie_key:[u8;64], pub cookie_secure:bool, pub workers:usize, pub public_url:String, pub upload_dir:String }

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let mongodb_url = std::env::var("MONGODB_URL")?;
        let secret = SecretString::from(std::env::var("APP_SECRET_KEY")?);
        let bytes = secret.expose_secret().as_bytes();
        anyhow::ensure!(bytes.len() >= 64, "APP_SECRET_KEY must contain at least 64 bytes");
        anyhow::ensure!(
            !secret.expose_secret().contains("replace-with")
                && !secret.expose_secret().contains("local-development-key")
                && !secret.expose_secret().contains("CHANGE_ME"),
            "APP_SECRET_KEY is still a placeholder; generate a unique production secret"
        );
        let mut cookie_key=[0u8;64]; cookie_key.copy_from_slice(&bytes[..64]);
        let cookie_secure = std::env::var("COOKIE_SECURE").map(|v|v!="false").unwrap_or(true);
        let public_url = std::env::var("PUBLIC_URL").unwrap_or_else(|_|"http://localhost:8080".into());
        let parsed_url = url::Url::parse(&public_url)?;
        anyhow::ensure!(matches!(parsed_url.scheme(), "http" | "https"), "PUBLIC_URL must use http or https");
        if cookie_secure {
            anyhow::ensure!(parsed_url.scheme() == "https", "PUBLIC_URL must use https when COOKIE_SECURE=true");
        }
        let workers:usize = std::env::var("WEB_WORKERS").unwrap_or_else(|_|"2".into()).parse()?;
        anyhow::ensure!(workers > 0, "WEB_WORKERS must be greater than zero");
        Ok(Self { host:std::env::var("HOST").unwrap_or_else(|_|"0.0.0.0".into()), port:std::env::var("PORT").unwrap_or_else(|_|"8080".into()).parse()?, mongodb_url, mongodb_database:std::env::var("MONGODB_DATABASE").unwrap_or_else(|_|"italy_developers".into()), cookie_key, cookie_secure, workers, public_url, upload_dir:std::env::var("UPLOAD_DIR").unwrap_or_else(|_|"uploads".into()) })
    }
}
