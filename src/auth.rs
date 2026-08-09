use actix_session::Session;
use actix_web::HttpResponse;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use mongodb::{bson::doc, bson::oid::ObjectId, Database};

use crate::models::Admin;

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("password hashing failed: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else { return false };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

fn login_redirect() -> HttpResponse {
    HttpResponse::Found().insert_header(("Location", "/admin/login")).finish()
}

pub async fn require_admin(session: &Session, db: &Database) -> Result<Admin, HttpResponse> {
    let id_hex = session.get::<String>("admin_id").ok().flatten().ok_or_else(login_redirect)?;
    let id = ObjectId::parse_str(&id_hex).map_err(|_| login_redirect())?;
    let admin = db
        .collection::<Admin>("admins")
        .find_one(doc! {"_id": id})
        .await
        .map_err(|_| login_redirect())?;
    admin.ok_or_else(login_redirect)
}
