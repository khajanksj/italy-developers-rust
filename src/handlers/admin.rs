use std::{collections::HashMap, sync::LazyLock};

use actix_session::Session;
use actix_web::{web, HttpResponse};
use askama::Template;
use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, Document},
    Database,
};
use serde::Deserialize;

use crate::{auth, content, error::AppError, handlers::html, models::Admin, security};

static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| auth::hash_password("not-a-real-password-used-only-for-timing").unwrap());

fn check_csrf(session: &Session, received: &str) -> Result<(), AppError> {
    let expected = session.get::<String>("csrf").map_err(|_| AppError::Forbidden)?.ok_or(AppError::Forbidden)?;
    if !security::csrf_valid(&expected, received) {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

fn redirect(location: &str) -> HttpResponse {
    HttpResponse::Found().insert_header(("Location", location)).finish()
}

// ---------- login / logout ----------

#[derive(Template)]
#[template(path = "admin/login.html")]
struct LoginTemplate<'a> {
    csrf: &'a str,
    error: Option<&'a str>,
}

pub async fn login_page(session: Session) -> Result<HttpResponse, AppError> {
    let csrf = security::get_or_create_csrf(&session)?;
    html(LoginTemplate { csrf: &csrf, error: None })
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
    csrf: String,
}

pub async fn login_submit(session: Session, db: web::Data<Database>, form: web::Form<LoginForm>) -> Result<HttpResponse, AppError> {
    check_csrf(&session, &form.csrf)?;
    let admin = db.collection::<Admin>("admins").find_one(doc! {"username": form.username.trim()}).await?;
    let hash = admin.as_ref().map(|a| a.password_hash.as_str()).unwrap_or(DUMMY_HASH.as_str());
    let verified = auth::verify_password(&form.password, hash);
    let Some(admin) = admin.filter(|_| verified) else {
        let csrf = security::get_or_create_csrf(&session)?;
        return html(LoginTemplate { csrf: &csrf, error: Some("Invalid username or password.") });
    };
    session.insert("admin_id", admin.id.expect("persisted admin has an id").to_hex()).map_err(|_| AppError::BadRequest)?;
    session.remove("csrf");
    Ok(redirect("/admin"))
}

pub async fn logout(session: Session, form: web::Form<CsrfOnly>) -> Result<HttpResponse, AppError> {
    check_csrf(&session, &form.csrf)?;
    session.clear();
    Ok(redirect("/admin/login"))
}

// ---------- dashboard ----------

struct CountRow {
    label: String,
    key: String,
    count: u64,
}

#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct DashboardTemplate {
    username: String,
    leads_new: u64,
    leads_total: u64,
    content_counts: Vec<CountRow>,
    csrf: String,
}

pub async fn dashboard(session: Session, db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let admin = match auth::require_admin(&session, &db).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let leads = db.collection::<Document>("leads");
    let leads_total = leads.count_documents(doc! {}).await?;
    let leads_new = leads.count_documents(doc! {"status": "new"}).await?;
    let mut content_counts = Vec::with_capacity(content::ALL.len());
    for ct in content::ALL {
        let count = db.collection::<Document>(ct.collection).count_documents(doc! {}).await?;
        content_counts.push(CountRow { label: ct.label.into(), key: ct.key.into(), count });
    }
    let csrf = security::get_or_create_csrf(&session)?;
    html(DashboardTemplate { username: admin.username, leads_new, leads_total, content_counts, csrf })
}

// ---------- leads ----------

struct LeadRow {
    id: String,
    name: String,
    email: String,
    company: String,
    service: String,
    message: String,
    status: String,
    created_at: String,
}

#[derive(Template)]
#[template(path = "admin/leads.html")]
struct LeadsTemplate {
    username: String,
    leads: Vec<LeadRow>,
    status_filter: String,
    csrf: String,
}

#[derive(Deserialize)]
pub struct LeadsQuery {
    status: Option<String>,
}

pub async fn leads_list(session: Session, db: web::Data<Database>, q: web::Query<LeadsQuery>) -> Result<HttpResponse, AppError> {
    let admin = match auth::require_admin(&session, &db).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let status_filter = q.status.clone().unwrap_or_default();
    let filter = if status_filter.is_empty() { doc! {} } else { doc! {"status": &status_filter} };
    let leads: Vec<crate::models::Lead> = db.collection("leads").find(filter).sort(doc! {"created_at": -1}).await?.try_collect().await?;
    let csrf = security::get_or_create_csrf(&session)?;
    let rows = leads
        .into_iter()
        .map(|l| LeadRow {
            id: l.id.map(|i| i.to_hex()).unwrap_or_default(),
            name: l.name,
            email: l.email,
            company: l.company.unwrap_or_default(),
            service: l.service.unwrap_or_default(),
            message: l.message,
            status: l.status,
            created_at: l.created_at.format("%Y-%m-%d %H:%M UTC").to_string(),
        })
        .collect();
    html(LeadsTemplate { username: admin.username, leads: rows, status_filter, csrf })
}

#[derive(Deserialize)]
pub struct StatusForm {
    status: String,
    csrf: String,
}

const LEAD_STATUSES: [&str; 4] = ["new", "contacted", "closed", "spam"];

pub async fn lead_update_status(session: Session, db: web::Data<Database>, path: web::Path<String>, form: web::Form<StatusForm>) -> Result<HttpResponse, AppError> {
    if let Err(r) = auth::require_admin(&session, &db).await {
        return Ok(r);
    }
    check_csrf(&session, &form.csrf)?;
    if !LEAD_STATUSES.contains(&form.status.as_str()) {
        return Err(AppError::BadRequest);
    }
    let id = ObjectId::parse_str(path.into_inner()).map_err(|_| AppError::BadRequest)?;
    db.collection::<Document>("leads").update_one(doc! {"_id": id}, doc! {"$set": {"status": &form.status}}).await?;
    Ok(redirect("/admin/leads"))
}

#[derive(Deserialize)]
pub struct CsrfOnly {
    csrf: String,
}

pub async fn lead_delete(session: Session, db: web::Data<Database>, path: web::Path<String>, form: web::Form<CsrfOnly>) -> Result<HttpResponse, AppError> {
    if let Err(r) = auth::require_admin(&session, &db).await {
        return Ok(r);
    }
    check_csrf(&session, &form.csrf)?;
    let id = ObjectId::parse_str(path.into_inner()).map_err(|_| AppError::BadRequest)?;
    db.collection::<Document>("leads").delete_one(doc! {"_id": id}).await?;
    Ok(redirect("/admin/leads"))
}

// ---------- generic content CRUD (services, work, testimonials, team, faqs, insights) ----------

struct ListRow {
    id: String,
    title: String,
    subtitle: String,
}

#[derive(Template)]
#[template(path = "admin/list.html")]
struct ContentListTemplate {
    username: String,
    type_key: String,
    type_label: String,
    rows: Vec<ListRow>,
    csrf: String,
}

pub async fn content_list(session: Session, db: web::Data<Database>, path: web::Path<String>) -> Result<HttpResponse, AppError> {
    let admin = match auth::require_admin(&session, &db).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let ct = content::find(&path).ok_or(AppError::NotFound)?;
    let docs: Vec<Document> = db.collection(ct.collection).find(doc! {}).sort(doc! {"order": 1}).await?.try_collect().await?;
    let rows = docs
        .into_iter()
        .map(|d| ListRow {
            id: d.get_object_id("_id").map(|i| i.to_hex()).unwrap_or_default(),
            title: d.get_str(ct.title_field).unwrap_or_default().to_string(),
            subtitle: d.get_str(ct.subtitle_field).unwrap_or_default().to_string(),
        })
        .collect();
    let csrf = security::get_or_create_csrf(&session)?;
    html(ContentListTemplate { username: admin.username, type_key: ct.key.into(), type_label: ct.label.into(), rows, csrf })
}

struct FormFieldView {
    name: String,
    label: String,
    value: String,
    kind: &'static str,
    required: bool,
}

fn build_field_views(fields: &[content::FieldSpec], doc: Option<&Document>) -> Vec<FormFieldView> {
    fields
        .iter()
        .map(|field| {
            let value = doc
                .and_then(|d| match field.kind {
                    content::FieldKind::Number => d.get_i32(field.name).ok().map(|v| v.to_string()),
                    _ => d.get_str(field.name).ok().map(str::to_string),
                })
                .unwrap_or_default();
            FormFieldView {
                name: field.name.into(),
                label: field.label.into(),
                value,
                kind: match field.kind {
                    content::FieldKind::Text => "text",
                    content::FieldKind::TextArea => "textarea",
                    content::FieldKind::Number => "number",
                    content::FieldKind::Markdown => "markdown",
                },
                required: field.required,
            }
        })
        .collect()
}

fn build_document(fields: &[content::FieldSpec], form: &HashMap<String, String>) -> Result<Document, AppError> {
    let mut out = Document::new();
    for field in fields {
        let raw = form.get(field.name).map(|s| s.trim()).unwrap_or("");
        if field.required && raw.is_empty() {
            return Err(AppError::BadRequest);
        }
        match field.kind {
            content::FieldKind::Number => {
                let n: i32 = raw.parse().map_err(|_| AppError::BadRequest)?;
                out.insert(field.name, n);
            }
            _ => {
                out.insert(field.name, raw.to_string());
            }
        }
    }
    Ok(out)
}

#[derive(Template)]
#[template(path = "admin/form.html")]
struct ContentFormTemplate {
    username: String,
    heading: String,
    action: String,
    back: String,
    fields: Vec<FormFieldView>,
    csrf: String,
    delete_action: Option<String>,
}

pub async fn content_new(session: Session, db: web::Data<Database>, path: web::Path<String>) -> Result<HttpResponse, AppError> {
    let admin = match auth::require_admin(&session, &db).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let ct = content::find(&path).ok_or(AppError::NotFound)?;
    let csrf = security::get_or_create_csrf(&session)?;
    html(ContentFormTemplate {
        username: admin.username,
        heading: format!("New {}", ct.label),
        action: format!("/admin/content/{}/new", ct.key),
        back: format!("/admin/content/{}", ct.key),
        fields: build_field_views(ct.fields, None),
        csrf,
        delete_action: None,
    })
}

pub async fn content_create(session: Session, db: web::Data<Database>, path: web::Path<String>, form: web::Form<HashMap<String, String>>) -> Result<HttpResponse, AppError> {
    if let Err(r) = auth::require_admin(&session, &db).await {
        return Ok(r);
    }
    let ct = content::find(&path).ok_or(AppError::NotFound)?;
    check_csrf(&session, form.get("csrf").map(String::as_str).unwrap_or(""))?;
    let mut doc = build_document(ct.fields, &form)?;
    if ct.key == "insights" {
        doc.insert("published_at", mongodb::bson::DateTime::now());
    }
    db.collection::<Document>(ct.collection).insert_one(doc).await?;
    Ok(redirect(&format!("/admin/content/{}", ct.key)))
}

pub async fn content_edit(session: Session, db: web::Data<Database>, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    let admin = match auth::require_admin(&session, &db).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let (type_key, id_hex) = path.into_inner();
    let ct = content::find(&type_key).ok_or(AppError::NotFound)?;
    let id = ObjectId::parse_str(&id_hex).map_err(|_| AppError::BadRequest)?;
    let doc = db.collection::<Document>(ct.collection).find_one(doc! {"_id": id}).await?.ok_or(AppError::NotFound)?;
    let csrf = security::get_or_create_csrf(&session)?;
    html(ContentFormTemplate {
        username: admin.username,
        heading: format!("Edit {}", ct.label),
        action: format!("/admin/content/{}/{}/edit", ct.key, id_hex),
        back: format!("/admin/content/{}", ct.key),
        fields: build_field_views(ct.fields, Some(&doc)),
        csrf,
        delete_action: Some(format!("/admin/content/{}/{}/delete", ct.key, id_hex)),
    })
}

pub async fn content_update(session: Session, db: web::Data<Database>, path: web::Path<(String, String)>, form: web::Form<HashMap<String, String>>) -> Result<HttpResponse, AppError> {
    if let Err(r) = auth::require_admin(&session, &db).await {
        return Ok(r);
    }
    let (type_key, id_hex) = path.into_inner();
    let ct = content::find(&type_key).ok_or(AppError::NotFound)?;
    check_csrf(&session, form.get("csrf").map(String::as_str).unwrap_or(""))?;
    let id = ObjectId::parse_str(&id_hex).map_err(|_| AppError::BadRequest)?;
    let doc = build_document(ct.fields, &form)?;
    db.collection::<Document>(ct.collection).update_one(doc! {"_id": id}, doc! {"$set": doc}).await?;
    Ok(redirect(&format!("/admin/content/{}", ct.key)))
}

pub async fn content_delete(session: Session, db: web::Data<Database>, path: web::Path<(String, String)>, form: web::Form<CsrfOnly>) -> Result<HttpResponse, AppError> {
    if let Err(r) = auth::require_admin(&session, &db).await {
        return Ok(r);
    }
    let (type_key, id_hex) = path.into_inner();
    let ct = content::find(&type_key).ok_or(AppError::NotFound)?;
    check_csrf(&session, &form.csrf)?;
    let id = ObjectId::parse_str(&id_hex).map_err(|_| AppError::BadRequest)?;
    db.collection::<Document>(ct.collection).delete_one(doc! {"_id": id}).await?;
    Ok(redirect(&format!("/admin/content/{}", ct.key)))
}

// ---------- site settings ----------

const SETTINGS_FIELDS: &[content::FieldSpec] = &[
    content::f("phone", "Phone", content::FieldKind::Text, true),
    content::f("email", "Contact email", content::FieldKind::Text, true),
    content::f("address", "Address", content::FieldKind::Text, true),
    content::f("twitter", "Twitter / X URL", content::FieldKind::Text, false),
    content::f("linkedin", "LinkedIn URL", content::FieldKind::Text, false),
    content::f("github", "GitHub URL", content::FieldKind::Text, false),
    content::f("years_active", "Years active", content::FieldKind::Number, true),
    content::f("projects_delivered", "Projects delivered", content::FieldKind::Number, true),
    content::f("countries_served", "Countries served", content::FieldKind::Number, true),
    content::f("response_time", "Typical response time", content::FieldKind::Text, true),
];

pub async fn settings_page(session: Session, db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let admin = match auth::require_admin(&session, &db).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let settings = crate::db::get_settings(&db).await?;
    let doc = mongodb::bson::to_document(&settings).map_err(|_| AppError::BadRequest)?;
    let csrf = security::get_or_create_csrf(&session)?;
    html(ContentFormTemplate {
        username: admin.username,
        heading: "Site settings".into(),
        action: "/admin/settings".into(),
        back: "/admin".into(),
        fields: build_field_views(SETTINGS_FIELDS, Some(&doc)),
        csrf,
        delete_action: None,
    })
}

pub async fn settings_update(session: Session, db: web::Data<Database>, form: web::Form<HashMap<String, String>>) -> Result<HttpResponse, AppError> {
    if let Err(r) = auth::require_admin(&session, &db).await {
        return Ok(r);
    }
    check_csrf(&session, form.get("csrf").map(String::as_str).unwrap_or(""))?;
    let doc = build_document(SETTINGS_FIELDS, &form)?;
    db.collection::<Document>("site_settings").update_one(doc! {}, doc! {"$set": doc}).upsert(true).await?;
    Ok(redirect("/admin/settings"))
}

// ---------- admin users ----------

struct AdminRow {
    id: String,
    username: String,
    created_at: String,
}

#[derive(Template)]
#[template(path = "admin/admins.html")]
struct AdminsTemplate {
    username: String,
    admins: Vec<AdminRow>,
    csrf: String,
}

pub async fn admins_list(session: Session, db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let admin = match auth::require_admin(&session, &db).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let rows: Vec<Admin> = db.collection("admins").find(doc! {}).sort(doc! {"created_at": 1}).await?.try_collect().await?;
    let csrf = security::get_or_create_csrf(&session)?;
    let rows = rows
        .into_iter()
        .map(|a| AdminRow {
            id: a.id.map(|i| i.to_hex()).unwrap_or_default(),
            username: a.username,
            created_at: a.created_at.format("%Y-%m-%d").to_string(),
        })
        .collect();
    html(AdminsTemplate { username: admin.username, admins: rows, csrf })
}

#[derive(Deserialize)]
pub struct NewAdminForm {
    username: String,
    password: String,
    csrf: String,
}

pub async fn admins_create(session: Session, db: web::Data<Database>, form: web::Form<NewAdminForm>) -> Result<HttpResponse, AppError> {
    if let Err(r) = auth::require_admin(&session, &db).await {
        return Ok(r);
    }
    check_csrf(&session, &form.csrf)?;
    let username = form.username.trim();
    if username.len() < 3 || form.password.len() < 12 {
        return Err(AppError::BadRequest);
    }
    let password_hash = auth::hash_password(&form.password).map_err(|_| AppError::BadRequest)?;
    db.collection("admins")
        .insert_one(Admin { id: None, username: username.to_string(), password_hash, created_at: chrono::Utc::now() })
        .await?;
    Ok(redirect("/admin/admins"))
}

pub async fn admins_delete(session: Session, db: web::Data<Database>, path: web::Path<String>, form: web::Form<CsrfOnly>) -> Result<HttpResponse, AppError> {
    if let Err(r) = auth::require_admin(&session, &db).await {
        return Ok(r);
    }
    check_csrf(&session, &form.csrf)?;
    let coll = db.collection::<Admin>("admins");
    if coll.count_documents(doc! {}).await? <= 1 {
        return Err(AppError::BadRequest);
    }
    let id = ObjectId::parse_str(path.into_inner()).map_err(|_| AppError::BadRequest)?;
    coll.delete_one(doc! {"_id": id}).await?;
    Ok(redirect("/admin/admins"))
}
