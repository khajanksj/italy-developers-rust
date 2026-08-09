use actix_session::Session;
use actix_web::{web, HttpResponse};
use askama::Template;
use chrono::Utc;
use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId},
    Database,
};
use serde::Deserialize;
use validator::Validate;

use crate::{
    comments, db,
    error::AppError,
    handlers::html,
    models::{Comment, Faq, Insight, Lead, Service, SiteSettings, TeamMember, TestimonialView, WorkItem},
    security,
};

fn html_fragment(body: String) -> HttpResponse {
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(body)
}

fn render_markdown(input: &str) -> String {
    let parser = pulldown_cmark::Parser::new(input);
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, parser);
    out
}

// ---------- home ----------

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    services: Vec<Service>,
    work: Vec<WorkItem>,
    testimonials: Vec<TestimonialView>,
    faqs: Vec<Faq>,
    insights: Vec<Insight>,
    stats: SiteSettings,
}

pub async fn home(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let services = db::list_ordered(&db, "services", doc! {}, Some(5)).await?;
    let work = db::list_ordered(&db, "work_items", doc! {}, Some(3)).await?;
    let testimonials: Vec<crate::models::Testimonial> = db::list_ordered(&db, "testimonials", doc! {}, Some(3)).await?;
    let testimonials = testimonials.into_iter().map(TestimonialView::from).collect();
    let faqs = db::list_ordered(&db, "faqs", doc! {"page": "home"}, None).await?;
    let insights = db::list_ordered(&db, "insights", doc! {}, Some(3)).await?;
    let stats = db::get_settings(&db).await?;
    html(HomeTemplate { services, work, testimonials, faqs, insights, stats })
}

// ---------- services ----------

#[derive(Template)]
#[template(path = "services.html")]
struct ServicesTemplate {
    services: Vec<Service>,
    testimonials: Vec<TestimonialView>,
    faqs: Vec<Faq>,
}

pub async fn services(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let services = db::list_ordered(&db, "services", doc! {}, None).await?;
    let testimonials: Vec<crate::models::Testimonial> = db::list_ordered(&db, "testimonials", doc! {}, Some(2)).await?;
    let testimonials = testimonials.into_iter().map(TestimonialView::from).collect();
    let faqs = db::list_ordered(&db, "faqs", doc! {"page": "services"}, None).await?;
    html(ServicesTemplate { services, testimonials, faqs })
}

// ---------- work ----------

#[derive(Template)]
#[template(path = "work.html")]
struct WorkTemplate {
    work: Vec<WorkItem>,
    faqs: Vec<Faq>,
}

pub async fn work(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let work = db::list_ordered(&db, "work_items", doc! {}, None).await?;
    let faqs = db::list_ordered(&db, "faqs", doc! {"page": "work"}, None).await?;
    html(WorkTemplate { work, faqs })
}

// ---------- about ----------

#[derive(Template)]
#[template(path = "about.html")]
struct AboutTemplate {
    team: Vec<TeamMember>,
    faqs: Vec<Faq>,
    stats: SiteSettings,
}

pub async fn about(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let team = db::list_ordered(&db, "team_members", doc! {}, None).await?;
    let faqs = db::list_ordered(&db, "faqs", doc! {"page": "about"}, None).await?;
    let stats = db::get_settings(&db).await?;
    html(AboutTemplate { team, faqs, stats })
}

// ---------- insights ----------

#[derive(Template)]
#[template(path = "insights.html")]
struct InsightsTemplate {
    insights: Vec<Insight>,
}

pub async fn insights(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let insights = db::list_ordered(&db, "insights", doc! {}, None).await?;
    html(InsightsTemplate { insights })
}

#[derive(Template)]
#[template(path = "insight_detail.html")]
struct InsightDetailTemplate {
    insight: Insight,
    body_html: String,
    published: String,
    comments_html: String,
}

pub async fn insight_detail(session: Session, db: web::Data<Database>, path: web::Path<String>) -> Result<HttpResponse, AppError> {
    let slug = path.into_inner();
    let insight = db.collection::<Insight>("insights").find_one(doc! {"slug": &slug}).await?.ok_or(AppError::NotFound)?;
    let body_html = render_markdown(&insight.body);
    let published = insight.published_at.format("%-d %B %Y").to_string();
    let csrf = security::get_or_create_csrf(&session)?;
    let comments_html = render_comments(&db, &slug, &csrf).await?;
    html(InsightDetailTemplate { insight, body_html, published, comments_html })
}

async fn render_comments(db: &Database, slug: &str, csrf: &str) -> Result<String, AppError> {
    let list: Vec<Comment> = db.collection("comments").find(doc! {"slug": slug}).sort(doc! {"created_at": 1}).await?.try_collect().await?;
    Ok(comments::render_block(slug, &list, csrf))
}

#[derive(Deserialize, Validate)]
pub struct CommentForm {
    #[validate(length(min = 2, max = 80))]
    author: String,
    #[validate(length(min = 2, max = 2000))]
    body: String,
    #[validate(length(max = 0))]
    website: Option<String>,
    parent_id: Option<String>,
    csrf: String,
}

pub async fn create_comment(session: Session, db: web::Data<Database>, path: web::Path<String>, form: web::Form<CommentForm>) -> Result<HttpResponse, AppError> {
    let slug = path.into_inner();
    form.validate().map_err(|_| AppError::BadRequest)?;
    let expected = session.get::<String>("csrf").map_err(|_| AppError::Forbidden)?.ok_or(AppError::Forbidden)?;
    if !security::csrf_valid(&expected, &form.csrf) {
        return Err(AppError::Forbidden);
    }
    db.collection::<Insight>("insights").find_one(doc! {"slug": &slug}).await?.ok_or(AppError::NotFound)?;
    let parent_id = match form.parent_id.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => {
            let id = ObjectId::parse_str(raw).map_err(|_| AppError::BadRequest)?;
            let exists = db.collection::<Comment>("comments").count_documents(doc! {"_id": id, "slug": &slug}).await? > 0;
            if !exists {
                return Err(AppError::BadRequest);
            }
            Some(id)
        }
        None => None,
    };
    let comment = Comment { id: None, slug: slug.clone(), parent_id, author: form.author.trim().to_string(), body: form.body.trim().to_string(), created_at: Utc::now() };
    db.collection("comments").insert_one(comment).await?;
    let csrf = security::get_or_create_csrf(&session)?;
    let body = render_comments(&db, &slug, &csrf).await?;
    Ok(html_fragment(body))
}

pub async fn reply_form(session: Session, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    let (slug, id) = path.into_inner();
    let csrf = security::get_or_create_csrf(&session)?;
    Ok(html_fragment(comments::render_form(&slug, Some(&id), &csrf)))
}

pub async fn reply_cancel() -> HttpResponse {
    html_fragment(String::new())
}

// ---------- contact ----------

#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate<'a> {
    csrf: &'a str,
    success: bool,
    settings: SiteSettings,
    faqs: Vec<Faq>,
}

pub async fn contact(session: Session, db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let csrf = security::get_or_create_csrf(&session)?;
    let settings = db::get_settings(&db).await?;
    let faqs = db::list_ordered(&db, "faqs", doc! {"page": "contact"}, None).await?;
    html(ContactTemplate { csrf: &csrf, success: false, settings, faqs })
}

#[derive(Deserialize, Validate)]
pub struct ContactForm {
    #[validate(length(min = 2, max = 80))]
    name: String,
    #[validate(email, length(max = 254))]
    email: String,
    #[validate(length(max = 120))]
    company: Option<String>,
    #[validate(length(max = 80))]
    service: Option<String>,
    #[validate(length(min = 10, max = 4000))]
    message: String,
    #[validate(length(max = 0))]
    website: Option<String>,
    csrf: String,
}

pub async fn submit_contact(session: Session, db: web::Data<Database>, form: web::Form<ContactForm>) -> Result<HttpResponse, AppError> {
    form.validate().map_err(|_| AppError::BadRequest)?;
    let expected = session.get::<String>("csrf").map_err(|_| AppError::Forbidden)?.ok_or(AppError::Forbidden)?;
    if !security::csrf_valid(&expected, &form.csrf) {
        return Err(AppError::Forbidden);
    }
    let lead = Lead {
        id: None,
        name: form.name.trim().to_string(),
        email: form.email.trim().to_lowercase(),
        company: form.company.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from),
        service: form.service.clone(),
        message: form.message.trim().to_string(),
        status: "new".into(),
        created_at: Utc::now(),
    };
    db.collection("leads").insert_one(lead).await?;
    session.remove("csrf");
    let settings = db::get_settings(&db).await?;
    html(ContactTemplate { csrf: "", success: true, settings, faqs: vec![] })
}

// ---------- misc ----------

pub async fn not_found() -> Result<HttpResponse, AppError> {
    Err(AppError::NotFound)
}

pub async fn robots() -> HttpResponse {
    HttpResponse::Ok().content_type("text/plain; charset=utf-8").body("User-agent: *\nAllow: /\nDisallow: /admin/\nDisallow: /health/\nSitemap: https://italydevelopers.com/sitemap.xml\n")
}

pub async fn sitemap() -> HttpResponse {
    HttpResponse::Ok().content_type("application/xml; charset=utf-8").body("<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"><url><loc>https://italydevelopers.com/</loc><priority>1.0</priority></url><url><loc>https://italydevelopers.com/services</loc></url><url><loc>https://italydevelopers.com/work</loc></url><url><loc>https://italydevelopers.com/about</loc></url><url><loc>https://italydevelopers.com/insights</loc></url><url><loc>https://italydevelopers.com/contact</loc></url></urlset>")
}

pub async fn live() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

pub async fn ready(db: web::Data<Database>) -> HttpResponse {
    match db.run_command(doc! {"ping": 1}).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"status": "ready"})),
        Err(_) => HttpResponse::ServiceUnavailable().json(serde_json::json!({"status": "unavailable"})),
    }
}

