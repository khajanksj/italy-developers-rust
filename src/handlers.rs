use actix_multipart::Multipart;
use actix_session::Session;
use actix_web::{http::header, web, HttpRequest, HttpResponse};
use askama::Template;
use futures_util::{StreamExt, TryStreamExt};
use mongodb::{
    bson::{doc, oid::ObjectId, DateTime, Document},
    options::IndexOptions,
    Database,
    IndexModel,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use validator::Validate;

use crate::{
    config::Config,
    error::AppError,
    i18n::{self, Ui},
    security,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentItem {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default = "default_lang")]
    pub lang: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub eyebrow: String,
    #[serde(default)]
    pub summary: String,
    /// Unique "at a glance" sidebar highlight for this item's detail page.
    #[serde(default)]
    pub glance: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub image_alt: String,
    #[serde(default)]
    pub seo_title: String,
    #[serde(default)]
    pub seo_description: String,
    #[serde(default)]
    pub keywords: String,
    #[serde(default)]
    pub cta: String,
    /// Where the `cta` label should link to (e.g. "/work/some-project"). Empty means no link.
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub published: bool,
    #[serde(default)]
    pub order: i32,
    #[serde(default = "default_datetime")]
    pub created_at: DateTime,
    #[serde(default = "default_datetime")]
    pub updated_at: DateTime,
}

fn default_datetime() -> DateTime {
    DateTime::from_millis(0)
}
fn default_lang() -> String {
    "en".into()
}
impl Default for ContentItem {
    fn default() -> Self {
        Self {
            id: None,
            kind: String::new(),
            slug: String::new(),
            lang: default_lang(),
            title: String::new(),
            eyebrow: String::new(),
            summary: String::new(),
            glance: String::new(),
            body: String::new(),
            image: String::new(),
            image_alt: String::new(),
            seo_title: String::new(),
            seo_description: String::new(),
            keywords: String::new(),
            cta: String::new(),
            link: String::new(),
            featured: false,
            published: false,
            order: 0,
            created_at: default_datetime(),
            updated_at: default_datetime(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Lead {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
    email: String,
    company: String,
    service: String,
    message: String,
    status: String,
    created_at: DateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BlogComment {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    post_slug: String,
    parent_id: Option<ObjectId>,
    #[serde(default)]
    user_id: String,
    #[serde(default)]
    author_email: String,
    author: String,
    body: String,
    likes: i64,
    published: bool,
    created_at: DateTime,
}

#[derive(Clone)]
struct CommentView {
    id: String,
    author: String,
    body: String,
    likes: i64,
    depth: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BlogReaction {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    target: String,
    visitor: String,
    created_at: DateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AdminUser {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    #[serde(default)]
    name: String,
    email: String,
    password_hash: String,
    role: String,
    active: bool,
    created_at: DateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HomeSettings {
    key: String,
    show_services: bool,
    service_limit: i64,
    show_work: bool,
    work_limit: i64,
    show_insights: bool,
    insight_limit: i64,
    show_testimonials: bool,
    testimonial_limit: i64,
    #[serde(default = "default_github_url")]
    github_url: String,
    #[serde(default)]
    linkedin_url: String,
    #[serde(default)]
    instagram_url: String,
    #[serde(default)]
    youtube_url: String,
}

fn default_github_url() -> String { "https://github.com/khajanksj".into() }

impl Default for HomeSettings {
    fn default() -> Self {
        Self {
            key: "home".into(),
            show_services: true,
            service_limit: 3,
            show_work: true,
            work_limit: 3,
            show_insights: true,
            insight_limit: 3,
            show_testimonials: true,
            testimonial_limit: 6,
            github_url: default_github_url(),
            linkedin_url: String::new(),
            instagram_url: String::new(),
            youtube_url: String::new(),
        }
    }
}

#[derive(Clone, Default)]
struct EditorErrors {
    title: String,
    slug: String,
    eyebrow: String,
    summary: String,
    body: String,
    seo_title: String,
    seo_description: String,
    image: String,
    form: String,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    services: Vec<ContentItem>,
    work: Vec<ContentItem>,
    insights: Vec<ContentItem>,
    testimonials: Vec<ContentItem>,
    lang: String,
    prefix: String,
    path_no_prefix: String,
    t: Ui,
}
#[derive(Template)]
#[template(path = "collection.html")]
struct CollectionTemplate {
    title: String,
    description: String,
    canonical: String,
    eyebrow: String,
    heading: String,
    intro: String,
    kind: String,
    items: Vec<ContentItem>,
    lang: String,
    prefix: String,
    path_no_prefix: String,
    t: Ui,
}
#[derive(Template)]
#[template(path = "detail.html")]
struct DetailTemplate {
    item: ContentItem,
    canonical: String,
    schema_type: String,
    csrf: String,
    comments: Vec<CommentView>,
    post_likes: i64,
    authenticated: bool,
    viewer_name: String,
    lang: String,
    prefix: String,
    path_no_prefix: String,
    t: Ui,
}

#[derive(Template)]
#[template(path = "member/auth.html")]
struct MemberAuthTemplate {
    register: bool,
    next: String,
    error: String,
    csrf: String,
    lang: String,
    prefix: String,
    path_no_prefix: String,
    t: Ui,
}
#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate {
    csrf: String,
    success: bool,
    lang: String,
    prefix: String,
    path_no_prefix: String,
    t: Ui,
}
#[derive(Template)]
#[template(path = "admin/login.html")]
struct AdminLoginTemplate {
    email: String,
    email_error: String,
    password_error: String,
    form_error: String,
}
#[derive(Template)]
#[template(path = "admin/dashboard.html")]
struct AdminDashboardTemplate {
    items: Vec<ContentItem>,
    leads: Vec<Lead>,
    actor_email: String,
    role: String,
    can_delete: bool,
    toast: String,
    hidden_by_limit: HashSet<String>,
}
#[derive(Clone)]
struct LangTab {
    code: &'static str,
    label: &'static str,
    item: ContentItem,
}
const LANG_TABS: [(&str, &str); 5] = [
    ("en", "English"),
    ("it", "Italiano"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("pt", "Português"),
];
/// Builds one tab per supported language, pulling in whichever sibling
/// documents already exist (matched by `lang`) and leaving the rest empty.
fn lang_tabs_from(existing: &[ContentItem]) -> Vec<LangTab> {
    LANG_TABS
        .iter()
        .map(|&(code, label)| {
            let item = existing
                .iter()
                .find(|e| e.lang == code)
                .cloned()
                .unwrap_or_else(|| ContentItem {
                    lang: code.into(),
                    ..ContentItem::default()
                });
            LangTab { code, label, item }
        })
        .collect()
}
#[derive(Template)]
#[template(path = "admin/editor.html")]
struct AdminEditorTemplate {
    shared: ContentItem,
    langs: Vec<LangTab>,
    is_new: bool,
    errors: EditorErrors,
    can_publish: bool,
}

#[derive(Template)]
#[template(path = "admin/homepage.html")]
struct AdminHomepageTemplate {
    settings: HomeSettings,
}

fn html<T: Template>(template: T) -> Result<HttpResponse, AppError> {
    let public_url =
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "https://italydevelopers.com".into());
    let rendered = template
        .render()?
        .replace("http://localhost:8080", public_url.trim_end_matches('/'));
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(rendered))
}
fn content(db: &Database) -> mongodb::Collection<ContentItem> {
    db.collection("content")
}
fn leads(db: &Database) -> mongodb::Collection<Lead> {
    db.collection("leads")
}
fn blog_comments(db: &Database) -> mongodb::Collection<BlogComment> {
    db.collection("blog_comments")
}
fn blog_reactions(db: &Database) -> mongodb::Collection<BlogReaction> {
    db.collection("blog_reactions")
}
fn users(db: &Database) -> mongodb::Collection<AdminUser> {
    db.collection("admin_users")
}
fn home_settings_collection(db: &Database) -> mongodb::Collection<HomeSettings> {
    db.collection("site_settings")
}

async fn home_settings(db: &Database) -> Result<HomeSettings, AppError> {
    if let Some(settings) = home_settings_collection(db)
        .find_one(doc! {"key":"home"})
        .await?
    {
        return Ok(settings);
    }
    let settings = HomeSettings::default();
    home_settings_collection(db).insert_one(&settings).await?;
    Ok(settings)
}

pub async fn user_command(db: &Database, args: &[String]) -> anyhow::Result<()> {
    let (role, email, password) = match args.first().map(String::as_str) {
        Some("create-superuser") if args.len() == 3 => {
            ("superuser", args[1].trim().to_lowercase(), args[2].clone())
        }
        Some("create-admin") if args.len() == 3 => {
            ("admin", args[1].trim().to_lowercase(), args[2].clone())
        }
        Some("create-staff") if args.len() == 3 => {
            ("staff", args[1].trim().to_lowercase(), args[2].clone())
        }
        _ => anyhow::bail!(
            "Usage: app create-superuser|create-admin|create-staff <email> <password>"
        ),
    };
    anyhow::ensure!(
        email.contains('@') && email.len() <= 254,
        "Enter a valid email address"
    );
    anyhow::ensure!(
        password.len() >= 12,
        "Password must contain at least 12 characters"
    );
    let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
    let user = AdminUser {
        id: None,
        name: email.split('@').next().unwrap_or("Member").into(),
        email: email.clone(),
        password_hash: hash,
        role: role.into(),
        active: true,
        created_at: DateTime::now(),
    };
    users(db)
        .replace_one(doc! {"email":&email}, user)
        .upsert(true)
        .await?;
    println!("Created or updated {role}: {email}");
    Ok(())
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::get().to(root))
        .route("/services", web::get().to(services))
        .route("/services/{slug}", web::get().to(service_detail))
        .route("/work", web::get().to(work))
        .route("/work/{slug}", web::get().to(work_detail))
        .route("/about", web::get().to(about))
        .route("/about/{slug}", web::get().to(about_detail))
        .route("/tech-stack", web::get().to(tech_stack))
        .route("/tech-stack/{slug}", web::get().to(tech_detail))
        .route("/insights", web::get().to(insights))
        .route("/insights/{slug}", web::get().to(insight_detail))
        .route("/blog", web::get().to(blog))
        .route("/blog/{slug}", web::get().to(blog_detail))
        .service(
            web::scope("/{lang:it|de|fr|pt}")
                .route("", web::get().to(home_localized))
                .route("/", web::get().to(home_localized))
                .route("/services", web::get().to(services_i18n))
                .route("/services/{slug}", web::get().to(service_detail_i18n))
                .route("/work", web::get().to(work_i18n))
                .route("/work/{slug}", web::get().to(work_detail_i18n))
                .route("/about", web::get().to(about_i18n))
                .route("/about/{slug}", web::get().to(about_detail_i18n))
                .route("/tech-stack", web::get().to(tech_stack_i18n))
                .route("/tech-stack/{slug}", web::get().to(tech_detail_i18n))
                .route("/insights", web::get().to(insights_i18n))
                .route("/insights/{slug}", web::get().to(insight_detail_i18n))
                .route("/blog", web::get().to(blog_i18n))
                .route("/blog/{slug}", web::get().to(blog_detail_i18n)),
        )
        .route("/blog/{slug}/comment", web::post().to(add_blog_comment))
        .route("/blog/{slug}/like", web::post().to(toggle_blog_like))
        .route("/blog/{slug}/comments/{id}/like", web::post().to(toggle_comment_like))
        .route("/login", web::get().to(member_login))
        .route("/login", web::post().to(member_auth))
        .route("/register", web::get().to(member_register))
        .route("/register", web::post().to(member_create))
        .route("/logout", web::post().to(member_logout))
        .route("/social/{platform}", web::get().to(social_redirect))
        .route("/api/social-links", web::get().to(social_links))
        .route("/contact", web::get().to(contact_page))
        .route("/contact", web::post().to(submit_contact))
        .route("/media/covers/{kind}/{slug}.svg", web::get().to(content_cover))
        .route("/robots.txt", web::get().to(robots))
        .route("/sitemap.xml", web::get().to(sitemap))
        .route("/health/live", web::get().to(live))
        .route("/health/ready", web::get().to(ready))
        .route("/admin", web::get().to(admin_dashboard))
        .route("/admin/login", web::get().to(admin_login))
        .route("/admin/login", web::post().to(admin_auth))
        .route("/admin/logout", web::post().to(admin_logout))
        .route("/admin/content/new", web::get().to(admin_new))
        .route("/admin/content/{id}/edit", web::get().to(admin_edit))
        .route("/admin/content/save", web::post().to(admin_save))
        .route("/admin/content/{id}/delete", web::post().to(admin_delete))
        .route("/admin/content/{id}/toggle/{field}", web::post().to(admin_toggle))
        .route("/admin/homepage", web::get().to(admin_homepage))
        .route("/admin/homepage", web::post().to(admin_homepage_save))
        .route(
            "/admin/leads/{id}/status",
            web::post().to(admin_lead_status),
        )
        .route(
            "/admin/leads/{id}/delete",
            web::post().to(admin_lead_delete),
        );
}

async fn ensure_seed(db: &Database) -> Result<(), AppError> {
    let migrations = db.collection::<mongodb::bson::Document>("content_migrations");
    blog_comments(db).create_index(IndexModel::builder().keys(doc! {"post_slug":1,"created_at":1}).build()).await?;
    blog_reactions(db).create_index(IndexModel::builder().keys(doc! {"target":1,"visitor":1}).options(IndexOptions::builder().unique(true).build()).build()).await?;
    // Documents created before multi-language support have no `lang` field at all —
    // Mongo query filters like `lang:{"$in":["en"]}` do NOT match a missing field
    // (serde's `#[serde(default)]` only helps when *reading* a matched document), so
    // without this backfill every pre-existing item would silently vanish from every
    // listing. Idempotent: matches nothing once applied.
    content(db)
        .update_many(doc! {"lang": {"$exists": false}}, doc! {"$set": {"lang": "en"}})
        .await?;
    if migrations.find_one(doc! {"key":"translations-services-v1"}).await?.is_none() {
        apply_translations_services_v1(db).await?;
        migrations.insert_one(doc! {"key":"translations-services-v1","applied_at":DateTime::now()}).await?;
    }
    if migrations.find_one(doc! {"key":"translations-work-v1"}).await?.is_none() {
        apply_translations_work_v1(db).await?;
        migrations.insert_one(doc! {"key":"translations-work-v1","applied_at":DateTime::now()}).await?;
    }
    if migrations.find_one(doc! {"key":"translations-blog-v1"}).await?.is_none() {
        apply_translations_blog_v1(db).await?;
        migrations.insert_one(doc! {"key":"translations-blog-v1","applied_at":DateTime::now()}).await?;
    }
    if migrations.find_one(doc! {"key":"testimonial-links-v14"}).await?.is_some() {
        return Ok(());
    }
    if migrations.find_one(doc! {"key":"unique-glance-v13"}).await?.is_some() {
        apply_project_proof_v9(db, DateTime::now()).await?;
        migrations.insert_one(doc! {"key":"testimonial-links-v14","applied_at":DateTime::now()}).await?;
        return Ok(());
    }
    if migrations.find_one(doc! {"key":"image-consistency-insights-v12"}).await?.is_some() {
        apply_unique_glance_v13(db).await?;
        migrations.insert_one(doc! {"key":"unique-glance-v13","applied_at":DateTime::now()}).await?;
        apply_project_proof_v9(db, DateTime::now()).await?;
        migrations.insert_one(doc! {"key":"testimonial-links-v14","applied_at":DateTime::now()}).await?;
        return Ok(());
    }
    if migrations.find_one(doc! {"key":"market-positioning-v11"}).await?.is_some() {
        apply_image_consistency_and_insights_v12(db, DateTime::now()).await?;
        migrations.insert_one(doc! {"key":"image-consistency-insights-v12","applied_at":DateTime::now()}).await?;
        apply_unique_glance_v13(db).await?;
        migrations.insert_one(doc! {"key":"unique-glance-v13","applied_at":DateTime::now()}).await?;
        apply_project_proof_v9(db, DateTime::now()).await?;
        migrations.insert_one(doc! {"key":"testimonial-links-v14","applied_at":DateTime::now()}).await?;
        return Ok(());
    }
    if migrations.find_one(doc! {"key":"real-photos-v10"}).await?.is_some() {
        apply_market_positioning_v11(db).await?;
        migrations.insert_one(doc! {"key":"market-positioning-v11","applied_at":DateTime::now()}).await?;
        apply_image_consistency_and_insights_v12(db, DateTime::now()).await?;
        migrations.insert_one(doc! {"key":"image-consistency-insights-v12","applied_at":DateTime::now()}).await?;
        apply_unique_glance_v13(db).await?;
        migrations.insert_one(doc! {"key":"unique-glance-v13","applied_at":DateTime::now()}).await?;
        apply_project_proof_v9(db, DateTime::now()).await?;
        migrations.insert_one(doc! {"key":"testimonial-links-v14","applied_at":DateTime::now()}).await?;
        return Ok(());
    }
    if migrations.find_one(doc! {"key":"editorial-v9"}).await?.is_some() {
        apply_real_photos_v10(db).await?;
        migrations.insert_one(doc! {"key":"real-photos-v10","applied_at":DateTime::now()}).await?;
        apply_market_positioning_v11(db).await?;
        migrations.insert_one(doc! {"key":"market-positioning-v11","applied_at":DateTime::now()}).await?;
        apply_image_consistency_and_insights_v12(db, DateTime::now()).await?;
        migrations.insert_one(doc! {"key":"image-consistency-insights-v12","applied_at":DateTime::now()}).await?;
        apply_unique_glance_v13(db).await?;
        migrations.insert_one(doc! {"key":"unique-glance-v13","applied_at":DateTime::now()}).await?;
        return Ok(());
    }
    let now = DateTime::now();
    let data=[
        ("service","siti-web-per-piccole-imprese","Websites for Italian small businesses","A fast, credible website designed to turn local searches into calls, bookings and qualified enquiries.","<p class=\"lead\">A small-business website does not need dozens of pages. It needs to explain what you do, prove that you do it well and make the next step effortless on a phone.</p><h2>What we build</h2><p>We plan the customer journey, write a clear page structure and build a responsive, accessible website you can manage. Typical launches include Home, Services, About, Work, FAQs and Contact, plus dedicated location or service pages where search demand justifies them.</p><div class=\"fact-grid\"><p><strong>Typical timeline</strong><br>4–7 weeks</p><p><strong>Best for</strong><br>Local services, hospitality and professionals</p></div><h2>Included in the launch</h2><ul><li>Discovery workshop and content plan</li><li>Original responsive design, not a recycled theme</li><li>Technical SEO, analytics and consent-aware measurement</li><li>Google Business Profile and local-search recommendations</li><li>Accessible forms, WhatsApp or booking journey</li><li>Training, documentation and 30 days of launch support</li></ul><h2>A sensible first release</h2><p>We agree one commercial goal and prioritise the pages that support it. Optional languages, integrations and ongoing content are priced separately, so the core project stays understandable.</p><h3>What you keep</h3><p>Your domain, content, source code and data remain under your control. We avoid unnecessary subscriptions and document every external service.</p>","Website design","/static/images/small-business-websites.png"),
        ("service","ecommerce-accessibile","Lean e-commerce for local brands","A focused, easy-to-run storefront for artisans, food producers and specialist retailers.","<p class=\"lead\">E-commerce works when operations are as carefully designed as the storefront. We begin with products, margins, fulfilment and customer questions—not decorative features.</p><h2>From catalogue to delivery</h2><p>We structure collections, product data, shipping zones, payments, returns and order emails as one coherent journey. The storefront is built for mobile shoppers and the back office for the person who runs it every day.</p><div class=\"fact-grid\"><p><strong>Typical timeline</strong><br>6–10 weeks</p><p><strong>Good first scope</strong><br>Up to 50 focused products</p></div><h2>Core deliverables</h2><ul><li>Product and category information architecture</li><li>Stripe or established platform payment setup</li><li>Italian tax, privacy and cookie implementation support</li><li>Shipping, click-and-collect or local-delivery rules</li><li>Search-friendly product templates and structured data</li><li>Order testing, owner training and launch checklist</li></ul><h2>Designed for reality</h2><p>We test slow connections, small screens, failed payments, out-of-stock products and confirmation messages. Legal and accounting decisions remain with your qualified advisers; we make their requirements clear in the product.</p><h3>Grow after evidence</h3><p>Loyalty, wholesale access, subscriptions and advanced automation are valuable only when order volume supports them. We keep those as planned extensions.</p>","E-commerce","/static/images/lean-ecommerce.png"),
        ("service","automazione-processi","Small-business workflow automation","Connect enquiries, quotations, documents and reminders without replacing every tool your team already knows.","<p class=\"lead\">Automation should remove repetitive work while keeping people in control. We map the current process first, including exceptions and hand-offs.</p><h2>Useful starting points</h2><p>Common projects include qualified enquiry routing, quote generation, appointment reminders, document requests, customer portals and weekly management reports.</p><div class=\"fact-grid\"><p><strong>Typical pilot</strong><br>2–5 weeks</p><p><strong>Success measure</strong><br>Time saved and fewer missed steps</p></div><h2>Our process</h2><ol><li>Observe the real workflow and measure its baseline.</li><li>Choose one high-volume, low-risk process.</li><li>Build approvals, error handling and an audit trail.</li><li>Run the old and new process in parallel before switching.</li></ol><h2>Privacy and resilience</h2><p>We minimise personal data, define retention, restrict access and document what happens when an integration is unavailable. High-impact decisions stay with a person.</p><h3>A pilot with a clear boundary</h3><p>You receive the workflow map, implementation, operating guide and a review after real use. The next phase is based on evidence, not a long wishlist.</p>","Automation","/static/images/workflow-automation.png"),
        ("work","osteria-verde","Osteria Verde — direct reservations","A representative hospitality concept showing how local search, menus and a low-friction booking journey work together.","<p class=\"lead\"><em>Demonstration case study: this concept illustrates our process and does not claim results for a trading client.</em></p><h2>The situation</h2><p>A neighbourhood osteria depends on discovery through Maps and social media, but guests still need current menus, dietary information, opening hours and a dependable way to reserve.</p><h2>The proposed experience</h2><p>We designed Italian and English landing pages around intent: lunch, dinner, groups and seasonal events. The menu is readable HTML rather than a hard-to-use PDF. Persistent booking actions pass date, party size and contact details to one system.</p><div class=\"fact-grid\"><p><strong>Sector</strong><br>Independent hospitality</p><p><strong>Primary KPI</strong><br>Completed direct reservations</p></div><h2>Measurement plan</h2><ul><li>Track calls, direction requests and booking completions separately</li><li>Compare direct bookings with marketplace referrals</li><li>Review unanswered searches and menu interactions monthly</li></ul><h3>Why the approach is credible</h3><p>It reduces duplicated updates, keeps essential information crawlable and makes the booking path obvious without inventing promotional claims.</p>","Concept case study","/static/images/small-business-websites.png"),
        ("work","falegnameria-rossi","Falegnameria Rossi — better quote requests","A representative portfolio concept for turning craftsmanship into specific, useful project enquiries.","<p class=\"lead\"><em>Demonstration case study: the business and outcome framework are illustrative.</em></p><h2>The problem</h2><p>A workshop can receive many vague requests that take hours to qualify. At the same time, compressed social images rarely communicate materials, joinery or the difference between custom and catalogue work.</p><h2>The proposed system</h2><p>Project stories are organised by room and service, with photography guidance for wide context, close detail and material finish. A guided enquiry asks for location, dimensions, inspiration, timing and an honest budget band.</p><div class=\"fact-grid\"><p><strong>Sector</strong><br>Custom joinery</p><p><strong>Primary KPI</strong><br>Qualified quote requests</p></div><h2>Content that earns trust</h2><ul><li>Process from survey to installation</li><li>Material and finish explanations</li><li>Representative lead times and service area</li><li>Real project constraints, not generic testimonials</li></ul><h3>The intended outcome</h3><p>Fewer unsuitable enquiries, better first conversations and a portfolio the owner can update without rebuilding the page.</p>","Concept case study","/static/images/lean-ecommerce.png"),
        ("work","studio-contabile-luce","Studio Contabile Luce — clearer client intake","A representative professional-services concept that turns complex services into a secure, understandable first step.","<p class=\"lead\"><em>Demonstration case study: no client relationship or performance result is implied.</em></p><h2>The challenge</h2><p>Accounting websites often describe internal categories while prospective clients think in moments: opening a company, hiring, selling abroad or changing adviser.</p><h2>The proposed experience</h2><p>We reorganised navigation around those moments, explained who each service fits and separated general enquiries from document exchange. Sensitive files use authenticated storage rather than ordinary email attachments.</p><div class=\"fact-grid\"><p><strong>Sector</strong><br>Professional services</p><p><strong>Priority</strong><br>Trust and data minimisation</p></div><h2>Workflow design</h2><ul><li>Short eligibility form before consultation booking</li><li>Explicit privacy notice at the point of collection</li><li>Document checklist generated from the selected need</li><li>Status reminders without exposing confidential details</li></ul><h3>What success would mean</h3><p>More complete first enquiries, less administrative chasing and a clear audit trail for each request.</p>","Concept case study","/static/images/workflow-automation.png"),
        ("insight","quanto-costa-un-sito-web-in-italia","What should a small-business website cost in Italy?","A transparent framework for setting scope and comparing proposals—without pretending there is one universal price.","<p class=\"lead\">Price follows scope, risk and responsibility. A five-page brochure assembled from supplied copy is a different product from a multilingual site with original research, photography, booking and migration.</p><h2>Use ranges as orientation</h2><p>Freelance and studio pricing varies widely by region and experience. Instead of treating a number as a market guarantee, ask what work is included: discovery, writing, design, development, accessibility, SEO, analytics, testing and support.</p><div class=\"fact-grid\"><p><strong>Lean launch</strong><br>Focused pages, supplied assets</p><p><strong>Custom build</strong><br>Research, content and integrations</p></div><h2>Questions every proposal should answer</h2><ul><li>Who owns the domain, code, design and accounts?</li><li>How many revision rounds and page templates are included?</li><li>Are copy, translations, photography and legal review included?</li><li>What recurring platform, plugin and maintenance costs apply?</li><li>How are accessibility, performance and backups tested?</li></ul><h2>Choose the commercial job first</h2><p>If the priority is qualified enquiries, invest in service clarity, evidence and a strong contact journey. If the priority is direct sales, operational setup and product data deserve more budget than animation.</p><h3>Compare like with like</h3><p>Ask each supplier to separate the must-have launch from optional phases. A smaller first release with measurement is often safer than a large fixed wishlist.</p>","Buyer’s guide","/static/images/digital-strategy.png"),
        ("blog","seo-locale-piccole-imprese","Local SEO for Italian small businesses","A durable checklist for helping nearby customers understand your offer, location and credibility.","<p class=\"lead\">Local SEO is mostly consistency and evidence. Search engines need to connect a real business, a real place, a specific service and signals from customers.</p><h2>Start with your Google Business Profile</h2><p>Use the precise business name, correct primary category, current hours, service area and a page on your own domain that confirms the same facts. Keep ownership in a company-controlled account.</p><h2>Build useful location evidence</h2><ul><li>Publish a complete contact page with accessible directions</li><li>Create service pages only where the offer is genuinely distinct</li><li>Add original project examples from the areas you serve</li><li>Ask for honest reviews without scripts or incentives</li><li>Reply to reviews with useful, privacy-respecting detail</li></ul><h2>Measure actions, not rankings alone</h2><p>Track qualified calls, forms, bookings and direction requests. Ranking reports are diagnostic; business outcomes decide what deserves further work.</p><h3>Avoid doorway pages</h3><p>Do not clone the same paragraph for dozens of towns. A location page should contain unique evidence, logistics and work relevant to that place.</p>","Local search guide","/static/images/digital-strategy.png"),
        ("blog","sito-veloce-piu-clienti","Why a faster website wins more enquiries","A practical performance guide focused on mobile visitors, measurable experience and maintainable fixes.","<p class=\"lead\">Speed is part of trust. A visitor comparing local providers on a mobile connection should not wait for oversized imagery, trackers and decorative scripts.</p><h2>Measure real pages</h2><p>Use field data when available and test the pages customers actually enter. Core Web Vitals are useful signals, but conversion, accessibility and content clarity remain part of the same experience.</p><h2>Fix the largest causes first</h2><ol><li>Resize and compress images for their rendered dimensions.</li><li>Load only the fonts and scripts the page needs.</li><li>Cache static assets and serve them close to visitors.</li><li>Reserve image dimensions to prevent layout movement.</li><li>Keep third-party widgets behind deliberate interaction.</li></ol><h2>Set a performance budget</h2><p>Agree limits for page weight, JavaScript and largest image before design approval. Test on mid-range hardware and a constrained network, not only a developer laptop.</p><h3>Performance is maintenance</h3><p>New campaigns, plugins and embeds can undo earlier work. Include representative page tests in every release.</p>","Performance guide","/static/images/digital-strategy.png"),
        ("tech","rust-actix","Rust and Actix Web","A secure, efficient foundation for server-rendered products, integrations and APIs that need predictable behaviour.","<p class=\"lead\">We choose Rust when reliability, resource efficiency and long-term maintainability justify a strongly typed systems language.</p><h2>Why Rust</h2><p>Rust prevents broad classes of memory errors at compile time and makes ownership and concurrency explicit. That does not make software automatically secure, but it gives engineering teams a strong foundation.</p><h2>Where Actix Web fits</h2><p>Actix Web provides routing, middleware, request extraction and asynchronous handling for web services. In this project it renders crawlable pages on the server and exposes bounded operational endpoints.</p><div class=\"fact-grid\"><p><strong>Good fit</strong><br>APIs, portals and dependable back ends</p><p><strong>Trade-off</strong><br>Higher learning curve than scripting stacks</p></div><h2>Operational discipline still matters</h2><ul><li>Pin and audit dependencies</li><li>Validate every external input</li><li>Use timeouts, structured logs and health checks</li><li>Keep secrets outside source control</li><li>Test database and integration failure paths</li></ul><h3>Technology follows the problem</h3><p>For a simple marketing site, a managed platform may be the better value. We recommend custom Rust only where its properties serve a real requirement.</p>","Engineering note","/static/images/digital-strategy.png"),
        ("tech","mongodb","MongoDB for evolving content and operations","A flexible document database used with explicit validation, indexes and ownership—not as an excuse to skip data design.","<p class=\"lead\">MongoDB stores related data as documents, which can suit content, catalogues and operational records that evolve together.</p><h2>How it is used here</h2><p>Published content, admin users, enquiries and site settings live in separate collections. Application structs validate expected fields, while indexes and bounded queries keep access predictable.</p><div class=\"fact-grid\"><p><strong>Strength</strong><br>Flexible document-shaped data</p><p><strong>Responsibility</strong><br>Schema and index discipline</p></div><h2>Production safeguards</h2><ul><li>Unique indexes for identifiers such as slugs and email</li><li>Least-privilege database credentials</li><li>Encrypted backups with restoration tests</li><li>Retention rules for personal information</li><li>Monitoring for slow queries and connection pressure</li></ul><h2>When we would choose differently</h2><p>Highly relational workflows, complex cross-entity reporting or strict multi-row transactions may point to PostgreSQL. Database choice is an architectural decision, not a brand preference.</p>","Data architecture","/static/images/digital-strategy.png"),
        ("about","our-approach","A global developer community for Italy","Italy Developers brings developers from around the world together for one focused mission: helping Italy’s local businesses and individuals use technology well.","<p class=\"lead\">Italy Developers is a worldwide developer community with an Italy-only service mission. Contributors can join from any country, but the community’s commercial work is dedicated exclusively to people and businesses in Italy.</p><h2>Why this community exists</h2><p>Local shops, professionals, associations, creators and small companies often need practical technology without enterprise complexity. We connect capable developers around that need and build focused websites, applications, APIs, dashboards and responsible AI tools.</p><div class=\"fact-grid\"><p><strong>Community</strong><br>Open to developers worldwide</p><p><strong>Client mission</strong><br>Exclusively for Italy</p></div><h2>Who can join</h2><p>Developers, designers, testers, technical writers, DevOps practitioners and responsible AI specialists from anywhere in the world can contribute. What matters is useful skill, honest communication and respect for the community’s Italy-focused mission.</p><h2>Who we build for</h2><ul><li>Italian local and family businesses</li><li>Independent professionals and creators in Italy</li><li>Italian community organisations and practical local initiatives</li><li>Individuals in Italy who need a useful digital product</li></ul><p>We do not take commercial work for businesses or individuals outside Italy. Worldwide participation strengthens the team; Italy remains the sole beneficiary market.</p><h2>How we work</h2><p>Projects use clear scope, visible milestones, accessible interfaces, owner-controlled data and technology the client can realistically maintain. We never invent portfolio results or promise capabilities the team cannot deliver.</p>","Our mission","/static/images/small-business-websites.png"),
        ("about","join-the-community","Join Italy Developers","Developers anywhere in the world can contribute their skills to technology that serves Italy’s local businesses and individuals.","<p class=\"lead\">You do not need to live in Italy to join Italy Developers. You do need to support the mission: practical, responsible technology for people and small organisations in Italy.</p><h2>Ways to contribute</h2><ul><li>Rust, Python, Django and API engineering</li><li>Accessible frontend and responsive interface work</li><li>Testing, security review and deployment</li><li>Content, documentation and Italian localisation</li><li>Responsible AI evaluation and product safeguards</li></ul><h2>What members can expect</h2><p>Real portfolio problems, reviewable work, clear ownership and opportunities to learn across technologies. Contributions should be documented and respectful of client privacy.</p><h2>Start a conversation</h2><p>Use the contact page and mention your skills, time zone, preferred technologies and the kind of Italy-focused project you want to support.</p>","Worldwide membership","/static/images/digital-strategy.png")
    ];
    let docs = data
        .into_iter()
        .enumerate()
        .map(
            |(i, (kind, slug, title, summary, body, eyebrow, _image))| ContentItem {
                id: None,
                kind: kind.into(),
                slug: slug.into(),
                lang: "en".into(),
                title: title.into(),
                eyebrow: eyebrow.into(),
                summary: summary.into(),
                glance: String::new(),
                body: body.into(),
                image: format!("/media/covers/{kind}/{slug}.svg"),
                image_alt: format!("Editorial image for {}", title),
                seo_title: format!("{} | Italy Developers", title),
                seo_description: summary.into(),
                keywords: "piccole imprese Italia, sito web economico, sviluppo web Italia".into(),
                cta: "Request a practical proposal".into(),
                link: String::new(),
                featured: i < 8,
                published: true,
                order: i as i32,
                created_at: now,
                updated_at: now,
            },
        )
        .collect::<Vec<_>>();
    for item in docs {
        content(db).replace_one(doc! {"kind":&item.kind,"slug":&item.slug}, item).upsert(true).await?;
    }
    migrations.insert_one(doc! {"key":"editorial-v2","applied_at":now}).await?;
    apply_editorial_v3(db, now).await?;
    apply_service_v8(db, now).await?;
    apply_project_proof_v9(db, now).await?;
    migrations.insert_one(doc! {"key":"editorial-v3","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v4","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v5","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v6","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v7","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v8","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v9","applied_at":now}).await?;
    apply_real_photos_v10(db).await?;
    migrations.insert_one(doc! {"key":"real-photos-v10","applied_at":now}).await?;
    apply_market_positioning_v11(db).await?;
    migrations.insert_one(doc! {"key":"market-positioning-v11","applied_at":now}).await?;
    apply_image_consistency_and_insights_v12(db, now).await?;
    migrations.insert_one(doc! {"key":"image-consistency-insights-v12","applied_at":now}).await?;
    apply_unique_glance_v13(db).await?;
    migrations.insert_one(doc! {"key":"unique-glance-v13","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"testimonial-links-v14","applied_at":now}).await?;
    Ok(())
}

async fn apply_real_photos_v10(db: &Database) -> Result<(), AppError> {
    let assignments = [
        ("service","custom-business-software","/static/images/generated/service-custom-software.webp"),
        ("service","apps-digital-products","/static/images/generated/service-apps-products.webp"),
        ("service","ai-chat-automation","/static/images/generated/service-ai-support.webp"),
        ("service","modernisation-rescue-support","/static/images/generated/service-product-rescue.webp"),
        ("blog","rust-actix-production-checklist","/static/images/generated/blog-rust-actix.webp"),
        ("blog","designing-secure-cms","/static/images/generated/blog-secure-cms.webp"),
        ("blog","mongodb-content-modeling","/static/images/generated/blog-mongodb-modeling.webp"),
        ("blog","django-rest-framework-dynamic-serializers","/static/images/generated/blog-drf-serializers.webp"),
        ("blog","nested-comments-data-model","/static/images/generated/blog-nested-comments.webp"),
        ("blog","docker-compose-production-overrides","/static/images/generated/blog-docker-compose.webp"),
        ("blog","server-rendered-seo-basics","/static/images/generated/blog-server-seo.webp"),
        ("blog","accessible-admin-forms","/static/images/generated/blog-accessible-forms.webp"),
        ("blog","api-ready-python-dashboard","/static/images/generated/blog-python-dashboard.webp"),
        ("blog","small-business-website-scope","/static/images/generated/blog-website-scope.webp"),
        ("work","doappointment-platform","/static/images/generated/work-doappointment.webp"),
        ("work","learning-management-system","/static/images/generated/work-lms.webp"),
        ("work","ai-chat-support","/static/images/generated/work-ai-chat.webp"),
        ("work","music-application","/static/images/generated/work-music-app.webp"),
        ("work","coinprofit-plus","/static/images/generated/work-coinprofit.webp"),
        ("work","gaming-platform","/static/images/generated/work-gaming.webp"),
        ("work","car-parking-system","/static/images/generated/work-carparking.webp"),
        ("work","pet-care-ai-upcoming","/static/images/generated/work-pet-care-ai.webp"),
        ("tech","rust-actix","/static/images/generated/tech-rust.webp"),
        ("tech","mongodb","/static/images/generated/tech-mongodb.webp"),
        ("tech","python-django-drf","/static/images/generated/tech-python.webp"),
        ("tech","docker-deployment","/static/images/generated/tech-docker.webp"),
        ("tech","html-css-javascript","/static/images/generated/tech-frontend.webp"),
        ("tech","flet-python","/static/images/generated/tech-flet.webp"),
        ("tech","git-github-ci","/static/images/generated/tech-git.webp"),
        ("about","our-approach","/static/images/generated/about-community.webp"),
        ("about","join-the-community","/static/images/generated/about-join.webp"),
    ];
    for (kind, slug, image) in assignments {
        content(db).update_one(doc! {"kind":kind,"slug":slug}, doc! {"$set":{"image":image,"updated_at":DateTime::now()}}).await?;
    }
    Ok(())
}

async fn apply_market_positioning_v11(db: &Database) -> Result<(), AppError> {
    let now = DateTime::now();
    let images = [
        ("work","italy-developers-cms","/static/images/generated/work-rust-cms.webp"),
        ("work","storemate-crm-inventory","/static/images/generated/work-storemate.webp"),
        ("insight","quanto-costa-un-sito-web-in-italia","/static/images/generated/insight-website-cost.webp"),
        ("testimonial","doappointment-proof","/static/images/generated/proof-doappointment.webp"),
        ("testimonial","pet-care-proof","/static/images/generated/proof-pet-care.webp"),
    ];
    for (kind, slug, image) in images {
        content(db).update_one(doc! {"kind":kind,"slug":slug}, doc! {"$set":{"image":image,"updated_at":now}}).await?;
    }
    let tech = [
        ("rust-actix", "Rust full-stack: Actix Web + Askama", "Fast, dependable server-rendered products and APIs built with Rust, Actix Web and type-safe Askama templates.", "<p class=\"lead\">We use Rust across the web stack when reliability, security and efficient operation matter.</p><h2>What we build</h2><ul><li>Actix Web APIs, middleware, sessions and integrations</li><li>Askama server-rendered interfaces with strong SEO and low JavaScript overhead</li><li>MongoDB or PostgreSQL persistence</li><li>Secure admin panels, uploads and production Docker delivery</li></ul><p>This website is a working reference for the same stack.</p>"),
        ("python-django-drf", "Python full-stack: Django, FastAPI + GraphQL", "Business applications, admin systems and integrations using Django, DRF, FastAPI, GraphQL and Python automation.", "<p class=\"lead\">Python gives us a productive path from operational workflow to maintainable web product.</p><h2>Capabilities</h2><ul><li>Django full-stack applications and secure admin workflows</li><li>Django REST Framework and FastAPI services</li><li>REST and GraphQL APIs with validation and permissions</li><li>Automation, reporting and AI integration</li><li>PostgreSQL or MongoDB data layers</li></ul><p>Our public DRF Shapeless Serializers package provides inspectable evidence of advanced serializer and API work.</p>"),
        ("mongodb", "MongoDB + PostgreSQL data systems", "Practical document and relational database design, indexing, migrations, backups and application integration.", "<p class=\"lead\">We choose the database around the product rather than forcing every workflow into one model.</p><h2>MongoDB</h2><p>A strong fit for evolving content and document-shaped operational data when schema and index discipline are maintained.</p><h2>PostgreSQL</h2><p>A strong fit for relational workflows, reporting, constraints and transaction-heavy systems.</p><h2>Production basics</h2><p>Least-privilege access, migrations, indexes, encrypted backups, restoration tests and query monitoring are part of delivery.</p>"),
        ("html-css-javascript", "React, Next.js + React Native", "Responsive web and mobile interfaces using React, Next.js, React Native and accessible web foundations.", "<p class=\"lead\">We build interfaces around the user journey, then choose the lightest suitable delivery model.</p><h2>Product interfaces</h2><ul><li>React dashboards and interactive applications</li><li>Next.js websites and full-stack products</li><li>React Native mobile applications</li><li>Semantic HTML, modern CSS, TypeScript and accessible forms</li></ul><p>For content-heavy public pages we may recommend server rendering; for rich workflows we use component-based front ends where they add real value.</p>"),
    ];
    for (slug, title, summary, body) in tech {
        content(db).update_one(doc! {"kind":"tech","slug":slug}, doc! {"$set":{"title":title,"summary":summary,"body":body,"updated_at":now}}).await?;
    }
    content(db).update_one(doc! {"kind":"service","slug":"ai-chat-automation"}, doc! {"$set":{
        "title":"AI-powered support and business solutions",
        "summary":"AI support, knowledge search and workflow automation with self-hosted or managed models, human hand-off and measurable controls.",
        "body":"<p class=\"lead\">We turn AI into a bounded business tool: customer support, internal knowledge search, document workflows and product assistance.</p><h2>What we can deliver</h2><ul><li>AI-enabled support with human hand-off and conversation history</li><li>RAG knowledge assistants grounded in approved business content</li><li>Self-hosted open-source AI when privacy, control or predictable usage justify it</li><li>Managed model integrations when speed and capability are the better trade-off</li><li>Classification, summaries, recommendations and workflow automation</li></ul><h2>Production safeguards</h2><p>Permissions, data boundaries, evaluation, cost controls, observability, feedback and escalation are designed with the feature—not added after launch.</p><h2>Start with a focused pilot</h2><p>We select one high-value workflow, define what success means and ship a reviewable first version before expanding.</p>",
        "updated_at":now
    }}).await?;
    Ok(())
}

/// Two fixes: (1) a project's own proof card should show the same photo as its work item,
/// not an unrelated stock image; (2) "insights" had one lonely article next to ten blog
/// posts, so it adds three more buyer-guide pieces in the same voice.
async fn apply_image_consistency_and_insights_v12(db: &Database, now: DateTime) -> Result<(), AppError> {
    let photo_fixes = [
        ("testimonial", "italy-developers-proof", "/static/images/generated/work-rust-cms.webp"),
        ("testimonial", "storemate-proof", "/static/images/generated/work-storemate.webp"),
        ("service", "apis-integrations-backends", "/static/images/workflow-automation.png"),
    ];
    for (kind, slug, image) in photo_fixes {
        content(db).update_one(doc! {"kind":kind,"slug":slug}, doc! {"$set":{"image":image,"updated_at":now}}).await?;
    }

    let insights = [
        ("come-scegliere-un-partner-web-in-italia","How to choose a website partner in Italy without regret","A practical evaluation framework for comparing freelancers, agencies and platforms before you sign anything.","<p class=\"lead\">The cheapest quote and the most expensive quote can promise the same outcome. What actually predicts a good result is process, ownership and communication—not the logo on the proposal.</p><h2>Ask about ownership before anything else</h2><p>Confirm in writing that you will own your domain, hosting account, source code, content and any third-party accounts opened on your behalf. A partner who resists handing over admin access is a partner you will struggle to leave later.</p><div class=\"fact-grid\"><p><strong>Good sign</strong><br>Access and exports handed over as standard practice</p><p><strong>Warning sign</strong><br>\"We'll manage that for you\" with no handover plan</p></div><h2>Compare process, not just price</h2><ul><li>How is scope defined, and what happens when it changes?</li><li>Who writes the content, and in what language?</li><li>How many working demos will you see before launch?</li><li>What is the plan for accessibility, performance and testing?</li><li>What happens after launch—is support scoped or open-ended?</li></ul><h2>Read the portfolio like a customer, not a spectator</h2><p>Open real examples on a phone, on a slow connection if possible. Look for genuine business detail: prices, hours, honest photography, working forms—not just a polished homepage screenshot.</p><h3>A short reference call is worth more than a long deck</h3><p>Ask a past client one direct question: would they hire this partner again, and why or why not. The answer usually tells you more than the pitch does.</p>","Buyer’s guide"),
        ("gdpr-cookie-base-sito-piccola-impresa","GDPR, cookies and privacy basics every small-business website needs","A practical starting checklist for privacy-aware small-business websites in Italy—not a substitute for qualified legal advice.","<p class=\"lead\">Most small-business websites collect more personal data than their owners realise: contact forms, analytics, embedded maps, booking widgets. Getting the basics right is achievable without a legal department.</p><h2>Start with what you actually collect</h2><p>List every form, cookie, script and third-party embed on the site, and what personal data each one touches. You cannot write an honest privacy notice, or configure consent correctly, without this inventory.</p><div class=\"fact-grid\"><p><strong>Typical sources</strong><br>Contact forms, analytics, maps, booking widgets, chat tools</p><p><strong>First deliverable</strong><br>A plain-language data inventory</p></div><h2>Practical defaults we build in</h2><ul><li>No non-essential cookies fire before consent is given</li><li>A clear, specific privacy notice at the point of data collection</li><li>Data minimisation—collect only what the process actually needs</li><li>A defined retention period instead of storing enquiries indefinitely</li><li>HTTPS everywhere and secure handling of stored submissions</li></ul><h2>Where a developer's job ends</h2><p>We can implement consent management, minimise data collection and document what the site does with personal information. Whether that implementation satisfies your specific legal obligations is a question for a qualified privacy adviser, particularly if you handle sensitive data, operate across borders or work in a regulated sector.</p><h3>Treat this as a foundation, not a certificate</h3><p>A well-built site makes compliance achievable. It does not replace a proper legal review for a business with real regulatory exposure.</p>","Compliance basics"),
        ("noleggiare-o-possedere-il-sito-web","Renting vs owning your website: platform builders compared with a custom build","A clear-eyed comparison of SaaS website builders and a custom build, based on what you actually control.","<p class=\"lead\">A page-builder subscription and a custom-built website can look similar on launch day. The difference shows up eighteen months later, when you need something the platform was never designed to do.</p><h2>What you are really choosing</h2><p>A SaaS builder rents you a platform: fast to start, predictable monthly cost, but your site lives inside someone else's system, with real limits on structure, integrations and data export.</p><p>A custom build costs more upfront and gives you a working product you fully own: your code, your data, your hosting choice, and no forced migration down the line.</p><div class=\"fact-grid\"><p><strong>Builders suit</strong><br>Simple, low-risk sites with standard needs</p><p><strong>Custom suits</strong><br>Sites tied closely to how the business actually operates</p></div><h2>Questions that reveal the right answer</h2><ul><li>Does the business depend on a specific booking, catalogue or workflow logic?</li><li>Will you need integrations the builder's app store does not offer?</li><li>Is switching platforms later an acceptable cost, or a real risk?</li><li>Does page speed and search visibility materially affect revenue?</li><li>Who needs to edit content day-to-day, and how technical are they?</li></ul><h2>A reasonable middle path</h2><p>Some businesses launch lean on a builder, prove demand, then commission a custom build once requirements are real rather than hypothetical. That sequencing is often smarter than guessing upfront.</p><h3>We are honest about the trade-off</h3><p>We build custom products because that is where we add the most value—not because a builder is always the wrong choice for a very simple site.</p>","Decision framework"),
    ];
    for (order, (slug, title, summary, body, eyebrow)) in insights.into_iter().enumerate() {
        let item = ContentItem {
            id: None,
            kind: "insight".into(),
            slug: slug.into(),
            lang: "en".into(),
            title: title.into(),
            eyebrow: eyebrow.into(),
            summary: summary.into(),
            glance: String::new(),
            body: body.into(),
            image: format!("/media/covers/insight/{slug}.svg"),
            image_alt: format!("Editorial cover for {title}"),
            seo_title: format!("{title} | Italy Developers"),
            seo_description: summary.into(),
            keywords: "piccole imprese Italia, guida sito web, privacy sito web, scegliere sviluppatore".into(),
            cta: "Request a practical proposal".into(),
            link: String::new(),
            featured: false,
            published: true,
            order: 10 + order as i32,
            created_at: now,
            updated_at: now,
        };
        content(db).replace_one(doc! {"kind":"insight","slug":slug}, item).upsert(true).await?;
    }
    Ok(())
}

/// One specific "at a glance" sentence per published item — not a template with a
/// swapped-out word. Every one references a concrete fact about that item, and several
/// cross-link the portfolio evidence a claim is based on.
async fn apply_unique_glance_v13(db: &Database) -> Result<(), AppError> {
    let glances: &[(&str, &str, &str)] = &[
        ("about", "our-approach", "Every engagement follows the same rule: worldwide talent, one client market. Nothing in our portfolio is an invented case study."),
        ("about", "join-the-community", "Contributors join from any time zone; every paid project they touch still serves a business or person in Italy."),

        ("blog", "rust-actix-production-checklist", "The exact checklist we run before any Actix Web service goes live behind this domain, not a generic \"best practices\" list."),
        ("blog", "designing-secure-cms", "Written while building this site's own admin panel: role separation, upload hardening and recoverable mistakes."),
        ("blog", "mongodb-content-modeling", "The access-pattern rules we actually used when designing this site's own MongoDB collections."),
        ("blog", "django-rest-framework-dynamic-serializers", "The reasoning behind our open-source drf-shapeless-serializers package, explained in plain language."),
        ("blog", "nested-comments-data-model", "The exact data model powering the comment threads and likes under this very post."),
        ("blog", "docker-compose-production-overrides", "Three Compose override failures we've actually hit in production, and the fix for each."),
        ("blog", "server-rendered-seo-basics", "No plugins, no tricks: the server-rendered fundamentals that let a crawler actually read a page."),
        ("blog", "accessible-admin-forms", "Patterns lifted directly from this site's own content editor, where our team fills in long forms daily."),
        ("blog", "api-ready-python-dashboard", "How we structure a Flet interface so it survives contact with real Django API responses."),
        ("blog", "small-business-website-scope", "The scoping conversation we have with every small-business client before a single page gets designed."),

        ("insight", "quanto-costa-un-sito-web-in-italia", "Five questions to ask before comparing two website quotes side by side, not a price list."),
        ("insight", "come-scegliere-un-partner-web-in-italia", "What to check before you sign, based on the handovers that go wrong when a partnership ends badly."),
        ("insight", "gdpr-cookie-base-sito-piccola-impresa", "A starting checklist, not legal advice, for owners who need the basics right before calling a lawyer."),
        ("insight", "noleggiare-o-possedere-il-sito-web", "The exact questions that separate a business that should rent a builder from one that needs a custom build."),

        ("service", "websites-ecommerce-booking", "Covers everything from a clinic's booking calendar to a producer's checkout: see DoAppointment and JGOB in our portfolio."),
        ("service", "custom-business-software", "The same discipline behind StoreMate's inventory system and the car-parking platform, applied to your operation."),
        ("service", "apps-digital-products", "From the music app to the gaming platform in our portfolio: one process for turning an idea into a released product."),
        ("service", "ai-chat-automation", "Built with the same human hand-off and visible-uncertainty safeguards we designed for the Pet Care AI platform."),
        ("service", "apis-integrations-backends", "The same API discipline behind our published drf-shapeless-serializers package and StoreMate's backend."),
        ("service", "modernisation-rescue-support", "Our first deliverable is always an honest audit: what to keep, what's risky and what a rewrite would actually cost."),

        ("tech", "rust-actix", "Running this website right now — Actix Web is handling the request that loaded this very sentence."),
        ("tech", "python-django-drf", "The stack behind StoreMate, JGOB and our published DRF Shapeless Serializers package."),
        ("tech", "mongodb", "This site's own content, leads and comments live in MongoDB collections designed with the rules explained here."),
        ("tech", "docker-deployment", "The exact multi-stage build and Compose setup this website ships with — see our public repository."),
        ("tech", "html-css-javascript", "Our choice for interactive dashboards and mobile apps, kept separate from server-rendered pages like this one."),
        ("tech", "flet-python", "Powers the Python-native admin tools we build when a team needs a desktop-capable interface without a full web stack."),
        ("tech", "git-github-ci", "The same review and CI discipline that gates every change to this website's own public repository."),

        ("work", "italy-developers-cms", "The site you are reading right now — its full source is public on GitHub, not a mock-up."),
        ("work", "drf-shapeless-serializers", "A real, published PyPI package with documentation on Read the Docs, not a private demo."),
        ("work", "doappointment-platform", "A working booking flow for professionals — the same pattern referenced on our websites-and-booking service page."),
        ("work", "learning-management-system", "Role-based access for learners, teachers and admins — the pattern behind our custom-software service page."),
        ("work", "jgob-commerce-community", "Combines causes, volunteering and Razorpay checkout in one Django platform: proof behind our commerce service page."),
        ("work", "storemate-crm-inventory", "A documented, tested backend with OTP auth and low-stock alerts; the private repository is available on request."),
        ("work", "ai-chat-support", "Conversation interface plus operator hand-off — the architecture referenced on our AI-automation service page."),
        ("work", "music-application", "A media-discovery product showing our approach to library, playback and account-experience design."),
        ("work", "coinprofit-plus", "A dashboard-first product covering account records and administrative control — evidence for our software service page."),
        ("work", "gaming-platform", "Player accounts, progression state and admin content — evidence of interactive-product capability."),
        ("work", "car-parking-system", "Full entry, exit and occupancy workflow — the same operational-systems pattern behind our custom-software page."),
        ("work", "pet-care-ai-upcoming", "Currently in development: probabilistic audio analysis with visible uncertainty, not a finished claim."),
    ];
    for (kind, slug, glance) in glances {
        content(db).update_one(doc! {"kind":*kind,"slug":*slug}, doc! {"$set":{"glance":*glance}}).await?;
    }
    Ok(())
}

async fn apply_editorial_v3(db: &Database, now: DateTime) -> Result<(), AppError> {
    content(db).delete_many(doc! {"kind":"work","slug":{"$in":["osteria-verde","falegnameria-rossi","studio-contabile-luce"]}}).await?;
    content(db).delete_many(doc! {"kind":"service","slug":{"$in":["siti-web-per-piccole-imprese","ecommerce-accessibile","automazione-processi"]}}).await?;
    content(db).delete_many(doc! {"kind":"blog","slug":{"$in":["seo-locale-piccole-imprese","sito-veloce-piu-clienti"]}}).await?;
    let entries = [
        ("service","custom-websites-cms","Custom websites and content-management systems","Fast server-rendered websites with an admin area, uploads, SEO controls, enquiries and deployment included.","<p class=\"lead\">We design and build maintainable business websites where owners can manage services, projects, articles and images without editing code.</p><h2>What we can deliver</h2><ul><li>Responsive public pages and reusable content sections</li><li>Secure role-based admin dashboard</li><li>Image uploads, SEO fields, sitemap and structured data</li><li>Contact and lead-management workflow</li><li>Docker deployment, health checks and persistent storage</li></ul><h2>Proven in this platform</h2><p>This Italy Developers website is the working reference implementation. Explore its public content, CMS architecture and deployment setup in the <a href=\"https://github.com/khajanksj/italy-developers-rust\" target=\"_blank\" rel=\"noopener\">public GitHub repository</a>.</p><h3>Best fit</h3><p>Service businesses, developer portfolios, publications and teams that need custom workflows beyond a page-builder template.</p>","Web platforms","/static/images/small-business-websites.png"),
        ("service","python-django-apis","Python, Django and REST API development","Structured back ends, admin workflows and APIs built with Python, Django and Django REST Framework.","<p class=\"lead\">We build and extend Django applications where clear data models, permissions, validation and maintainable APIs matter.</p><h2>Practical capabilities</h2><ul><li>Django models, admin and business workflows</li><li>Django REST Framework serializers, ViewSets and permissions</li><li>Nested resources, filtering and version-aware response shapes</li><li>API integration with web or Python clients</li><li>Tests, documentation and containerized delivery</li></ul><h2>Public package evidence</h2><p>Our open-source <a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">drf-shapeless-serializers</a> package supports runtime field selection, renaming, conditional fields and deeply nested serializer configuration. Documentation is available on <a href=\"https://drf-shapeless-serializers.readthedocs.io/en/latest/\" target=\"_blank\" rel=\"noopener\">Read the Docs</a>.</p>","Backend and APIs","/static/images/digital-strategy.png"),
        ("service","admin-dashboards-python","Python admin dashboards and internal tools","Responsive operational dashboards, searchable tables, forms and API-ready interfaces built around real team workflows.","<p class=\"lead\">We can build focused internal tools in Python and Flet for teams that need dashboards, forms, searchable records, settings and responsive navigation.</p><h2>What is realistic</h2><ul><li>Dashboard and analytics cards</li><li>Searchable, filterable data tables</li><li>Validated create and edit forms</li><li>Role-aware navigation and settings</li><li>Light and dark themes</li><li>Connection to Django REST APIs</li></ul><h2>How we scope it</h2><p>We begin with the users, decisions and records the tool must support. Complex accounting, regulated decisions or unsupported desktop integrations are not promised without discovery.</p>","Internal tools","/static/images/workflow-automation.png"),
        ("work","italy-developers-cms","Italy Developers — production-ready Rust CMS","A complete public website and MongoDB-backed content platform built with Rust, Actix Web, Askama and Docker.","<p class=\"lead\">This live codebase demonstrates the work we can deliver rather than describing a fictional client result.</p><h2>Implemented features</h2><ul><li>Server-rendered home, collection and detail pages</li><li>Role-aware CMS for services, work, technology, blogs and enquiries</li><li>Validated image uploads and persistent storage</li><li>SEO metadata, Schema.org output, sitemap and robots policy</li><li>CSRF protection, secure sessions, rate limiting and security headers</li><li>Authenticated MongoDB production configuration and health checks</li><li>Nested blog comments and visitor likes</li></ul><h2>Source and verification</h2><p>Review the implementation, Docker setup and release history in the <a href=\"https://github.com/khajanksj/italy-developers-rust\" target=\"_blank\" rel=\"noopener\">Italy Developers Rust repository</a>.</p>","Public project","/static/images/small-business-websites.png"),
        ("work","drf-shapeless-serializers","DRF Shapeless Serializers — open-source Python package","A published Django REST Framework extension for flexible runtime serializer configuration and deeply nested responses.","<p class=\"lead\">The package addresses serializer duplication when different endpoints need different views of the same models.</p><h2>Implemented features</h2><ul><li>Runtime field selection and output-key renaming</li><li>Dynamic field attributes and conditional fields</li><li>Nested serializer configuration at arbitrary depth</li><li>Class-based ViewSet mixin support</li><li>Inline serializers for one-off response shapes</li><li>PyPI packaging and public documentation</li></ul><h2>Project links</h2><p><a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">GitHub source</a> · <a href=\"https://pypi.org/project/drf-shapeless-serializers/\" target=\"_blank\" rel=\"noopener\">PyPI package</a> · <a href=\"https://drf-shapeless-serializers.readthedocs.io/en/latest/\" target=\"_blank\" rel=\"noopener\">Documentation</a></p>","Open-source package","/static/images/digital-strategy.png"),
        ("work","doappointment-platform","DoAppointment — booking and professional discovery platform","A Django-based appointment product with professional profiles, availability, working hours, customer accounts and booking workflows.","<p class=\"lead\">DoAppointment brings service discovery and appointment operations into one product.</p><h2>Implemented product areas</h2><ul><li>Customer and professional account flows</li><li>Professional profiles and service information</li><li>Working-hour and availability management</li><li>Appointment creation and status workflow</li><li>Location and profile detail experiences</li><li>Administrative management through Django</li></ul><h2>What it proves</h2><p>We can build two-sided scheduling products where different user roles manage profiles, time and bookings. Source is private, so no public repository link is presented.</p>","Scheduling product","/static/images/small-business-websites.png"),
        ("work","learning-management-system","Learning Management System","A role-based learning platform for organising courses, learners, teaching content, progress and administration.","<p class=\"lead\">The LMS work covers the core operational structure required to manage learning content and user journeys.</p><h2>Product capabilities</h2><ul><li>Administrator, instructor and learner roles</li><li>Course and lesson organisation</li><li>Enrollment and learner access</li><li>Progress-oriented dashboard views</li><li>Content and account administration</li><li>API-ready architecture for future clients</li></ul><p>This is a private portfolio project; details are intentionally limited to implemented capability and no invented institution or learner metrics are claimed.</p>","Education platform","/static/images/digital-strategy.png"),
        ("work","jgob-commerce-community","JGOB — community, content and commerce platform","A Django/PostgreSQL platform combining organisation content, causes, volunteering, shop, cart, checkout and Razorpay payment flows.","<p class=\"lead\">JGOB demonstrates a multi-section organisation platform rather than a simple brochure website.</p><h2>Implemented features</h2><ul><li>Causes, stories, team and volunteer content</li><li>Product catalogue, product detail and cart</li><li>Checkout and Razorpay integration points</li><li>Django admin and seedable content</li><li>PostgreSQL, Redis and persistent media</li><li>Docker Compose development environment</li></ul><h2>Technology variants</h2><p>The portfolio also includes a Rust/Actix, Askama and MongoDB JGOB implementation with editable content and static Vercel export.</p>","Community commerce","/static/images/lean-ecommerce.png"),
        ("work","storemate-crm-inventory","StoreMate — CRM and inventory operations backend","A Django REST Framework system for authentication, business profiles, products, stock, suppliers, alerts and automated communication.","<p class=\"lead\">StoreMate demonstrates operational API work across identity, inventory and customer communication.</p><h2>Implemented features</h2><ul><li>JWT authentication and email/phone registration</li><li>OTP verification and password recovery</li><li>Profiles and business information</li><li>Inventory, products, categories and suppliers</li><li>Low-stock and security email notifications</li><li>PostgreSQL, Redis, Celery and Firebase integration points</li><li>Browsable API schema and Postman collection</li></ul><p>The repository is private; this page describes verified local implementation without publishing credentials or private source.</p>","CRM and inventory","/static/images/workflow-automation.png"),
        ("work","ai-chat-support","AI-enabled chat and customer support","A support-product capability combining conversation interfaces, structured customer context, operator workflows and AI-assisted responses.","<p class=\"lead\">The work focuses on useful assistance with human control—not an unsupported claim of fully autonomous support.</p><h2>Capability areas</h2><ul><li>Conversation and message interfaces</li><li>Customer context and support history</li><li>AI-assisted drafting and knowledge retrieval</li><li>Operator hand-off and status workflows</li><li>Admin configuration and API integration</li><li>Clear boundaries for privacy and high-impact decisions</li></ul><p>Client and deployment links remain private; the service is offered only after confirming the data source, model cost and escalation workflow.</p>","AI support","/static/images/digital-strategy.png"),
        ("work","music-application","Music application","A media-focused product covering music discovery, playback-oriented interfaces, library organisation and account experiences.","<p class=\"lead\">The music app demonstrates consumer interface and media-product design capability.</p><h2>Product areas</h2><ul><li>Track and collection browsing</li><li>Search and discovery interface</li><li>User library and playlist-oriented flows</li><li>Responsive playback experience</li><li>Account and administration foundations</li></ul><p>This private project is shown as product capability. Licensing, catalogue acquisition and commercial streaming infrastructure are separate business requirements and are not implied.</p>","Media product","/static/images/lean-ecommerce.png"),
        ("work","coinprofit-plus","CoinProfit Plus — finance-oriented dashboard product","A portfolio application centred on account dashboards, financial records, status visibility and administrative control.","<p class=\"lead\">CoinProfit Plus demonstrates data-dense dashboard and workflow implementation.</p><h2>Capability demonstrated</h2><ul><li>Account and profile flows</li><li>Dashboard summaries and record history</li><li>Administrative views and status management</li><li>Responsive data presentation</li><li>Validation and security-aware architecture</li></ul><p>This page does not provide investment advice, promise returns or claim regulated financial services. Public transaction or performance metrics are intentionally not invented.</p>","Dashboard product","/static/images/digital-strategy.png"),
        ("work","gaming-platform","Gaming platform experience","A game-oriented application capability covering player accounts, interactive state, progression views and administrative content.","<p class=\"lead\">The gaming work demonstrates stateful consumer-product interfaces and supporting back-end workflows.</p><h2>Capability areas</h2><ul><li>Player identity and profile experience</li><li>Game state and progress presentation</li><li>Score, reward or leaderboard-ready data models</li><li>Responsive interactive interface</li><li>Administrative content and moderation foundations</li></ul><p>Specific game mechanics and commercial integrations remain private and are scoped case by case.</p>","Interactive product","/static/images/lean-ecommerce.png"),
        ("work","car-parking-system","Car parking management system","A parking workflow product covering spaces, vehicles, entry/exit records, availability and operational administration.","<p class=\"lead\">The car-parking project demonstrates real-world resource and transaction workflow modelling.</p><h2>Product areas</h2><ul><li>Parking-space and zone management</li><li>Vehicle and user records</li><li>Entry, exit and occupancy status</li><li>Operator dashboard and searchable history</li><li>Payment-ready workflow boundaries</li><li>Reports and administrative controls</li></ul><p>Hardware gates, number-plate recognition and payment providers are offered only when the required devices and integrations are confirmed.</p>","Operations system","/static/images/workflow-automation.png"),
        ("work","pet-care-ai-upcoming","Pet Care AI — upcoming behavioural insight platform","An in-development Django platform for pet profiles, audio analysis, probabilistic dog behavioural-state estimates and owner feedback.","<p class=\"lead\"><strong>Upcoming project:</strong> Pet Care AI is being developed as an uncertainty-aware support tool, not an animal-language translator or veterinary diagnosis product.</p><h2>Implemented foundation</h2><ul><li>Email-login accounts and owner-scoped pet profiles</li><li>Private audio uploads and processing records</li><li>Dog audio embeddings with supervised behaviour heads</li><li>Probability distribution, confidence and risk presentation</li><li>Feedback, live progress, GraphQL and health endpoints</li><li>Django, PostgreSQL, Celery and Docker architecture</li></ul><h2>Responsible AI boundary</h2><p>Results describe possible behavioural states and expose uncertainty. Medical concerns must go to a qualified veterinarian. Current model support is dog-specific and licensing limits are documented.</p>","Upcoming · Responsible AI","/static/images/workflow-automation.png"),
        ("tech","rust-actix","Rust and Actix Web","Memory-safe systems development and efficient server-rendered web applications with explicit validation and predictable performance.","<p class=\"lead\">Rust and Actix Web power this website’s routing, middleware, forms, sessions, uploads and operational endpoints.</p><h2>Where we use it</h2><p>Custom web back ends, APIs, content platforms and integrations where correctness and resource efficiency justify a compiled stack.</p><h2>Supporting tools</h2><p>Askama templates, Actix Session, tracing, validation, bcrypt and production Docker builds.</p>","Backend engineering","/static/images/digital-strategy.png"),
        ("tech","python-django-drf","Python, Django and Django REST Framework","Productive back-end development for data-driven applications, admin workflows, APIs and reusable packages.","<p class=\"lead\">Python is our practical choice for business applications, API development, automation and developer tooling.</p><h2>Verified experience</h2><p>The public DRF Shapeless Serializers package demonstrates advanced serializer composition, ViewSet integration, package publishing and documentation.</p><h2>Good fit</h2><p>Operational systems, REST APIs, admin-heavy applications and integrations that benefit from Django’s mature ecosystem.</p>","Python ecosystem","/static/images/digital-strategy.png"),
        ("tech","mongodb","MongoDB","Document-oriented storage for evolving content, operational records, enquiries, comments and application settings.","<p class=\"lead\">MongoDB powers content, users, leads, comments and reactions in this platform.</p><h2>How we use it responsibly</h2><p>Typed application structures, bounded queries, authentication, persistent volumes, indexes, validation and documented backup requirements.</p>","Data layer","/static/images/workflow-automation.png"),
        ("tech","docker-deployment","Docker and Compose","Reproducible multi-stage builds, isolated services, persistent storage, health checks and production configuration.","<p class=\"lead\">We package applications with repeatable Docker builds and explicit runtime configuration.</p><h2>Production practices</h2><ul><li>Non-root read-only application containers</li><li>Persistent database and upload volumes</li><li>Service readiness checks and restart policies</li><li>Secret values kept outside source control</li><li>Bounded logs and documented backup steps</li></ul>","Deployment","/static/images/workflow-automation.png"),
        ("tech","html-css-javascript","HTML, CSS and JavaScript","Accessible semantic interfaces, responsive layouts and focused browser behaviour without unnecessary front-end weight.","<p class=\"lead\">We use standards-based HTML, CSS and JavaScript for fast public pages and maintainable admin interfaces.</p><h2>Capabilities</h2><p>Responsive design, progressive enhancement, accessible forms, content layouts, client-side validation and lightweight navigation.</p>","Frontend fundamentals","/static/images/small-business-websites.png"),
        ("tech","flet-python","Flet for Python interfaces","Material-style Python interfaces for responsive dashboards, forms and internal tools that can target desktop or web.","<p class=\"lead\">Flet lets us build operational interfaces in Python while keeping components, routing, themes and services cleanly separated.</p><h2>Realistic uses</h2><p>Admin dashboards, searchable tables, internal forms, settings, chat-style tools and API-connected business utilities.</p>","Python UI","/static/images/workflow-automation.png"),
        ("tech","git-github-ci","Git, GitHub and CI workflows","Reviewable version control, reproducible checks and traceable delivery for application and package development.","<p class=\"lead\">Source history, focused branches, pull requests and automated compile checks make changes easier to review and recover.</p><h2>Delivery practice</h2><p>We keep secrets out of commits, document release steps and validate production images before deployment.</p>","Engineering workflow","/static/images/digital-strategy.png"),
        ("blog","rust-actix-production-checklist","A production checklist for Rust and Actix Web","The practical checks we use before putting an Actix Web service behind a real domain.","<p class=\"lead\">A release build is only one part of production readiness. Configuration, failure behaviour and ownership matter just as much.</p><h2>Application checks</h2><ul><li>Validate environment variables at startup</li><li>Bound JSON, form and upload sizes</li><li>Use secure, HTTP-only session cookies</li><li>Apply CSRF protection to state-changing forms</li><li>Return separate live and ready health signals</li></ul><h2>Container checks</h2><p>Run as a non-root user, use a read-only filesystem where possible, persist only necessary data and keep database ports off the public host.</p><h2>Operational checks</h2><p>Test restoration, log rotation, dependency updates and a rollback before launch. A health endpoint is not a backup strategy.</p>","Rust deployment","/static/images/digital-strategy.png"),
        ("blog","designing-secure-cms","Designing a secure small-team CMS","How to balance editor convenience with roles, validation, safe uploads and recoverable operations.","<p class=\"lead\">A CMS is a privileged application. It deserves stronger boundaries than the public marketing pages it controls.</p><h2>Separate permissions</h2><p>Editors can draft and update content while publishing, deletion and lead access remain with trusted roles. Sessions should expire and cookies should not be readable by browser scripts.</p><h2>Treat uploads as hostile input</h2><p>Check size, extension and file signatures; generate unpredictable filenames; store outside executable paths; and serve with strict content policies.</p><h2>Make mistakes recoverable</h2><p>Keep database and upload backups together, log important changes and avoid destructive bulk actions without confirmation.</p>","CMS engineering","/static/images/small-business-websites.png"),
        ("blog","mongodb-content-modeling","MongoDB content modelling without losing discipline","A document database is flexible, but useful content systems still need explicit shapes, indexes and migration strategy.","<p class=\"lead\">Flexibility helps content evolve; it should not mean every document has an accidental schema.</p><h2>Model around access patterns</h2><p>Keep content, users, enquiries and comments in separate collections. Query by stable fields such as kind, slug, publication state and creation date.</p><h2>Use application validation</h2><p>Typed Rust structures provide defaults for older documents while admin forms enforce title, slug, summary, SEO and image rules.</p><h2>Migrate intentionally</h2><p>Version editorial seeds and make migrations idempotent. Never overwrite arbitrary editor-created records just because the service restarted.</p>","MongoDB","/static/images/workflow-automation.png"),
        ("blog","django-rest-framework-dynamic-serializers","When dynamic Django REST Framework serializers help","How runtime field selection can reduce duplication without turning API responses into an undocumented free-for-all.","<p class=\"lead\">List, detail, export and permission-aware endpoints often need different representations of the same model.</p><h2>The duplication problem</h2><p>Creating a serializer class for every small variation increases maintenance and makes nested changes repetitive.</p><h2>A controlled dynamic approach</h2><p>Allow known fields, renames, attributes and nested serializers to be configured at runtime while keeping model and permission rules explicit.</p><h2>Use it deliberately</h2><p>Document supported response shapes, test nesting and avoid letting untrusted query parameters expose arbitrary fields. See the implementation in <a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">DRF Shapeless Serializers</a>.</p>","Django REST Framework","/static/images/digital-strategy.png"),
        ("blog","nested-comments-data-model","Building nested comments and likes without a front-end framework","A practical server-rendered model for replies, reactions, validation and progressive enhancement.","<p class=\"lead\">Discussion features do not require a large client application. Standard forms and redirects remain dependable with JavaScript disabled.</p><h2>Store the relationship</h2><p>Each comment keeps a post slug and optional parent identifier. Rendering starts at root comments and walks children with a maximum depth to protect the page.</p><h2>Make likes idempotent</h2><p>Store one reaction per visitor and target, then toggle it. The displayed counter is updated with the reaction so repeated clicks do not create unlimited likes.</p><h2>Protect every write</h2><p>Use CSRF tokens, input length limits, escaped output and rate limiting. Moderation and abuse reporting are the next requirements for an open public launch.</p>","Community features","/static/images/small-business-websites.png"),
        ("blog","docker-compose-production-overrides","Docker Compose overrides that behave in production","Why merged lists, persistent volumes and environment interpolation deserve testing before deployment.","<p class=\"lead\">Compose merges multiple files, but not every field replaces the previous value. Lists such as ports can produce surprising duplicates.</p><h2>Inspect the merged result</h2><p>Run <code>docker compose config</code> with the real file combination. Explicitly override port mappings when a development mapping must disappear.</p><h2>Separate secrets from templates</h2><p>Commit an example environment file, ignore the real one and fail startup when placeholders remain.</p><h2>Respect existing volumes</h2><p>Database initialization variables usually apply only to an empty data directory. Plan authentication migrations instead of deleting data to make a container start.</p>","Docker","/static/images/workflow-automation.png"),
        ("blog","server-rendered-seo-basics","SEO foundations for server-rendered business websites","The technical and editorial basics that make pages discoverable without chasing search-engine tricks.","<p class=\"lead\">Search visibility starts with useful pages that load reliably, answer a clear need and can be understood without executing JavaScript.</p><h2>Give every page one job</h2><p>Use a descriptive title, useful summary, logical headings, internal links and a clear next action. Avoid cloning thin pages for every keyword variation.</p><h2>Ship complete metadata</h2><p>Canonical URLs, descriptions, social images, structured data, sitemap entries and crawl rules should reflect the published content.</p><h2>Measure business actions</h2><p>Track qualified enquiries, bookings or downloads—not ranking screenshots alone. Improve pages with real questions from customers.</p>","Technical SEO","/static/images/small-business-websites.png"),
        ("blog","accessible-admin-forms","Accessible admin forms that editors can trust","Patterns for validation, focus, labels, errors and image descriptions in content-management interfaces.","<p class=\"lead\">Admin accessibility improves accuracy for every editor, especially in long forms with validation and rich content.</p><h2>Keep labels and errors specific</h2><p>Every control needs a persistent label. Put the error beside the field, explain the valid range and preserve entered values after rejection.</p><h2>Support keyboard workflows</h2><p>Use semantic buttons, visible focus, predictable tab order and dialogs that return focus. Do not make drag-and-drop the only upload path.</p><h2>Make content quality visible</h2><p>Character counters, previews and alt-text prompts help editors publish better pages without turning guidelines into guesswork.</p>","Accessibility","/static/images/small-business-websites.png"),
        ("blog","api-ready-python-dashboard","Designing an API-ready Python dashboard","How components, typed models and service boundaries prepare a Flet interface for real Django data.","<p class=\"lead\">A dashboard prototype becomes easier to connect when local demonstration data is already separated from UI components.</p><h2>Split responsibilities</h2><p>Keep routing, theme, components, pages, domain models and data services in separate modules. Pages should request data rather than own transport details.</p><h2>Design loading and failure states</h2><p>An API-connected interface needs empty, loading, validation, authorization and retry behaviour—not only a successful table.</p><h2>Connect through a service layer</h2><p>Map Django REST responses into typed client models. This prevents raw dictionaries and authentication details from leaking through every component.</p>","Python and Flet","/static/images/workflow-automation.png"),
        ("blog","small-business-website-scope","How to scope a useful small-business website","A practical way to choose pages, proof and workflows without promising features the team cannot maintain.","<p class=\"lead\">Begin with the customer decision and the action the business can reliably fulfil.</p><h2>Map the essential questions</h2><p>Who is the service for? What problem does it solve? Where is it available? What evidence builds trust? What happens after contact?</p><h2>Prioritise the working core</h2><p>Launch the strongest service pages, real work, about information and a dependable enquiry path before advanced personalization or automation.</p><h2>Keep ownership clear</h2><p>The business should control its domain, content, accounts and data. Document ongoing costs and choose technology the team can support.</p>","Project planning","/static/images/lean-ecommerce.png")
    ];
    for (order, (kind, slug, title, summary, body, eyebrow, _image)) in entries.into_iter().enumerate() {
        let item = ContentItem { id:None, kind:kind.into(), slug:slug.into(), lang:"en".into(), title:title.into(), eyebrow:eyebrow.into(), summary:summary.into(), glance:String::new(), body:body.into(), image:format!("/media/covers/{kind}/{slug}.svg"), image_alt:format!("Editorial illustration for {title}"), seo_title:format!("{title} | Italy Developers"), seo_description:summary.into(), keywords:"Rust, Python, Django, APIs, CMS, Docker, web development".into(), cta:"Discuss a practical project".into(), link:String::new(), featured:kind == "service" || kind == "work" || (kind == "blog" && order < 18), published:true, order:order as i32, created_at:now, updated_at:now };
        content(db).replace_one(doc! {"kind":kind,"slug":slug,"lang":"en"}, item).upsert(true).await?;
    }
    let seeded: Vec<ContentItem> = content(db).find(doc! {"published":true}).await?.try_collect().await?;
    for item in seeded {
        if let Some(id) = item.id {
            let unique_cover = format!("/media/covers/{}/{}.svg", item.kind, item.slug);
            if item.image.is_empty() || item.image.starts_with("/static/images/") {
                content(db).update_one(doc! {"_id":id}, doc! {"$set":{"image":unique_cover,"image_alt":format!("Editorial illustration for {}",item.title)}}).await?;
            }
        }
    }
    let photo_assignments = [
        ("service", "custom-websites-cms", "/static/images/small-business-websites.png"),
        ("work", "jgob-commerce-community", "/static/images/lean-ecommerce.png"),
        ("work", "pet-care-ai-upcoming", "/static/images/workflow-automation.png"),
        ("work", "drf-shapeless-serializers", "/static/images/digital-strategy.png"),
    ];
    for (kind, slug, image) in photo_assignments {
        content(db).update_one(doc! {"kind":kind,"slug":slug}, doc! {"$set":{"image":image}}).await?;
    }
    Ok(())
}

async fn apply_service_v8(db: &Database, now: DateTime) -> Result<(), AppError> {
    content(db).delete_many(doc! {"kind":"service"}).await?;
    let services = [
        ("websites-ecommerce-booking","Websites, e-commerce and booking platforms","Launch a credible website, sell products, accept bookings and manage content without depending on a developer for every update.","<p class=\"lead\">For an Italian business, professional or creator who needs more than a template: we design the customer journey, build the product and make the everyday management simple.</p><h2>What you can ask us to build</h2><ul><li>Business and professional websites</li><li>Online stores, catalogues, carts and checkout</li><li>Appointment, reservation and availability systems</li><li>Membership, directory and community platforms</li><li>Multilingual content and local-search journeys</li><li>Secure owner dashboard for pages, orders and enquiries</li></ul><h2>Real-life examples</h2><p>A clinic can manage professionals and appointment slots. A local producer can sell online and track orders. A restaurant can publish menus and receive direct reservations. A consultant can qualify leads before a call.</p><h2>What we handle</h2><p>Product planning, interface design, backend, database, payments or booking integrations, admin tools, testing, security, deployment and handover. The first release focuses on the shortest useful path from visitor to customer.</p><h3>Proof in our portfolio</h3><p>DoAppointment, JGOB commerce and this Italy Developers CMS demonstrate scheduling, catalogue, checkout, content and administration capability.</p>","Sell · book · grow","/static/images/small-business-websites.png"),
        ("custom-business-software","Custom business software and internal systems","Replace spreadsheets, disconnected tools and repetitive admin with one system designed around how your organisation actually works.","<p class=\"lead\">When off-the-shelf software forces your team into the wrong workflow, we can build the focused system you actually need.</p><h2>Systems we can deliver</h2><ul><li>CRM and customer-management platforms</li><li>Inventory, supplier and stock operations</li><li>Learning-management and training systems</li><li>Staff dashboards, approvals and reporting</li><li>Parking, property, resource and booking management</li><li>Secure portals for customers, partners or members</li></ul><h2>How a project starts</h2><p>We map users, records, decisions, exceptions and permissions. Then we build the smallest operational version, test it with the people doing the work and expand only where it saves time or improves control.</p><h2>What is included</h2><p>Role-based access, searchable records, validated forms, audit-friendly status changes, notifications, exports, responsive dashboards, API connections and documented deployment.</p><h3>Proof in our portfolio</h3><p>StoreMate, LMS, CoinProfit Plus and the car-parking system show experience with complex records, roles, dashboards and operational workflows.</p>","Operate · organise · control","/media/covers/service/custom-business-software.svg"),
        ("apps-digital-products","Mobile, desktop and cross-platform applications","Turn a product idea into a usable application for customers, staff or a focused community—from prototype to production release.","<p class=\"lead\">We help shape the idea, choose the right platform and build a product people can understand without technical training.</p><h2>Products we can create</h2><ul><li>Consumer and community applications</li><li>Music, media and content experiences</li><li>Gaming and interactive products</li><li>Admin and operational desktop tools</li><li>Customer self-service portals</li><li>API-connected cross-platform interfaces</li></ul><h2>Web, mobile or desktop?</h2><p>We choose based on users, offline needs, distribution, device features and budget. A responsive web app is often the fastest first release; Flet or another cross-platform approach can serve desktop and app-like interfaces where it fits.</p><h2>From idea to release</h2><p>Product definition, user flows, design system, application code, backend APIs, authentication, data, testing, release preparation and ongoing improvement can be handled as one delivery.</p><h3>Proof in our portfolio</h3><p>The music application, gaming work, Flet admin and multiple dashboard products demonstrate consumer and operational interface capability.</p>","Imagine · build · launch","/media/covers/service/apps-digital-products.svg"),
        ("ai-chat-automation","AI features, chat and intelligent automation","Add useful AI to an existing product or build a new AI-assisted workflow with clear human control, privacy boundaries and measurable value.","<p class=\"lead\">We use AI where it can improve a real task—not as decoration and not as a replacement for responsible human decisions.</p><h2>Practical AI use cases</h2><ul><li>AI-assisted customer support and reply drafting</li><li>Knowledge search across approved business information</li><li>Document, enquiry and message classification</li><li>Summaries, recommendations and next-step assistance</li><li>Audio or media analysis with uncertainty shown clearly</li><li>Workflow automation with review and escalation</li></ul><h2>What makes it production-ready</h2><p>Approved data sources, permission checks, prompt and output safeguards, cost controls, logging, feedback, human hand-off and clear statements about what the model cannot decide.</p><h2>We can integrate or build</h2><p>AI can be added to your current CRM, website, support system or internal tool, or delivered as a focused new product with its own dashboard and APIs.</p><h3>Proof in our portfolio</h3><p>AI-enabled support work and the upcoming Pet Care behavioural-insight platform demonstrate conversational and model-backed product architecture.</p>","Assist · automate · improve","/media/covers/service/ai-chat-automation.svg"),
        ("apis-integrations-backends","APIs, integrations and reliable backends","Connect products, payments, data and third-party services through a secure backend that is documented and ready to evolve.","<p class=\"lead\">A dependable backend keeps customer-facing applications, internal tools and external services working as one product.</p><h2>What we build</h2><ul><li>REST and GraphQL APIs</li><li>Authentication, permissions and account flows</li><li>Payments, email, OTP, notifications and background jobs</li><li>Database design, search and reporting endpoints</li><li>Third-party and legacy-system integrations</li><li>Webhooks, scheduled work and operational health checks</li></ul><h2>Technology chosen for the requirement</h2><p>Python, Django and Django REST Framework provide a strong foundation for business systems and integrations. Rust and Actix Web fit services that benefit from strict correctness, efficiency and predictable performance.</p><h2>Delivery quality</h2><p>Input validation, access control, rate limits, structured errors, API documentation, automated tests, Docker deployment and monitoring hooks are part of a serious backend—not optional extras.</p><h3>Proof in our portfolio</h3><p>DRF Shapeless Serializers, StoreMate APIs, Pet Care GraphQL and the Rust CMS demonstrate both framework-level and product-level backend work.</p>","Connect · secure · scale","/static/images/digital-strategy.png"),
        ("modernisation-rescue-support","Product modernisation, rescue and ongoing support","Take over an unfinished, fragile or outdated application, understand what is valuable and move it toward a maintainable release.","<p class=\"lead\">You may already have code, data and users—but no reliable path forward. We can audit the product and improve it without automatically recommending a full rewrite.</p><h2>When this service helps</h2><ul><li>A previous developer or agency is no longer available</li><li>Deployment is unreliable or undocumented</li><li>The interface is difficult on mobile</li><li>Security, permissions or backups are unclear</li><li>New features are slow because the structure is fragile</li><li>A prototype needs production foundations</li></ul><h2>Our first deliverable</h2><p>A technical and product assessment: what works, what is risky, what should be preserved and a phased recovery plan. Critical access, secrets and backups are addressed before cosmetic changes.</p><h2>Possible next phases</h2><p>Bug fixing, UI modernisation, API cleanup, database migration, containerisation, tests, performance work, security hardening, documentation and a controlled production release.</p><h3>No forced rewrite</h3><p>We recommend replacement only when evidence shows that repair would cost more or leave unacceptable risk.</p>","Audit · repair · evolve","/media/covers/service/modernisation-rescue-support.svg")
    ];
    for (order, (slug, title, summary, body, eyebrow, image)) in services.into_iter().enumerate() {
        let item = ContentItem { id:None, kind:"service".into(), slug:slug.into(), lang:"en".into(), title:title.into(), eyebrow:eyebrow.into(), summary:summary.into(), glance:String::new(), body:body.into(), image:image.into(), image_alt:format!("Italy Developers service: {title}"), seo_title:format!("{title} | Italy Developers"), seo_description:summary.into(), keywords:"custom software Italy, application development, AI integration, websites, APIs".into(), cta:"Tell us what you want to build".into(), link:String::new(), featured:true, published:true, order:order as i32, created_at:now, updated_at:now };
        content(db).replace_one(doc! {"kind":"service","slug":slug,"lang":"en"}, item).upsert(true).await?;
    }
    Ok(())
}

struct Translated {
    slug: &'static str,
    lang: &'static str,
    title: &'static str,
    eyebrow: &'static str,
    summary: &'static str,
    glance: &'static str,
    body: &'static str,
    seo_title: &'static str,
    seo_description: &'static str,
    cta: &'static str,
}
/// Inserts translated documents alongside the existing English ones for a
/// `kind`, reusing the English doc's image/order/link/featured. Safe to call
/// repeatedly (upserts on kind+slug+lang).
async fn apply_translations(db: &Database, kind: &str, rows: &[Translated]) -> Result<(), AppError> {
    let english: Vec<ContentItem> = content(db)
        .find(doc! {"kind": kind, "lang": "en"})
        .await?
        .try_collect()
        .await?;
    let now = DateTime::now();
    for row in rows {
        let Some(src) = english.iter().find(|e| e.slug == row.slug) else { continue };
        let item = ContentItem {
            id: None,
            kind: kind.into(),
            slug: row.slug.into(),
            lang: row.lang.into(),
            title: row.title.into(),
            eyebrow: row.eyebrow.into(),
            summary: row.summary.into(),
            glance: row.glance.into(),
            body: row.body.into(),
            image: src.image.clone(),
            image_alt: row.title.into(),
            seo_title: row.seo_title.into(),
            seo_description: row.seo_description.into(),
            keywords: src.keywords.clone(),
            cta: row.cta.into(),
            link: src.link.clone(),
            featured: src.featured,
            published: true,
            order: src.order,
            created_at: now,
            updated_at: now,
        };
        content(db)
            .replace_one(doc! {"kind": kind, "slug": row.slug, "lang": row.lang}, item)
            .upsert(true)
            .await?;
    }
    Ok(())
}

async fn apply_translations_services_v1(db: &Database) -> Result<(), AppError> {
    let rows: Vec<Translated> = vec![
        Translated{slug:"websites-ecommerce-booking",lang:"it",title:"Siti web, e-commerce e piattaforme di prenotazione",eyebrow:"Vendi · prenota · cresci",summary:"Lancia un sito web credibile, vendi prodotti, accetta prenotazioni e gestisci i contenuti senza dipendere da uno sviluppatore per ogni modifica.",glance:"Dal calendario prenotazioni di uno studio medico al checkout di un produttore: guarda DoAppointment e JGOB nel nostro portfolio.",body:"<p class=\"lead\">Per un'azienda, un professionista o un creator italiano che ha bisogno di più di un template: progettiamo il percorso del cliente, costruiamo il prodotto e rendiamo semplice la gestione quotidiana.</p><h2>Cosa puoi chiederci di costruire</h2><ul><li>Siti web aziendali e professionali</li><li>Negozi online, cataloghi, carrelli e checkout</li><li>Sistemi di appuntamenti, prenotazioni e disponibilità</li><li>Piattaforme di membership, directory e community</li><li>Contenuti multilingua e percorsi per la ricerca locale</li><li>Dashboard sicura per pagine, ordini e richieste</li></ul><h2>Esempi reali</h2><p>Uno studio medico può gestire professionisti e slot di appuntamento. Un produttore locale può vendere online e tracciare gli ordini. Un ristorante può pubblicare i menu e ricevere prenotazioni dirette. Un consulente può qualificare i contatti prima di una chiamata.</p><h2>Di cosa ci occupiamo</h2><p>Pianificazione del prodotto, design dell'interfaccia, backend, database, integrazioni di pagamento o prenotazione, strumenti di amministrazione, test, sicurezza, deployment e passaggio di consegne. La prima release si concentra sul percorso più breve e utile dal visitatore al cliente.</p><h3>Prova nel nostro portfolio</h3><p>DoAppointment, il commerce di JGOB e questo CMS di Italy Developers dimostrano capacità di pianificazione, catalogo, checkout, contenuti e amministrazione.</p>",seo_title:"Siti web, e-commerce e piattaforme di prenotazione | Italy Developers",seo_description:"Lancia un sito web credibile, vendi prodotti, accetta prenotazioni e gestisci i contenuti senza dipendere da uno sviluppatore per ogni modifica.",cta:"Dicci cosa vuoi costruire"},
        Translated{slug:"websites-ecommerce-booking",lang:"de",title:"Websites, E-Commerce und Buchungsplattformen",eyebrow:"Verkaufen · buchen · wachsen",summary:"Starten Sie eine glaubwürdige Website, verkaufen Sie Produkte, nehmen Sie Buchungen an und verwalten Sie Inhalte, ohne für jede Änderung einen Entwickler zu brauchen.",glance:"Von der Terminkalender einer Praxis bis zum Checkout eines Herstellers: siehe DoAppointment und JGOB in unserem Portfolio.",body:"<p class=\"lead\">Für ein italienisches Unternehmen, einen Fachmann oder Creator, der mehr als eine Vorlage braucht: Wir gestalten die Customer Journey, bauen das Produkt und machen die tägliche Verwaltung einfach.</p><h2>Was Sie uns bauen lassen können</h2><ul><li>Geschäfts- und Fachwebsites</li><li>Onlineshops, Kataloge, Warenkörbe und Checkout</li><li>Termin-, Reservierungs- und Verfügbarkeitssysteme</li><li>Mitgliedschafts-, Verzeichnis- und Community-Plattformen</li><li>Mehrsprachige Inhalte und lokale Suchpfade</li><li>Sicheres Inhaber-Dashboard für Seiten, Bestellungen und Anfragen</li></ul><h2>Praxisbeispiele</h2><p>Eine Praxis kann Fachkräfte und Terminslots verwalten. Ein lokaler Hersteller kann online verkaufen und Bestellungen verfolgen. Ein Restaurant kann Speisekarten veröffentlichen und direkte Reservierungen erhalten. Ein Berater kann Leads vor einem Anruf qualifizieren.</p><h2>Was wir übernehmen</h2><p>Produktplanung, Interface-Design, Backend, Datenbank, Zahlungs- oder Buchungsintegrationen, Admin-Tools, Tests, Sicherheit, Deployment und Übergabe. Die erste Version konzentriert sich auf den kürzesten nützlichen Weg vom Besucher zum Kunden.</p><h3>Beleg in unserem Portfolio</h3><p>DoAppointment, der JGOB-Commerce und dieses Italy-Developers-CMS zeigen Fähigkeiten in Terminplanung, Katalog, Checkout, Inhalten und Administration.</p>",seo_title:"Websites, E-Commerce und Buchungsplattformen | Italy Developers",seo_description:"Starten Sie eine glaubwürdige Website, verkaufen Sie Produkte, nehmen Sie Buchungen an und verwalten Sie Inhalte, ohne für jede Änderung einen Entwickler zu brauchen.",cta:"Sagen Sie uns, was Sie bauen möchten"},
        Translated{slug:"websites-ecommerce-booking",lang:"fr",title:"Sites web, e-commerce et plateformes de réservation",eyebrow:"Vendre · réserver · grandir",summary:"Lancez un site web crédible, vendez des produits, acceptez des réservations et gérez le contenu sans dépendre d'un développeur pour chaque mise à jour.",glance:"Du calendrier de réservation d'un cabinet au paiement d'un producteur : voyez DoAppointment et JGOB dans notre portfolio.",body:"<p class=\"lead\">Pour une entreprise, un professionnel ou un créateur italien qui a besoin de plus qu'un modèle : nous concevons le parcours client, construisons le produit et simplifions la gestion quotidienne.</p><h2>Ce que vous pouvez nous demander de construire</h2><ul><li>Sites web professionnels et d'entreprise</li><li>Boutiques en ligne, catalogues, paniers et paiement</li><li>Systèmes de rendez-vous, réservation et disponibilité</li><li>Plateformes d'adhésion, d'annuaire et de communauté</li><li>Contenu multilingue et parcours de recherche locale</li><li>Tableau de bord sécurisé pour les pages, commandes et demandes</li></ul><h2>Exemples concrets</h2><p>Un cabinet peut gérer ses praticiens et créneaux de rendez-vous. Un producteur local peut vendre en ligne et suivre les commandes. Un restaurant peut publier ses menus et recevoir des réservations directes. Un consultant peut qualifier ses prospects avant un appel.</p><h2>Ce que nous prenons en charge</h2><p>Planification produit, design d'interface, backend, base de données, intégrations de paiement ou de réservation, outils d'administration, tests, sécurité, déploiement et transfert. La première version se concentre sur le chemin le plus court et utile du visiteur au client.</p><h3>Preuve dans notre portfolio</h3><p>DoAppointment, le commerce JGOB et ce CMS Italy Developers démontrent des capacités de planification, catalogue, paiement, contenu et administration.</p>",seo_title:"Sites web, e-commerce et plateformes de réservation | Italy Developers",seo_description:"Lancez un site web crédible, vendez des produits, acceptez des réservations et gérez le contenu sans dépendre d'un développeur pour chaque mise à jour.",cta:"Dites-nous ce que vous voulez construire"},
        Translated{slug:"websites-ecommerce-booking",lang:"pt",title:"Sites, e-commerce e plataformas de reservas",eyebrow:"Vender · reservar · crescer",summary:"Lance um site confiável, venda produtos, aceite reservas e gerencie conteúdo sem depender de um desenvolvedor a cada atualização.",glance:"Do calendário de reservas de uma clínica ao checkout de um produtor: veja DoAppointment e JGOB no nosso portfólio.",body:"<p class=\"lead\">Para uma empresa, profissional ou criador italiano que precisa de mais do que um template: projetamos a jornada do cliente, construímos o produto e tornamos a gestão diária simples.</p><h2>O que você pode nos pedir para construir</h2><ul><li>Sites empresariais e profissionais</li><li>Lojas online, catálogos, carrinhos e checkout</li><li>Sistemas de agendamento, reserva e disponibilidade</li><li>Plataformas de associação, diretório e comunidade</li><li>Conteúdo multilíngue e jornadas de busca local</li><li>Painel seguro do proprietário para páginas, pedidos e solicitações</li></ul><h2>Exemplos reais</h2><p>Uma clínica pode gerenciar profissionais e horários de consulta. Um produtor local pode vender online e rastrear pedidos. Um restaurante pode publicar cardápios e receber reservas diretas. Um consultor pode qualificar leads antes de uma ligação.</p><h2>O que cuidamos</h2><p>Planejamento de produto, design de interface, backend, banco de dados, integrações de pagamento ou reserva, ferramentas administrativas, testes, segurança, implantação e entrega. O primeiro lançamento foca no caminho mais curto e útil do visitante ao cliente.</p><h3>Prova no nosso portfólio</h3><p>DoAppointment, o comércio da JGOB e este CMS da Italy Developers demonstram capacidade de agendamento, catálogo, checkout, conteúdo e administração.</p>",seo_title:"Sites, e-commerce e plataformas de reservas | Italy Developers",seo_description:"Lance um site confiável, venda produtos, aceite reservas e gerencie conteúdo sem depender de um desenvolvedor a cada atualização.",cta:"Diga-nos o que você quer construir"},

        Translated{slug:"custom-business-software",lang:"it",title:"Software aziendale su misura e sistemi interni",eyebrow:"Operare · organizzare · controllare",summary:"Sostituisci fogli di calcolo, strumenti scollegati e amministrazione ripetitiva con un unico sistema progettato attorno al modo in cui la tua organizzazione lavora davvero.",glance:"La stessa disciplina dietro il sistema di inventario di StoreMate e la piattaforma di parcheggi, applicata alla tua attività.",body:"<p class=\"lead\">Quando un software preconfezionato costringe il tuo team nel flusso di lavoro sbagliato, possiamo costruire il sistema mirato di cui hai davvero bisogno.</p><h2>Sistemi che possiamo realizzare</h2><ul><li>Piattaforme CRM e di gestione clienti</li><li>Operazioni di inventario, fornitori e magazzino</li><li>Sistemi di formazione e gestione dell'apprendimento</li><li>Dashboard per il personale, approvazioni e reportistica</li><li>Gestione di parcheggi, proprietà, risorse e prenotazioni</li><li>Portali sicuri per clienti, partner o membri</li></ul><h2>Come inizia un progetto</h2><p>Mappiamo utenti, record, decisioni, eccezioni e permessi. Poi costruiamo la versione operativa più piccola, la testiamo con le persone che fanno il lavoro e la espandiamo solo dove fa risparmiare tempo o migliora il controllo.</p><h2>Cosa è incluso</h2><p>Accesso basato sui ruoli, record ricercabili, moduli validati, cambi di stato tracciabili, notifiche, esportazioni, dashboard responsive, connessioni API e deployment documentato.</p><h3>Prova nel nostro portfolio</h3><p>StoreMate, LMS, CoinProfit Plus e il sistema di parcheggi mostrano esperienza con record complessi, ruoli, dashboard e flussi operativi.</p>",seo_title:"Software aziendale su misura e sistemi interni | Italy Developers",seo_description:"Sostituisci fogli di calcolo, strumenti scollegati e amministrazione ripetitiva con un unico sistema progettato attorno al modo in cui la tua organizzazione lavora davvero.",cta:"Dicci cosa vuoi costruire"},
        Translated{slug:"custom-business-software",lang:"de",title:"Individuelle Unternehmenssoftware und interne Systeme",eyebrow:"Betreiben · organisieren · kontrollieren",summary:"Ersetzen Sie Tabellenkalkulationen, unverbundene Tools und wiederkehrende Verwaltungsarbeit durch ein System, das um Ihre tatsächliche Arbeitsweise herum gebaut ist.",glance:"Die gleiche Disziplin hinter StoreMates Bestandssystem und der Parkplatzplattform, angewendet auf Ihren Betrieb.",body:"<p class=\"lead\">Wenn Standardsoftware Ihr Team in den falschen Workflow zwingt, bauen wir das fokussierte System, das Sie wirklich brauchen.</p><h2>Systeme, die wir liefern können</h2><ul><li>CRM- und Kundenmanagement-Plattformen</li><li>Bestands-, Lieferanten- und Lageroperationen</li><li>Lernmanagement- und Schulungssysteme</li><li>Mitarbeiter-Dashboards, Genehmigungen und Berichte</li><li>Verwaltung von Parkplätzen, Immobilien, Ressourcen und Buchungen</li><li>Sichere Portale für Kunden, Partner oder Mitglieder</li></ul><h2>Wie ein Projekt beginnt</h2><p>Wir kartieren Nutzer, Datensätze, Entscheidungen, Ausnahmen und Berechtigungen. Dann bauen wir die kleinste funktionsfähige Version, testen sie mit den Menschen, die die Arbeit tatsächlich machen, und erweitern sie nur dort, wo sie Zeit spart oder die Kontrolle verbessert.</p><h2>Was enthalten ist</h2><p>Rollenbasierter Zugriff, durchsuchbare Datensätze, validierte Formulare, nachvollziehbare Statusänderungen, Benachrichtigungen, Exporte, responsive Dashboards, API-Anbindungen und dokumentiertes Deployment.</p><h3>Beleg in unserem Portfolio</h3><p>StoreMate, LMS, CoinProfit Plus und das Parkplatzsystem zeigen Erfahrung mit komplexen Datensätzen, Rollen, Dashboards und operativen Abläufen.</p>",seo_title:"Individuelle Unternehmenssoftware und interne Systeme | Italy Developers",seo_description:"Ersetzen Sie Tabellenkalkulationen, unverbundene Tools und wiederkehrende Verwaltungsarbeit durch ein System, das um Ihre tatsächliche Arbeitsweise herum gebaut ist.",cta:"Sagen Sie uns, was Sie bauen möchten"},
        Translated{slug:"custom-business-software",lang:"fr",title:"Logiciels métier sur mesure et systèmes internes",eyebrow:"Exploiter · organiser · contrôler",summary:"Remplacez les tableurs, les outils déconnectés et l'administration répétitive par un seul système conçu autour de la façon dont votre organisation fonctionne réellement.",glance:"La même rigueur derrière le système d'inventaire de StoreMate et la plateforme de parking, appliquée à votre activité.",body:"<p class=\"lead\">Lorsqu'un logiciel standard force votre équipe dans le mauvais flux de travail, nous pouvons construire le système ciblé dont vous avez vraiment besoin.</p><h2>Systèmes que nous pouvons livrer</h2><ul><li>Plateformes CRM et de gestion clients</li><li>Opérations d'inventaire, fournisseurs et stock</li><li>Systèmes de formation et de gestion de l'apprentissage</li><li>Tableaux de bord du personnel, approbations et rapports</li><li>Gestion de parkings, propriétés, ressources et réservations</li><li>Portails sécurisés pour clients, partenaires ou membres</li></ul><h2>Comment un projet démarre</h2><p>Nous cartographions les utilisateurs, les enregistrements, les décisions, les exceptions et les permissions. Nous construisons ensuite la plus petite version opérationnelle, la testons avec les personnes qui font le travail et ne l'étendons que là où elle fait gagner du temps ou améliore le contrôle.</p><h2>Ce qui est inclus</h2><p>Accès basé sur les rôles, enregistrements consultables, formulaires validés, changements de statut traçables, notifications, exports, tableaux de bord responsives, connexions API et déploiement documenté.</p><h3>Preuve dans notre portfolio</h3><p>StoreMate, LMS, CoinProfit Plus et le système de parking montrent une expérience avec des enregistrements complexes, des rôles, des tableaux de bord et des flux opérationnels.</p>",seo_title:"Logiciels métier sur mesure et systèmes internes | Italy Developers",seo_description:"Remplacez les tableurs, les outils déconnectés et l'administration répétitive par un seul système conçu autour de la façon dont votre organisation fonctionne réellement.",cta:"Dites-nous ce que vous voulez construire"},
        Translated{slug:"custom-business-software",lang:"pt",title:"Software empresarial sob medida e sistemas internos",eyebrow:"Operar · organizar · controlar",summary:"Substitua planilhas, ferramentas desconectadas e administração repetitiva por um único sistema projetado em torno de como sua organização realmente funciona.",glance:"A mesma disciplina por trás do sistema de estoque da StoreMate e da plataforma de estacionamento, aplicada à sua operação.",body:"<p class=\"lead\">Quando um software pronto força sua equipe a um fluxo de trabalho errado, podemos construir o sistema focado de que você realmente precisa.</p><h2>Sistemas que podemos entregar</h2><ul><li>Plataformas de CRM e gestão de clientes</li><li>Operações de estoque, fornecedores e inventário</li><li>Sistemas de gestão de aprendizagem e treinamento</li><li>Painéis para equipe, aprovações e relatórios</li><li>Gestão de estacionamento, propriedades, recursos e reservas</li><li>Portais seguros para clientes, parceiros ou membros</li></ul><h2>Como um projeto começa</h2><p>Mapeamos usuários, registros, decisões, exceções e permissões. Depois construímos a menor versão operacional, testamos com as pessoas que fazem o trabalho e expandimos apenas onde economiza tempo ou melhora o controle.</p><h2>O que está incluído</h2><p>Acesso baseado em funções, registros pesquisáveis, formulários validados, mudanças de status rastreáveis, notificações, exportações, painéis responsivos, conexões de API e implantação documentada.</p><h3>Prova no nosso portfólio</h3><p>StoreMate, LMS, CoinProfit Plus e o sistema de estacionamento mostram experiência com registros complexos, funções, painéis e fluxos operacionais.</p>",seo_title:"Software empresarial sob medida e sistemas internos | Italy Developers",seo_description:"Substitua planilhas, ferramentas desconectadas e administração repetitiva por um único sistema projetado em torno de como sua organização realmente funciona.",cta:"Diga-nos o que você quer construir"},

        Translated{slug:"apps-digital-products",lang:"it",title:"Applicazioni mobili, desktop e multipiattaforma",eyebrow:"Immaginare · costruire · lanciare",summary:"Trasforma un'idea di prodotto in un'applicazione utilizzabile per clienti, personale o una community mirata—dal prototipo al rilascio in produzione.",glance:"Dall'app musicale alla piattaforma di gioco nel nostro portfolio: un solo processo per trasformare un'idea in un prodotto rilasciato.",body:"<p class=\"lead\">Aiutiamo a definire l'idea, scegliere la piattaforma giusta e costruire un prodotto che le persone possano capire senza formazione tecnica.</p><h2>Prodotti che possiamo creare</h2><ul><li>Applicazioni consumer e per community</li><li>Esperienze musicali, multimediali e di contenuto</li><li>Prodotti di gioco e interattivi</li><li>Strumenti desktop amministrativi e operativi</li><li>Portali self-service per i clienti</li><li>Interfacce multipiattaforma connesse via API</li></ul><h2>Web, mobile o desktop?</h2><p>Scegliamo in base a utenti, esigenze offline, distribuzione, funzionalità del dispositivo e budget. Una web app responsive è spesso la prima release più rapida; Flet o un altro approccio multipiattaforma può servire interfacce desktop e app-like dove è adatto.</p><h2>Dall'idea al rilascio</h2><p>Definizione del prodotto, flussi utente, design system, codice applicativo, API backend, autenticazione, dati, test, preparazione al rilascio e miglioramento continuo possono essere gestiti come un'unica consegna.</p><h3>Prova nel nostro portfolio</h3><p>L'applicazione musicale, il lavoro di gaming, l'admin Flet e diversi prodotti dashboard dimostrano capacità di interfaccia consumer e operativa.</p>",seo_title:"Applicazioni mobili, desktop e multipiattaforma | Italy Developers",seo_description:"Trasforma un'idea di prodotto in un'applicazione utilizzabile per clienti, personale o una community mirata—dal prototipo al rilascio in produzione.",cta:"Dicci cosa vuoi costruire"},
        Translated{slug:"apps-digital-products",lang:"de",title:"Mobile, Desktop- und plattformübergreifende Anwendungen",eyebrow:"Erdenken · bauen · starten",summary:"Verwandeln Sie eine Produktidee in eine nutzbare Anwendung für Kunden, Mitarbeiter oder eine fokussierte Community—vom Prototyp zur Produktionsversion.",glance:"Von der Musik-App bis zur Gaming-Plattform in unserem Portfolio: ein Prozess, um eine Idee in ein veröffentlichtes Produkt zu verwandeln.",body:"<p class=\"lead\">Wir helfen, die Idee zu formen, die richtige Plattform zu wählen und ein Produkt zu bauen, das Menschen ohne technische Schulung verstehen können.</p><h2>Produkte, die wir erstellen können</h2><ul><li>Verbraucher- und Community-Anwendungen</li><li>Musik-, Medien- und Content-Erlebnisse</li><li>Gaming- und interaktive Produkte</li><li>Admin- und operative Desktop-Tools</li><li>Kunden-Self-Service-Portale</li><li>API-verbundene plattformübergreifende Interfaces</li></ul><h2>Web, mobil oder Desktop?</h2><p>Wir wählen basierend auf Nutzern, Offline-Bedarf, Distribution, Gerätefunktionen und Budget. Eine responsive Web-App ist oft die schnellste erste Version; Flet oder ein anderer plattformübergreifender Ansatz kann Desktop- und app-ähnliche Interfaces bedienen, wo es passt.</p><h2>Von der Idee zur Veröffentlichung</h2><p>Produktdefinition, Nutzerflüsse, Design-System, Anwendungscode, Backend-APIs, Authentifizierung, Daten, Tests, Release-Vorbereitung und laufende Verbesserung können als eine Lieferung behandelt werden.</p><h3>Beleg in unserem Portfolio</h3><p>Die Musikanwendung, Gaming-Arbeit, Flet-Admin und mehrere Dashboard-Produkte zeigen Fähigkeiten bei Verbraucher- und operativen Interfaces.</p>",seo_title:"Mobile, Desktop- und plattformübergreifende Anwendungen | Italy Developers",seo_description:"Verwandeln Sie eine Produktidee in eine nutzbare Anwendung für Kunden, Mitarbeiter oder eine fokussierte Community—vom Prototyp zur Produktionsversion.",cta:"Sagen Sie uns, was Sie bauen möchten"},
        Translated{slug:"apps-digital-products",lang:"fr",title:"Applications mobiles, de bureau et multiplateformes",eyebrow:"Imaginer · construire · lancer",summary:"Transformez une idée de produit en une application utilisable pour les clients, le personnel ou une communauté ciblée—du prototype à la version de production.",glance:"De l'application musicale à la plateforme de jeu dans notre portfolio : un seul processus pour transformer une idée en produit lancé.",body:"<p class=\"lead\">Nous aidons à façonner l'idée, choisir la bonne plateforme et construire un produit que les gens peuvent comprendre sans formation technique.</p><h2>Produits que nous pouvons créer</h2><ul><li>Applications grand public et communautaires</li><li>Expériences musicales, médias et de contenu</li><li>Produits de jeu et interactifs</li><li>Outils de bureau d'administration et opérationnels</li><li>Portails libre-service pour les clients</li><li>Interfaces multiplateformes connectées par API</li></ul><h2>Web, mobile ou bureau ?</h2><p>Nous choisissons en fonction des utilisateurs, des besoins hors ligne, de la distribution, des fonctionnalités de l'appareil et du budget. Une application web responsive est souvent la première version la plus rapide ; Flet ou une autre approche multiplateforme peut servir des interfaces de bureau et de type application là où cela convient.</p><h2>De l'idée à la sortie</h2><p>Définition du produit, parcours utilisateurs, système de design, code applicatif, API backend, authentification, données, tests, préparation de la sortie et amélioration continue peuvent être traités comme une seule livraison.</p><h3>Preuve dans notre portfolio</h3><p>L'application musicale, le travail de jeu, l'admin Flet et plusieurs produits de tableau de bord démontrent une capacité d'interface grand public et opérationnelle.</p>",seo_title:"Applications mobiles, de bureau et multiplateformes | Italy Developers",seo_description:"Transformez une idée de produit en une application utilisable pour les clients, le personnel ou une communauté ciblée—du prototype à la version de production.",cta:"Dites-nous ce que vous voulez construire"},
        Translated{slug:"apps-digital-products",lang:"pt",title:"Aplicativos móveis, desktop e multiplataforma",eyebrow:"Imaginar · construir · lançar",summary:"Transforme uma ideia de produto em um aplicativo utilizável para clientes, equipe ou uma comunidade focada—do protótipo ao lançamento em produção.",glance:"Do aplicativo de música à plataforma de jogos no nosso portfólio: um processo para transformar uma ideia em produto lançado.",body:"<p class=\"lead\">Ajudamos a moldar a ideia, escolher a plataforma certa e construir um produto que as pessoas entendam sem treinamento técnico.</p><h2>Produtos que podemos criar</h2><ul><li>Aplicativos para consumidores e comunidades</li><li>Experiências de música, mídia e conteúdo</li><li>Produtos de jogos e interativos</li><li>Ferramentas desktop administrativas e operacionais</li><li>Portais de autoatendimento para clientes</li><li>Interfaces multiplataforma conectadas via API</li></ul><h2>Web, mobile ou desktop?</h2><p>Escolhemos com base nos usuários, necessidades offline, distribuição, recursos do dispositivo e orçamento. Um aplicativo web responsivo costuma ser o primeiro lançamento mais rápido; Flet ou outra abordagem multiplataforma pode servir interfaces desktop e do tipo app onde for adequado.</p><h2>Da ideia ao lançamento</h2><p>Definição de produto, fluxos de usuário, sistema de design, código do aplicativo, APIs de backend, autenticação, dados, testes, preparação de lançamento e melhoria contínua podem ser tratados como uma única entrega.</p><h3>Prova no nosso portfólio</h3><p>O aplicativo de música, o trabalho de jogos, o admin em Flet e vários produtos de painel demonstram capacidade de interface para consumidores e operacional.</p>",seo_title:"Aplicativos móveis, desktop e multiplataforma | Italy Developers",seo_description:"Transforme uma ideia de produto em um aplicativo utilizável para clientes, equipe ou uma comunidade focada—do protótipo ao lançamento em produção.",cta:"Diga-nos o que você quer construir"},

        Translated{slug:"ai-chat-automation",lang:"it",title:"Supporto e soluzioni aziendali basate su AI",eyebrow:"Assistere · automatizzare · migliorare",summary:"Supporto AI, ricerca della conoscenza e automazione dei flussi di lavoro con modelli self-hosted o gestiti, passaggio a un operatore umano e controlli misurabili.",glance:"Costruito con le stesse misure di passaggio a un operatore umano e incertezza visibile progettate per la piattaforma Pet Care AI.",body:"<p class=\"lead\">Trasformiamo l'AI in uno strumento aziendale con confini chiari: supporto clienti, ricerca della conoscenza interna, flussi documentali e assistenza al prodotto.</p><h2>Cosa possiamo consegnare</h2><ul><li>Supporto abilitato dall'AI con passaggio a un operatore umano e cronologia delle conversazioni</li><li>Assistenti di conoscenza RAG basati su contenuti aziendali approvati</li><li>AI open-source self-hosted quando privacy, controllo o utilizzo prevedibile lo giustificano</li><li>Integrazioni con modelli gestiti quando velocità e capacità sono il compromesso migliore</li><li>Classificazione, riepiloghi, raccomandazioni e automazione dei flussi di lavoro</li></ul><h2>Misure di sicurezza in produzione</h2><p>Permessi, confini dei dati, valutazione, controllo dei costi, osservabilità, feedback ed escalation sono progettati insieme alla funzionalità, non aggiunti dopo il lancio.</p><h2>Inizia con un pilota mirato</h2><p>Selezioniamo un flusso di lavoro ad alto valore, definiamo cosa significa successo e rilasciamo una prima versione verificabile prima di espandere.</p>",seo_title:"Funzionalità AI, chat e automazione intelligente | Italy Developers",seo_description:"Aggiungi un'AI utile a un prodotto esistente o costruisci un nuovo flusso di lavoro assistito dall'AI con controllo umano chiaro, confini di privacy e valore misurabile.",cta:"Dicci cosa vuoi costruire"},
        Translated{slug:"ai-chat-automation",lang:"de",title:"KI-gestützter Support und Geschäftslösungen",eyebrow:"Unterstützen · automatisieren · verbessern",summary:"KI-Support, Wissenssuche und Workflow-Automatisierung mit selbst gehosteten oder verwalteten Modellen, menschlicher Übergabe und messbaren Kontrollen.",glance:"Gebaut mit denselben Schutzmaßnahmen für menschliche Übergabe und sichtbare Unsicherheit, die wir für die Pet-Care-KI-Plattform entworfen haben.",body:"<p class=\"lead\">Wir verwandeln KI in ein begrenztes Geschäftswerkzeug: Kundensupport, interne Wissenssuche, Dokumenten-Workflows und Produktunterstützung.</p><h2>Was wir liefern können</h2><ul><li>KI-gestützten Support mit menschlicher Übergabe und Gesprächsverlauf</li><li>RAG-Wissensassistenten, verankert in genehmigten Geschäftsinhalten</li><li>Selbst gehostete Open-Source-KI, wenn Datenschutz, Kontrolle oder vorhersehbare Nutzung dies rechtfertigen</li><li>Integrationen mit verwalteten Modellen, wenn Geschwindigkeit und Fähigkeit der bessere Kompromiss sind</li><li>Klassifizierung, Zusammenfassungen, Empfehlungen und Workflow-Automatisierung</li></ul><h2>Schutzmaßnahmen für den Produktivbetrieb</h2><p>Berechtigungen, Datengrenzen, Evaluierung, Kostenkontrolle, Beobachtbarkeit, Feedback und Eskalation werden mit der Funktion entworfen—nicht erst nach dem Start hinzugefügt.</p><h2>Beginnen Sie mit einem fokussierten Pilotprojekt</h2><p>Wir wählen einen wertvollen Workflow aus, definieren, was Erfolg bedeutet, und liefern eine überprüfbare erste Version, bevor wir erweitern.</p>",seo_title:"KI-Funktionen, Chat und intelligente Automatisierung | Italy Developers",seo_description:"Fügen Sie einem bestehenden Produkt nützliche KI hinzu oder bauen Sie einen neuen KI-gestützten Workflow mit klarer menschlicher Kontrolle, Datenschutzgrenzen und messbarem Wert.",cta:"Sagen Sie uns, was Sie bauen möchten"},
        Translated{slug:"ai-chat-automation",lang:"fr",title:"Support et solutions métier basés sur l'IA",eyebrow:"Assister · automatiser · améliorer",summary:"Support IA, recherche de connaissances et automatisation des flux de travail avec des modèles auto-hébergés ou gérés, transfert humain et contrôles mesurables.",glance:"Construit avec les mêmes garanties de transfert humain et d'incertitude visible que nous avons conçues pour la plateforme Pet Care AI.",body:"<p class=\"lead\">Nous transformons l'IA en un outil métier borné : support client, recherche de connaissances internes, flux documentaires et assistance produit.</p><h2>Ce que nous pouvons livrer</h2><ul><li>Support activé par l'IA avec transfert humain et historique des conversations</li><li>Assistants de connaissances RAG ancrés dans du contenu métier approuvé</li><li>IA open-source auto-hébergée quand la confidentialité, le contrôle ou l'usage prévisible le justifient</li><li>Intégrations de modèles gérés quand la rapidité et la capacité sont le meilleur compromis</li><li>Classification, résumés, recommandations et automatisation des flux de travail</li></ul><h2>Garanties en production</h2><p>Permissions, limites des données, évaluation, contrôle des coûts, observabilité, retour d'expérience et escalade sont conçus avec la fonctionnalité—pas ajoutés après le lancement.</p><h2>Commencez par un pilote ciblé</h2><p>Nous sélectionnons un flux de travail à forte valeur, définissons ce que signifie le succès et livrons une première version vérifiable avant d'étendre.</p>",seo_title:"Fonctionnalités IA, chat et automatisation intelligente | Italy Developers",seo_description:"Ajoutez une IA utile à un produit existant ou construisez un nouveau flux de travail assisté par IA avec un contrôle humain clair, des limites de confidentialité et une valeur mesurable.",cta:"Dites-nous ce que vous voulez construire"},
        Translated{slug:"ai-chat-automation",lang:"pt",title:"Suporte e soluções empresariais com IA",eyebrow:"Auxiliar · automatizar · melhorar",summary:"Suporte com IA, busca de conhecimento e automação de fluxos de trabalho com modelos self-hosted ou gerenciados, transferência para humanos e controles mensuráveis.",glance:"Construído com as mesmas proteções de transferência para humanos e incerteza visível que projetamos para a plataforma Pet Care AI.",body:"<p class=\"lead\">Transformamos a IA em uma ferramenta empresarial delimitada: suporte ao cliente, busca de conhecimento interno, fluxos de documentos e assistência de produto.</p><h2>O que podemos entregar</h2><ul><li>Suporte habilitado por IA com transferência para humanos e histórico de conversas</li><li>Assistentes de conhecimento RAG baseados em conteúdo empresarial aprovado</li><li>IA open-source self-hosted quando privacidade, controle ou uso previsível justificam</li><li>Integrações de modelos gerenciados quando velocidade e capacidade são a melhor escolha</li><li>Classificação, resumos, recomendações e automação de fluxos de trabalho</li></ul><h2>Proteções em produção</h2><p>Permissões, limites de dados, avaliação, controle de custos, observabilidade, feedback e escalonamento são projetados junto com o recurso—não adicionados após o lançamento.</p><h2>Comece com um piloto focado</h2><p>Selecionamos um fluxo de trabalho de alto valor, definimos o que significa sucesso e lançamos uma primeira versão revisável antes de expandir.</p>",seo_title:"Recursos de IA, chat e automação inteligente | Italy Developers",seo_description:"Adicione IA útil a um produto existente ou construa um novo fluxo de trabalho assistido por IA com controle humano claro, limites de privacidade e valor mensurável.",cta:"Diga-nos o que você quer construir"},

        Translated{slug:"apis-integrations-backends",lang:"it",title:"API, integrazioni e backend affidabili",eyebrow:"Connettere · proteggere · scalare",summary:"Collega prodotti, pagamenti, dati e servizi di terze parti attraverso un backend sicuro, documentato e pronto a evolversi.",glance:"La stessa disciplina API dietro il nostro pacchetto pubblicato drf-shapeless-serializers e il backend di StoreMate.",body:"<p class=\"lead\">Un backend affidabile mantiene applicazioni rivolte ai clienti, strumenti interni e servizi esterni funzionanti come un unico prodotto.</p><h2>Cosa costruiamo</h2><ul><li>API REST e GraphQL</li><li>Autenticazione, permessi e flussi account</li><li>Pagamenti, email, OTP, notifiche e job in background</li><li>Progettazione database, ricerca ed endpoint di reportistica</li><li>Integrazioni con terze parti e sistemi legacy</li><li>Webhook, lavori pianificati e controlli di stato operativi</li></ul><h2>Tecnologia scelta per il requisito</h2><p>Python, Django e Django REST Framework offrono una solida base per sistemi aziendali e integrazioni. Rust e Actix Web si adattano a servizi che beneficiano di correttezza rigorosa, efficienza e prestazioni prevedibili.</p><h2>Qualità della consegna</h2><p>Validazione degli input, controllo degli accessi, rate limit, errori strutturati, documentazione API, test automatizzati, deployment Docker e hook di monitoraggio fanno parte di un backend serio, non extra opzionali.</p><h3>Prova nel nostro portfolio</h3><p>DRF Shapeless Serializers, le API di StoreMate, la GraphQL di Pet Care e il CMS Rust dimostrano capacità sia a livello di framework che di prodotto.</p>",seo_title:"API, integrazioni e backend affidabili | Italy Developers",seo_description:"Collega prodotti, pagamenti, dati e servizi di terze parti attraverso un backend sicuro, documentato e pronto a evolversi.",cta:"Dicci cosa vuoi costruire"},
        Translated{slug:"apis-integrations-backends",lang:"de",title:"APIs, Integrationen und zuverlässige Backends",eyebrow:"Verbinden · sichern · skalieren",summary:"Verbinden Sie Produkte, Zahlungen, Daten und Drittanbieterdienste über ein sicheres, dokumentiertes und zukunftsfähiges Backend.",glance:"Die gleiche API-Disziplin hinter unserem veröffentlichten Paket drf-shapeless-serializers und dem Backend von StoreMate.",body:"<p class=\"lead\">Ein verlässliches Backend hält kundenorientierte Anwendungen, interne Tools und externe Dienste als ein Produkt funktionsfähig.</p><h2>Was wir bauen</h2><ul><li>REST- und GraphQL-APIs</li><li>Authentifizierung, Berechtigungen und Kontoabläufe</li><li>Zahlungen, E-Mail, OTP, Benachrichtigungen und Hintergrundjobs</li><li>Datenbankdesign, Such- und Berichts-Endpunkte</li><li>Integrationen mit Drittanbietern und Altsystemen</li><li>Webhooks, geplante Aufgaben und operative Health-Checks</li></ul><h2>Für den Bedarf gewählte Technologie</h2><p>Python, Django und Django REST Framework bieten eine solide Grundlage für Geschäftssysteme und Integrationen. Rust und Actix Web passen zu Diensten, die von strikter Korrektheit, Effizienz und vorhersehbarer Leistung profitieren.</p><h2>Lieferqualität</h2><p>Eingabevalidierung, Zugriffskontrolle, Rate Limits, strukturierte Fehler, API-Dokumentation, automatisierte Tests, Docker-Deployment und Monitoring-Hooks sind Teil eines ernsthaften Backends—keine optionalen Extras.</p><h3>Beleg in unserem Portfolio</h3><p>DRF Shapeless Serializers, StoreMate-APIs, Pet-Care-GraphQL und das Rust-CMS zeigen sowohl Framework- als auch Produktebene-Backend-Arbeit.</p>",seo_title:"APIs, Integrationen und zuverlässige Backends | Italy Developers",seo_description:"Verbinden Sie Produkte, Zahlungen, Daten und Drittanbieterdienste über ein sicheres, dokumentiertes und zukunftsfähiges Backend.",cta:"Sagen Sie uns, was Sie bauen möchten"},
        Translated{slug:"apis-integrations-backends",lang:"fr",title:"API, intégrations et backends fiables",eyebrow:"Connecter · sécuriser · faire évoluer",summary:"Connectez produits, paiements, données et services tiers via un backend sécurisé, documenté et prêt à évoluer.",glance:"La même rigueur API derrière notre paquet publié drf-shapeless-serializers et le backend de StoreMate.",body:"<p class=\"lead\">Un backend fiable maintient les applications orientées client, les outils internes et les services externes fonctionnant comme un seul produit.</p><h2>Ce que nous construisons</h2><ul><li>API REST et GraphQL</li><li>Authentification, permissions et flux de compte</li><li>Paiements, e-mail, OTP, notifications et tâches en arrière-plan</li><li>Conception de base de données, recherche et points de terminaison de reporting</li><li>Intégrations avec des tiers et systèmes existants</li><li>Webhooks, tâches planifiées et contrôles de santé opérationnels</li></ul><h2>Technologie choisie pour le besoin</h2><p>Python, Django et Django REST Framework offrent une base solide pour les systèmes métier et les intégrations. Rust et Actix Web conviennent aux services qui bénéficient d'une exactitude stricte, d'efficacité et de performances prévisibles.</p><h2>Qualité de livraison</h2><p>Validation des entrées, contrôle d'accès, limites de débit, erreurs structurées, documentation API, tests automatisés, déploiement Docker et hooks de surveillance font partie d'un backend sérieux—pas des extras optionnels.</p><h3>Preuve dans notre portfolio</h3><p>DRF Shapeless Serializers, les API StoreMate, le GraphQL Pet Care et le CMS Rust démontrent un travail backend au niveau framework et produit.</p>",seo_title:"API, intégrations et backends fiables | Italy Developers",seo_description:"Connectez produits, paiements, données et services tiers via un backend sécurisé, documenté et prêt à évoluer.",cta:"Dites-nous ce que vous voulez construire"},
        Translated{slug:"apis-integrations-backends",lang:"pt",title:"APIs, integrações e backends confiáveis",eyebrow:"Conectar · proteger · escalar",summary:"Conecte produtos, pagamentos, dados e serviços de terceiros por meio de um backend seguro, documentado e pronto para evoluir.",glance:"A mesma disciplina de API por trás do nosso pacote publicado drf-shapeless-serializers e do backend da StoreMate.",body:"<p class=\"lead\">Um backend confiável mantém aplicativos voltados ao cliente, ferramentas internas e serviços externos funcionando como um único produto.</p><h2>O que construímos</h2><ul><li>APIs REST e GraphQL</li><li>Autenticação, permissões e fluxos de conta</li><li>Pagamentos, e-mail, OTP, notificações e tarefas em segundo plano</li><li>Design de banco de dados, busca e endpoints de relatórios</li><li>Integrações com terceiros e sistemas legados</li><li>Webhooks, tarefas agendadas e verificações de saúde operacionais</li></ul><h2>Tecnologia escolhida para a necessidade</h2><p>Python, Django e Django REST Framework oferecem uma base sólida para sistemas empresariais e integrações. Rust e Actix Web se encaixam em serviços que se beneficiam de correção rigorosa, eficiência e desempenho previsível.</p><h2>Qualidade de entrega</h2><p>Validação de entrada, controle de acesso, limites de taxa, erros estruturados, documentação de API, testes automatizados, implantação Docker e ganchos de monitoramento fazem parte de um backend sério—não extras opcionais.</p><h3>Prova no nosso portfólio</h3><p>DRF Shapeless Serializers, as APIs da StoreMate, o GraphQL do Pet Care e o CMS Rust demonstram trabalho de backend em nível de framework e de produto.</p>",seo_title:"APIs, integrações e backends confiáveis | Italy Developers",seo_description:"Conecte produtos, pagamentos, dados e serviços de terceiros por meio de um backend seguro, documentado e pronto para evoluir.",cta:"Diga-nos o que você quer construir"},

        Translated{slug:"modernisation-rescue-support",lang:"it",title:"Modernizzazione, recupero e supporto continuo del prodotto",eyebrow:"Verificare · riparare · evolvere",summary:"Prendi in carico un'applicazione incompleta, fragile o obsoleta, capisci cosa ha valore e portala verso una release mantenibile.",glance:"Il nostro primo output è sempre un audit onesto: cosa mantenere, cosa è rischioso e quanto costerebbe davvero una riscrittura.",body:"<p class=\"lead\">Potresti già avere codice, dati e utenti, ma nessun percorso affidabile in avanti. Possiamo verificare il prodotto e migliorarlo senza raccomandare automaticamente una riscrittura completa.</p><h2>Quando questo servizio aiuta</h2><ul><li>Uno sviluppatore o un'agenzia precedente non è più disponibile</li><li>Il deployment è inaffidabile o non documentato</li><li>L'interfaccia è difficile da usare su mobile</li><li>Sicurezza, permessi o backup non sono chiari</li><li>Le nuove funzionalità sono lente perché la struttura è fragile</li><li>Un prototipo ha bisogno di fondamenta pronte per la produzione</li></ul><h2>Il nostro primo output</h2><p>Una valutazione tecnica e di prodotto: cosa funziona, cosa è rischioso, cosa dovrebbe essere preservato e un piano di recupero graduale. Accessi critici, segreti e backup vengono affrontati prima delle modifiche estetiche.</p><h2>Possibili fasi successive</h2><p>Correzione di bug, modernizzazione dell'interfaccia, pulizia delle API, migrazione del database, containerizzazione, test, lavoro sulle prestazioni, hardening della sicurezza, documentazione e una release in produzione controllata.</p><h3>Nessuna riscrittura forzata</h3><p>Raccomandiamo la sostituzione solo quando le evidenze mostrano che la riparazione costerebbe di più o lascerebbe un rischio inaccettabile.</p>",seo_title:"Modernizzazione, recupero e supporto continuo del prodotto | Italy Developers",seo_description:"Prendi in carico un'applicazione incompleta, fragile o obsoleta, capisci cosa ha valore e portala verso una release mantenibile.",cta:"Dicci cosa vuoi costruire"},
        Translated{slug:"modernisation-rescue-support",lang:"de",title:"Produktmodernisierung, Rettung und laufender Support",eyebrow:"Prüfen · reparieren · weiterentwickeln",summary:"Übernehmen Sie eine unfertige, fragile oder veraltete Anwendung, verstehen Sie, was wertvoll ist, und bringen Sie sie auf einen wartbaren Stand.",glance:"Unser erstes Ergebnis ist immer ein ehrliches Audit: was zu behalten ist, was riskant ist und was eine Neuentwicklung tatsächlich kosten würde.",body:"<p class=\"lead\">Sie haben vielleicht schon Code, Daten und Nutzer—aber keinen verlässlichen Weg nach vorn. Wir können das Produkt prüfen und verbessern, ohne automatisch eine komplette Neuentwicklung zu empfehlen.</p><h2>Wann dieser Service hilft</h2><ul><li>Ein früherer Entwickler oder eine Agentur ist nicht mehr verfügbar</li><li>Das Deployment ist unzuverlässig oder undokumentiert</li><li>Die Oberfläche ist auf Mobilgeräten schwierig zu bedienen</li><li>Sicherheit, Berechtigungen oder Backups sind unklar</li><li>Neue Funktionen sind langsam, weil die Struktur brüchig ist</li><li>Ein Prototyp braucht produktionsreife Grundlagen</li></ul><h2>Unser erstes Ergebnis</h2><p>Eine technische und produktbezogene Bewertung: was funktioniert, was riskant ist, was erhalten werden sollte, und ein stufenweiser Wiederherstellungsplan. Kritische Zugänge, Geheimnisse und Backups werden vor kosmetischen Änderungen angegangen.</p><h2>Mögliche nächste Phasen</h2><p>Fehlerbehebung, UI-Modernisierung, API-Bereinigung, Datenbankmigration, Containerisierung, Tests, Performance-Arbeit, Sicherheitshärtung, Dokumentation und eine kontrollierte Produktivfreigabe.</p><h3>Keine erzwungene Neuentwicklung</h3><p>Wir empfehlen einen Ersatz nur, wenn Belege zeigen, dass eine Reparatur mehr kosten oder ein inakzeptables Risiko hinterlassen würde.</p>",seo_title:"Produktmodernisierung, Rettung und laufender Support | Italy Developers",seo_description:"Übernehmen Sie eine unfertige, fragile oder veraltete Anwendung, verstehen Sie, was wertvoll ist, und bringen Sie sie auf einen wartbaren Stand.",cta:"Sagen Sie uns, was Sie bauen möchten"},
        Translated{slug:"modernisation-rescue-support",lang:"fr",title:"Modernisation, sauvetage et support continu de produit",eyebrow:"Auditer · réparer · faire évoluer",summary:"Reprenez une application inachevée, fragile ou obsolète, comprenez ce qui a de la valeur et faites-la évoluer vers une version maintenable.",glance:"Notre premier livrable est toujours un audit honnête : quoi garder, quoi est risqué et ce que coûterait vraiment une réécriture.",body:"<p class=\"lead\">Vous avez peut-être déjà du code, des données et des utilisateurs—mais aucun chemin fiable à suivre. Nous pouvons auditer le produit et l'améliorer sans recommander automatiquement une réécriture complète.</p><h2>Quand ce service aide</h2><ul><li>Un développeur ou une agence précédente n'est plus disponible</li><li>Le déploiement est peu fiable ou non documenté</li><li>L'interface est difficile sur mobile</li><li>La sécurité, les permissions ou les sauvegardes ne sont pas claires</li><li>Les nouvelles fonctionnalités sont lentes car la structure est fragile</li><li>Un prototype a besoin de fondations de production</li></ul><h2>Notre premier livrable</h2><p>Une évaluation technique et produit : ce qui fonctionne, ce qui est risqué, ce qui doit être préservé, et un plan de récupération par phases. Les accès critiques, secrets et sauvegardes sont traités avant les changements cosmétiques.</p><h2>Phases suivantes possibles</h2><p>Correction de bugs, modernisation de l'interface, nettoyage des API, migration de base de données, conteneurisation, tests, travail de performance, renforcement de la sécurité, documentation et une mise en production contrôlée.</p><h3>Aucune réécriture forcée</h3><p>Nous ne recommandons un remplacement que lorsque les preuves montrent qu'une réparation coûterait plus cher ou laisserait un risque inacceptable.</p>",seo_title:"Modernisation, sauvetage et support continu de produit | Italy Developers",seo_description:"Reprenez une application inachevée, fragile ou obsolète, comprenez ce qui a de la valeur et faites-la évoluer vers une version maintenable.",cta:"Dites-nous ce que vous voulez construire"},
        Translated{slug:"modernisation-rescue-support",lang:"pt",title:"Modernização, resgate e suporte contínuo de produto",eyebrow:"Auditar · reparar · evoluir",summary:"Assuma um aplicativo inacabado, frágil ou desatualizado, entenda o que tem valor e leve-o a uma versão sustentável.",glance:"Nossa primeira entrega é sempre uma auditoria honesta: o que manter, o que é arriscado e quanto uma reescrita realmente custaria.",body:"<p class=\"lead\">Você já pode ter código, dados e usuários—mas nenhum caminho confiável a seguir. Podemos auditar o produto e melhorá-lo sem recomendar automaticamente uma reescrita completa.</p><h2>Quando este serviço ajuda</h2><ul><li>Um desenvolvedor ou agência anterior não está mais disponível</li><li>A implantação é pouco confiável ou não documentada</li><li>A interface é difícil no celular</li><li>Segurança, permissões ou backups não estão claros</li><li>Novos recursos são lentos porque a estrutura é frágil</li><li>Um protótipo precisa de bases prontas para produção</li></ul><h2>Nossa primeira entrega</h2><p>Uma avaliação técnica e de produto: o que funciona, o que é arriscado, o que deve ser preservado e um plano de recuperação em fases. Acessos críticos, segredos e backups são tratados antes de mudanças estéticas.</p><h2>Possíveis próximas fases</h2><p>Correção de bugs, modernização de UI, limpeza de API, migração de banco de dados, containerização, testes, trabalho de performance, hardening de segurança, documentação e um lançamento em produção controlado.</p><h3>Sem reescrita forçada</h3><p>Recomendamos a substituição apenas quando as evidências mostram que o reparo custaria mais ou deixaria um risco inaceitável.</p>",seo_title:"Modernização, resgate e suporte contínuo de produto | Italy Developers",seo_description:"Assuma um aplicativo inacabado, frágil ou desatualizado, entenda o que tem valor e leve-o a uma versão sustentável.",cta:"Diga-nos o que você quer construir"},
    ];
    apply_translations(db, "service", &rows).await
}

async fn apply_translations_work_v1(db: &Database) -> Result<(), AppError> {
    let rows: Vec<Translated> = vec![
        Translated{slug:"italy-developers-cms",lang:"it",title:"Italy Developers — CMS Rust pronto per la produzione",eyebrow:"Progetto pubblico",summary:"Un sito web pubblico completo e una piattaforma di contenuti basata su MongoDB, costruita con Rust, Actix Web, Askama e Docker.",glance:"Il sito che stai leggendo in questo momento — il codice sorgente completo è pubblico su GitHub, non un mock-up.",body:"<p class=\"lead\">Questo codice live dimostra il lavoro che possiamo consegnare, invece di descrivere un risultato cliente immaginario.</p><h2>Funzionalità implementate</h2><ul><li>Pagine home, di collezione e di dettaglio rese lato server</li><li>CMS basato sui ruoli per servizi, lavori, tecnologia, blog e richieste</li><li>Caricamento immagini validato e archiviazione persistente</li><li>Metadati SEO, output Schema.org, sitemap e policy robots</li><li>Protezione CSRF, sessioni sicure, rate limiting e header di sicurezza</li><li>Configurazione MongoDB autenticata per la produzione e health check</li><li>Commenti annidati del blog e like dei visitatori</li></ul><h2>Codice sorgente e verifica</h2><p>Rivedi l'implementazione, la configurazione Docker e la cronologia delle release nel <a href=\"https://github.com/khajanksj/italy-developers-rust\" target=\"_blank\" rel=\"noopener\">repository Italy Developers Rust</a>.</p>",seo_title:"Italy Developers — CMS Rust pronto per la produzione | Italy Developers",seo_description:"Un sito web pubblico completo e una piattaforma di contenuti basata su MongoDB, costruita con Rust, Actix Web, Askama e Docker.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"italy-developers-cms",lang:"de",title:"Italy Developers — produktionsreifes Rust-CMS",eyebrow:"Öffentliches Projekt",summary:"Eine vollständige öffentliche Website und MongoDB-gestützte Content-Plattform, gebaut mit Rust, Actix Web, Askama und Docker.",glance:"Die Website, die Sie gerade lesen — der vollständige Quellcode ist öffentlich auf GitHub, kein Mock-up.",body:"<p class=\"lead\">Diese Live-Codebasis zeigt die Arbeit, die wir liefern können, statt ein fiktives Kundenergebnis zu beschreiben.</p><h2>Implementierte Funktionen</h2><ul><li>Serverseitig gerenderte Start-, Sammlungs- und Detailseiten</li><li>Rollenbasiertes CMS für Leistungen, Arbeiten, Technologie, Blogs und Anfragen</li><li>Validierte Bild-Uploads und persistente Speicherung</li><li>SEO-Metadaten, Schema.org-Ausgabe, Sitemap und Robots-Richtlinie</li><li>CSRF-Schutz, sichere Sitzungen, Rate-Limiting und Sicherheits-Header</li><li>Authentifizierte MongoDB-Produktionskonfiguration und Health-Checks</li><li>Verschachtelte Blog-Kommentare und Besucher-Likes</li></ul><h2>Quellcode und Verifizierung</h2><p>Sehen Sie sich die Implementierung, das Docker-Setup und die Release-Historie im <a href=\"https://github.com/khajanksj/italy-developers-rust\" target=\"_blank\" rel=\"noopener\">Italy-Developers-Rust-Repository</a> an.</p>",seo_title:"Italy Developers — produktionsreifes Rust-CMS | Italy Developers",seo_description:"Eine vollständige öffentliche Website und MongoDB-gestützte Content-Plattform, gebaut mit Rust, Actix Web, Askama und Docker.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"italy-developers-cms",lang:"fr",title:"Italy Developers — CMS Rust prêt pour la production",eyebrow:"Projet public",summary:"Un site web public complet et une plateforme de contenu basée sur MongoDB, construite avec Rust, Actix Web, Askama et Docker.",glance:"Le site que vous lisez actuellement — son code source complet est public sur GitHub, pas une maquette.",body:"<p class=\"lead\">Cette base de code en production démontre le travail que nous pouvons livrer plutôt que de décrire un résultat client fictif.</p><h2>Fonctionnalités implémentées</h2><ul><li>Pages d'accueil, de collection et de détail rendues côté serveur</li><li>CMS basé sur les rôles pour services, travaux, technologie, blogs et demandes</li><li>Téléversement d'images validé et stockage persistant</li><li>Métadonnées SEO, sortie Schema.org, sitemap et politique robots</li><li>Protection CSRF, sessions sécurisées, limitation de débit et en-têtes de sécurité</li><li>Configuration MongoDB authentifiée pour la production et vérifications de santé</li><li>Commentaires de blog imbriqués et likes des visiteurs</li></ul><h2>Code source et vérification</h2><p>Consultez l'implémentation, la configuration Docker et l'historique des versions dans le <a href=\"https://github.com/khajanksj/italy-developers-rust\" target=\"_blank\" rel=\"noopener\">dépôt Italy Developers Rust</a>.</p>",seo_title:"Italy Developers — CMS Rust prêt pour la production | Italy Developers",seo_description:"Un site web public complet et une plateforme de contenu basée sur MongoDB, construite avec Rust, Actix Web, Askama et Docker.",cta:"Discutons d'un projet concret"},
        Translated{slug:"italy-developers-cms",lang:"pt",title:"Italy Developers — CMS Rust pronto para produção",eyebrow:"Projeto público",summary:"Um site público completo e uma plataforma de conteúdo baseada em MongoDB, construída com Rust, Actix Web, Askama e Docker.",glance:"O site que você está lendo agora — o código-fonte completo é público no GitHub, não uma maquete.",body:"<p class=\"lead\">Esta base de código ao vivo demonstra o trabalho que podemos entregar, em vez de descrever um resultado fictício de cliente.</p><h2>Recursos implementados</h2><ul><li>Páginas de início, coleção e detalhe renderizadas no servidor</li><li>CMS baseado em funções para serviços, trabalhos, tecnologia, blogs e solicitações</li><li>Upload de imagens validado e armazenamento persistente</li><li>Metadados de SEO, saída Schema.org, sitemap e política de robots</li><li>Proteção CSRF, sessões seguras, limitação de taxa e cabeçalhos de segurança</li><li>Configuração de produção autenticada do MongoDB e verificações de saúde</li><li>Comentários aninhados de blog e curtidas de visitantes</li></ul><h2>Código-fonte e verificação</h2><p>Revise a implementação, a configuração Docker e o histórico de lançamentos no <a href=\"https://github.com/khajanksj/italy-developers-rust\" target=\"_blank\" rel=\"noopener\">repositório Italy Developers Rust</a>.</p>",seo_title:"Italy Developers — CMS Rust pronto para produção | Italy Developers",seo_description:"Um site público completo e uma plataforma de conteúdo baseada em MongoDB, construída com Rust, Actix Web, Askama e Docker.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"drf-shapeless-serializers",lang:"it",title:"DRF Shapeless Serializers — pacchetto Python open-source",eyebrow:"Pacchetto open-source",summary:"Un'estensione pubblicata per Django REST Framework per la configurazione flessibile dei serializer a runtime e risposte profondamente annidate.",glance:"Un pacchetto PyPI reale e pubblicato con documentazione su Read the Docs, non una demo privata.",body:"<p class=\"lead\">Il pacchetto risolve la duplicazione dei serializer quando endpoint diversi necessitano di viste diverse degli stessi modelli.</p><h2>Funzionalità implementate</h2><ul><li>Selezione dei campi a runtime e rinomina delle chiavi di output</li><li>Attributi di campo dinamici e campi condizionali</li><li>Configurazione dei serializer annidati a profondità arbitraria</li><li>Supporto mixin per ViewSet basati su classi</li><li>Serializer inline per forme di risposta occasionali</li><li>Packaging PyPI e documentazione pubblica</li></ul><h2>Link del progetto</h2><p><a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">Codice sorgente su GitHub</a> · <a href=\"https://pypi.org/project/drf-shapeless-serializers/\" target=\"_blank\" rel=\"noopener\">Pacchetto PyPI</a> · <a href=\"https://drf-shapeless-serializers.readthedocs.io/en/latest/\" target=\"_blank\" rel=\"noopener\">Documentazione</a></p>",seo_title:"DRF Shapeless Serializers — pacchetto Python open-source | Italy Developers",seo_description:"Un'estensione pubblicata per Django REST Framework per la configurazione flessibile dei serializer a runtime e risposte profondamente annidate.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"drf-shapeless-serializers",lang:"de",title:"DRF Shapeless Serializers — Open-Source-Python-Paket",eyebrow:"Open-Source-Paket",summary:"Eine veröffentlichte Django-REST-Framework-Erweiterung für flexible Laufzeit-Serializer-Konfiguration und tief verschachtelte Antworten.",glance:"Ein echtes, veröffentlichtes PyPI-Paket mit Dokumentation auf Read the Docs, keine private Demo.",body:"<p class=\"lead\">Das Paket löst Serializer-Duplizierung, wenn verschiedene Endpunkte unterschiedliche Ansichten derselben Modelle benötigen.</p><h2>Implementierte Funktionen</h2><ul><li>Laufzeit-Feldauswahl und Umbenennung von Ausgabeschlüsseln</li><li>Dynamische Feldattribute und bedingte Felder</li><li>Verschachtelte Serializer-Konfiguration in beliebiger Tiefe</li><li>Unterstützung für klassenbasierte ViewSet-Mixins</li><li>Inline-Serializer für einmalige Antwortformen</li><li>PyPI-Packaging und öffentliche Dokumentation</li></ul><h2>Projekt-Links</h2><p><a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">GitHub-Quellcode</a> · <a href=\"https://pypi.org/project/drf-shapeless-serializers/\" target=\"_blank\" rel=\"noopener\">PyPI-Paket</a> · <a href=\"https://drf-shapeless-serializers.readthedocs.io/en/latest/\" target=\"_blank\" rel=\"noopener\">Dokumentation</a></p>",seo_title:"DRF Shapeless Serializers — Open-Source-Python-Paket | Italy Developers",seo_description:"Eine veröffentlichte Django-REST-Framework-Erweiterung für flexible Laufzeit-Serializer-Konfiguration und tief verschachtelte Antworten.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"drf-shapeless-serializers",lang:"fr",title:"DRF Shapeless Serializers — paquet Python open-source",eyebrow:"Paquet open-source",summary:"Une extension publiée pour Django REST Framework permettant une configuration flexible des sérialiseurs à l'exécution et des réponses profondément imbriquées.",glance:"Un vrai paquet PyPI publié avec documentation sur Read the Docs, pas une démo privée.",body:"<p class=\"lead\">Le paquet résout la duplication des sérialiseurs lorsque différents points de terminaison ont besoin de vues différentes des mêmes modèles.</p><h2>Fonctionnalités implémentées</h2><ul><li>Sélection de champs à l'exécution et renommage des clés de sortie</li><li>Attributs de champ dynamiques et champs conditionnels</li><li>Configuration de sérialiseurs imbriqués à profondeur arbitraire</li><li>Support de mixin ViewSet basé sur les classes</li><li>Sérialiseurs inline pour des formes de réponse ponctuelles</li><li>Packaging PyPI et documentation publique</li></ul><h2>Liens du projet</h2><p><a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">Code source GitHub</a> · <a href=\"https://pypi.org/project/drf-shapeless-serializers/\" target=\"_blank\" rel=\"noopener\">Paquet PyPI</a> · <a href=\"https://drf-shapeless-serializers.readthedocs.io/en/latest/\" target=\"_blank\" rel=\"noopener\">Documentation</a></p>",seo_title:"DRF Shapeless Serializers — paquet Python open-source | Italy Developers",seo_description:"Une extension publiée pour Django REST Framework permettant une configuration flexible des sérialiseurs à l'exécution et des réponses profondément imbriquées.",cta:"Discutons d'un projet concret"},
        Translated{slug:"drf-shapeless-serializers",lang:"pt",title:"DRF Shapeless Serializers — pacote Python open-source",eyebrow:"Pacote open-source",summary:"Uma extensão publicada do Django REST Framework para configuração flexível de serializers em tempo de execução e respostas profundamente aninhadas.",glance:"Um pacote PyPI real e publicado com documentação no Read the Docs, não uma demonstração privada.",body:"<p class=\"lead\">O pacote resolve a duplicação de serializers quando endpoints diferentes precisam de visões diferentes dos mesmos modelos.</p><h2>Recursos implementados</h2><ul><li>Seleção de campos em tempo de execução e renomeação de chaves de saída</li><li>Atributos de campo dinâmicos e campos condicionais</li><li>Configuração de serializers aninhados em profundidade arbitrária</li><li>Suporte a mixin de ViewSet baseado em classes</li><li>Serializers inline para formatos de resposta pontuais</li><li>Empacotamento PyPI e documentação pública</li></ul><h2>Links do projeto</h2><p><a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">Código-fonte no GitHub</a> · <a href=\"https://pypi.org/project/drf-shapeless-serializers/\" target=\"_blank\" rel=\"noopener\">Pacote PyPI</a> · <a href=\"https://drf-shapeless-serializers.readthedocs.io/en/latest/\" target=\"_blank\" rel=\"noopener\">Documentação</a></p>",seo_title:"DRF Shapeless Serializers — pacote Python open-source | Italy Developers",seo_description:"Uma extensão publicada do Django REST Framework para configuração flexível de serializers em tempo de execução e respostas profundamente aninhadas.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"doappointment-platform",lang:"it",title:"DoAppointment — piattaforma di prenotazione e scoperta professionisti",eyebrow:"Prodotto di scheduling",summary:"Un prodotto di appuntamenti basato su Django con profili professionali, disponibilità, orari di lavoro, account cliente e flussi di prenotazione.",glance:"Un flusso di prenotazione funzionante per professionisti — lo stesso schema citato nella nostra pagina di servizio siti-e-prenotazioni.",body:"<p class=\"lead\">DoAppointment unisce la scoperta dei servizi e la gestione degli appuntamenti in un unico prodotto.</p><h2>Aree del prodotto implementate</h2><ul><li>Flussi account per clienti e professionisti</li><li>Profili professionali e informazioni sui servizi</li><li>Gestione orari di lavoro e disponibilità</li><li>Creazione appuntamenti e flusso di stato</li><li>Esperienze di dettaglio posizione e profilo</li><li>Gestione amministrativa tramite Django</li></ul><h2>Cosa dimostra</h2><p>Possiamo costruire prodotti di scheduling a due lati dove ruoli utente diversi gestiscono profili, tempo e prenotazioni. Il codice è privato, quindi non viene presentato un link a repository pubblico.</p>",seo_title:"DoAppointment — piattaforma di prenotazione e scoperta professionisti | Italy Developers",seo_description:"Un prodotto di appuntamenti basato su Django con profili professionali, disponibilità, orari di lavoro, account cliente e flussi di prenotazione.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"doappointment-platform",lang:"de",title:"DoAppointment — Buchungs- und Fachkräfte-Discovery-Plattform",eyebrow:"Terminplanungsprodukt",summary:"Ein Django-basiertes Terminprodukt mit Fachkräfteprofilen, Verfügbarkeit, Arbeitszeiten, Kundenkonten und Buchungsabläufen.",glance:"Ein funktionierender Buchungsablauf für Fachkräfte — dasselbe Muster wie auf unserer Websites-und-Buchung-Service-Seite.",body:"<p class=\"lead\">DoAppointment vereint Dienstleistungssuche und Terminverwaltung in einem Produkt.</p><h2>Implementierte Produktbereiche</h2><ul><li>Kunden- und Fachkräfte-Kontoabläufe</li><li>Fachkräfteprofile und Dienstleistungsinformationen</li><li>Verwaltung von Arbeitszeiten und Verfügbarkeit</li><li>Terminerstellung und Statusworkflow</li><li>Standort- und Profildetailansichten</li><li>Administrative Verwaltung über Django</li></ul><h2>Was es beweist</h2><p>Wir können zweiseitige Terminplanungsprodukte bauen, bei denen verschiedene Nutzerrollen Profile, Zeit und Buchungen verwalten. Der Quellcode ist privat, daher wird kein öffentlicher Repository-Link angegeben.</p>",seo_title:"DoAppointment — Buchungs- und Fachkräfte-Discovery-Plattform | Italy Developers",seo_description:"Ein Django-basiertes Terminprodukt mit Fachkräfteprofilen, Verfügbarkeit, Arbeitszeiten, Kundenkonten und Buchungsabläufen.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"doappointment-platform",lang:"fr",title:"DoAppointment — plateforme de réservation et découverte de professionnels",eyebrow:"Produit de planification",summary:"Un produit de rendez-vous basé sur Django avec profils professionnels, disponibilité, horaires de travail, comptes clients et flux de réservation.",glance:"Un flux de réservation fonctionnel pour les professionnels — le même schéma cité sur notre page de service sites-et-réservation.",body:"<p class=\"lead\">DoAppointment réunit la découverte de services et la gestion des rendez-vous en un seul produit.</p><h2>Domaines de produit implémentés</h2><ul><li>Flux de compte pour clients et professionnels</li><li>Profils professionnels et informations sur les services</li><li>Gestion des horaires de travail et de la disponibilité</li><li>Création de rendez-vous et flux de statut</li><li>Expériences de détail de localisation et de profil</li><li>Gestion administrative via Django</li></ul><h2>Ce que cela prouve</h2><p>Nous pouvons construire des produits de planification bilatéraux où différents rôles d'utilisateurs gèrent profils, temps et réservations. Le code source est privé, aucun lien vers un dépôt public n'est donc présenté.</p>",seo_title:"DoAppointment — plateforme de réservation et découverte de professionnels | Italy Developers",seo_description:"Un produit de rendez-vous basé sur Django avec profils professionnels, disponibilité, horaires de travail, comptes clients et flux de réservation.",cta:"Discutons d'un projet concret"},
        Translated{slug:"doappointment-platform",lang:"pt",title:"DoAppointment — plataforma de reservas e descoberta de profissionais",eyebrow:"Produto de agendamento",summary:"Um produto de agendamento baseado em Django com perfis profissionais, disponibilidade, horários de trabalho, contas de clientes e fluxos de reserva.",glance:"Um fluxo de reserva funcional para profissionais — o mesmo padrão citado na nossa página de serviço de sites e reservas.",body:"<p class=\"lead\">O DoAppointment reúne a descoberta de serviços e a gestão de agendamentos em um único produto.</p><h2>Áreas de produto implementadas</h2><ul><li>Fluxos de conta para clientes e profissionais</li><li>Perfis profissionais e informações de serviço</li><li>Gestão de horários de trabalho e disponibilidade</li><li>Criação de agendamentos e fluxo de status</li><li>Experiências de detalhe de localização e perfil</li><li>Gestão administrativa via Django</li></ul><h2>O que isso prova</h2><p>Podemos construir produtos de agendamento bilaterais onde diferentes funções de usuário gerenciam perfis, tempo e reservas. O código-fonte é privado, portanto nenhum link de repositório público é apresentado.</p>",seo_title:"DoAppointment — plataforma de reservas e descoberta de profissionais | Italy Developers",seo_description:"Um produto de agendamento baseado em Django com perfis profissionais, disponibilidade, horários de trabalho, contas de clientes e fluxos de reserva.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"learning-management-system",lang:"it",title:"Sistema di gestione dell'apprendimento",eyebrow:"Piattaforma educativa",summary:"Una piattaforma di apprendimento basata sui ruoli per organizzare corsi, studenti, contenuti didattici, progressi e amministrazione.",glance:"Accesso basato sui ruoli per studenti, insegnanti e admin — lo schema dietro la nostra pagina di servizio software su misura.",body:"<p class=\"lead\">Il lavoro sull'LMS copre la struttura operativa principale necessaria per gestire contenuti didattici e percorsi utente.</p><h2>Capacità del prodotto</h2><ul><li>Ruoli amministratore, istruttore e studente</li><li>Organizzazione di corsi e lezioni</li><li>Iscrizione e accesso per gli studenti</li><li>Viste dashboard orientate ai progressi</li><li>Amministrazione di contenuti e account</li><li>Architettura pronta per API per futuri clienti</li></ul><p>Questo è un progetto di portfolio privato; i dettagli sono intenzionalmente limitati alle capacità implementate e non vengono dichiarate istituzioni o metriche studente inventate.</p>",seo_title:"Sistema di gestione dell'apprendimento | Italy Developers",seo_description:"Una piattaforma di apprendimento basata sui ruoli per organizzare corsi, studenti, contenuti didattici, progressi e amministrazione.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"learning-management-system",lang:"de",title:"Lernmanagementsystem",eyebrow:"Bildungsplattform",summary:"Eine rollenbasierte Lernplattform zur Organisation von Kursen, Lernenden, Lehrinhalten, Fortschritt und Verwaltung.",glance:"Rollenbasierter Zugriff für Lernende, Lehrer und Admins — das Muster hinter unserer Individuelle-Software-Service-Seite.",body:"<p class=\"lead\">Die LMS-Arbeit deckt die zentrale operative Struktur ab, die zur Verwaltung von Lerninhalten und Nutzerabläufen erforderlich ist.</p><h2>Produktfähigkeiten</h2><ul><li>Administrator-, Lehrer- und Lernenden-Rollen</li><li>Kurs- und Lektionsorganisation</li><li>Einschreibung und Lernendenzugriff</li><li>Fortschrittsorientierte Dashboard-Ansichten</li><li>Inhalts- und Kontoverwaltung</li><li>API-fertige Architektur für zukünftige Kunden</li></ul><p>Dies ist ein privates Portfolioprojekt; Details sind bewusst auf implementierte Fähigkeiten beschränkt, es werden keine erfundenen Institutionen oder Lernenden-Metriken behauptet.</p>",seo_title:"Lernmanagementsystem | Italy Developers",seo_description:"Eine rollenbasierte Lernplattform zur Organisation von Kursen, Lernenden, Lehrinhalten, Fortschritt und Verwaltung.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"learning-management-system",lang:"fr",title:"Système de gestion de l'apprentissage",eyebrow:"Plateforme éducative",summary:"Une plateforme d'apprentissage basée sur les rôles pour organiser cours, apprenants, contenu pédagogique, progression et administration.",glance:"Accès basé sur les rôles pour apprenants, enseignants et admins — le schéma derrière notre page de service logiciels sur mesure.",body:"<p class=\"lead\">Le travail sur le LMS couvre la structure opérationnelle centrale nécessaire pour gérer le contenu pédagogique et les parcours utilisateurs.</p><h2>Capacités du produit</h2><ul><li>Rôles administrateur, formateur et apprenant</li><li>Organisation des cours et des leçons</li><li>Inscription et accès des apprenants</li><li>Vues de tableau de bord orientées progression</li><li>Administration du contenu et des comptes</li><li>Architecture prête pour API pour de futurs clients</li></ul><p>Il s'agit d'un projet de portfolio privé ; les détails sont intentionnellement limités aux capacités implémentées, aucune institution ou métrique d'apprenant inventée n'est revendiquée.</p>",seo_title:"Système de gestion de l'apprentissage | Italy Developers",seo_description:"Une plateforme d'apprentissage basée sur les rôles pour organiser cours, apprenants, contenu pédagogique, progression et administration.",cta:"Discutons d'un projet concret"},
        Translated{slug:"learning-management-system",lang:"pt",title:"Sistema de gestão de aprendizagem",eyebrow:"Plataforma educacional",summary:"Uma plataforma de aprendizagem baseada em funções para organizar cursos, alunos, conteúdo didático, progresso e administração.",glance:"Acesso baseado em funções para alunos, professores e administradores — o padrão por trás da nossa página de serviço de software sob medida.",body:"<p class=\"lead\">O trabalho no LMS cobre a estrutura operacional principal necessária para gerenciar conteúdo de aprendizagem e jornadas de usuário.</p><h2>Capacidades do produto</h2><ul><li>Funções de administrador, instrutor e aluno</li><li>Organização de cursos e aulas</li><li>Inscrição e acesso de alunos</li><li>Visualizações de painel orientadas a progresso</li><li>Administração de conteúdo e contas</li><li>Arquitetura pronta para API para futuros clientes</li></ul><p>Este é um projeto de portfólio privado; os detalhes são intencionalmente limitados às capacidades implementadas e nenhuma instituição ou métrica de aluno inventada é alegada.</p>",seo_title:"Sistema de gestão de aprendizagem | Italy Developers",seo_description:"Uma plataforma de aprendizagem baseada em funções para organizar cursos, alunos, conteúdo didático, progresso e administração.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"jgob-commerce-community",lang:"it",title:"JGOB — piattaforma di community, contenuti e commercio",eyebrow:"Commercio comunitario",summary:"Una piattaforma Django/PostgreSQL che combina contenuti organizzativi, cause, volontariato, negozio, carrello, checkout e flussi di pagamento Razorpay.",glance:"Combina cause, volontariato e checkout Razorpay in un'unica piattaforma Django: la prova dietro la nostra pagina di servizio e-commerce.",body:"<p class=\"lead\">JGOB dimostra una piattaforma organizzativa multi-sezione piuttosto che un semplice sito vetrina.</p><h2>Funzionalità implementate</h2><ul><li>Contenuti su cause, storie, team e volontari</li><li>Catalogo prodotti, dettaglio prodotto e carrello</li><li>Checkout e punti di integrazione Razorpay</li><li>Admin Django e contenuti seminabili</li><li>PostgreSQL, Redis e media persistenti</li><li>Ambiente di sviluppo Docker Compose</li></ul><h2>Varianti tecnologiche</h2><p>Il portfolio include anche un'implementazione JGOB in Rust/Actix, Askama e MongoDB con contenuti modificabili ed export statico Vercel.</p>",seo_title:"JGOB — piattaforma di community, contenuti e commercio | Italy Developers",seo_description:"Una piattaforma Django/PostgreSQL che combina contenuti organizzativi, cause, volontariato, negozio, carrello, checkout e flussi di pagamento Razorpay.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"jgob-commerce-community",lang:"de",title:"JGOB — Community-, Content- und Commerce-Plattform",eyebrow:"Community-Commerce",summary:"Eine Django/PostgreSQL-Plattform, die Organisationsinhalte, Anliegen, Freiwilligenarbeit, Shop, Warenkorb, Checkout und Razorpay-Zahlungsabläufe vereint.",glance:"Vereint Anliegen, Freiwilligenarbeit und Razorpay-Checkout in einer Django-Plattform: Beleg für unsere Commerce-Service-Seite.",body:"<p class=\"lead\">JGOB zeigt eine mehrteilige Organisationsplattform statt einer einfachen Broschüren-Website.</p><h2>Implementierte Funktionen</h2><ul><li>Inhalte zu Anliegen, Geschichten, Team und Freiwilligen</li><li>Produktkatalog, Produktdetails und Warenkorb</li><li>Checkout- und Razorpay-Integrationspunkte</li><li>Django-Admin und befüllbare Inhalte</li><li>PostgreSQL, Redis und persistente Medien</li><li>Docker-Compose-Entwicklungsumgebung</li></ul><h2>Technologie-Varianten</h2><p>Das Portfolio enthält auch eine JGOB-Implementierung in Rust/Actix, Askama und MongoDB mit editierbaren Inhalten und statischem Vercel-Export.</p>",seo_title:"JGOB — Community-, Content- und Commerce-Plattform | Italy Developers",seo_description:"Eine Django/PostgreSQL-Plattform, die Organisationsinhalte, Anliegen, Freiwilligenarbeit, Shop, Warenkorb, Checkout und Razorpay-Zahlungsabläufe vereint.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"jgob-commerce-community",lang:"fr",title:"JGOB — plateforme de communauté, contenu et commerce",eyebrow:"Commerce communautaire",summary:"Une plateforme Django/PostgreSQL combinant contenu organisationnel, causes, bénévolat, boutique, panier, paiement et flux de paiement Razorpay.",glance:"Combine causes, bénévolat et paiement Razorpay dans une seule plateforme Django : preuve derrière notre page de service commerce.",body:"<p class=\"lead\">JGOB démontre une plateforme organisationnelle à sections multiples plutôt qu'un simple site vitrine.</p><h2>Fonctionnalités implémentées</h2><ul><li>Contenu sur les causes, histoires, équipe et bénévoles</li><li>Catalogue de produits, détail produit et panier</li><li>Points d'intégration paiement et Razorpay</li><li>Admin Django et contenu ensemençable</li><li>PostgreSQL, Redis et médias persistants</li><li>Environnement de développement Docker Compose</li></ul><h2>Variantes technologiques</h2><p>Le portfolio inclut également une implémentation JGOB en Rust/Actix, Askama et MongoDB avec contenu modifiable et export statique Vercel.</p>",seo_title:"JGOB — plateforme de communauté, contenu et commerce | Italy Developers",seo_description:"Une plateforme Django/PostgreSQL combinant contenu organisationnel, causes, bénévolat, boutique, panier, paiement et flux de paiement Razorpay.",cta:"Discutons d'un projet concret"},
        Translated{slug:"jgob-commerce-community",lang:"pt",title:"JGOB — plataforma de comunidade, conteúdo e comércio",eyebrow:"Comércio comunitário",summary:"Uma plataforma Django/PostgreSQL combinando conteúdo organizacional, causas, voluntariado, loja, carrinho, checkout e fluxos de pagamento Razorpay.",glance:"Combina causas, voluntariado e checkout Razorpay em uma única plataforma Django: prova por trás da nossa página de serviço de comércio.",body:"<p class=\"lead\">O JGOB demonstra uma plataforma organizacional multi-seção em vez de um simples site institucional.</p><h2>Recursos implementados</h2><ul><li>Conteúdo sobre causas, histórias, equipe e voluntários</li><li>Catálogo de produtos, detalhe de produto e carrinho</li><li>Pontos de integração de checkout e Razorpay</li><li>Admin Django e conteúdo semeável</li><li>PostgreSQL, Redis e mídia persistente</li><li>Ambiente de desenvolvimento Docker Compose</li></ul><h2>Variantes de tecnologia</h2><p>O portfólio também inclui uma implementação do JGOB em Rust/Actix, Askama e MongoDB com conteúdo editável e exportação estática para Vercel.</p>",seo_title:"JGOB — plataforma de comunidade, conteúdo e comércio | Italy Developers",seo_description:"Uma plataforma Django/PostgreSQL combinando conteúdo organizacional, causas, voluntariado, loja, carrinho, checkout e fluxos de pagamento Razorpay.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"storemate-crm-inventory",lang:"it",title:"StoreMate — backend CRM e gestione inventario",eyebrow:"CRM e inventario",summary:"Un sistema Django REST Framework per autenticazione, profili aziendali, prodotti, magazzino, fornitori, avvisi e comunicazione automatizzata.",glance:"Un backend documentato e testato con autenticazione OTP e avvisi di scorte basse; il repository privato è disponibile su richiesta.",body:"<p class=\"lead\">StoreMate dimostra il lavoro API operativo su identità, inventario e comunicazione con i clienti.</p><h2>Funzionalità implementate</h2><ul><li>Autenticazione JWT e registrazione via email/telefono</li><li>Verifica OTP e recupero password</li><li>Profili e informazioni aziendali</li><li>Inventario, prodotti, categorie e fornitori</li><li>Notifiche email per scorte basse e sicurezza</li><li>PostgreSQL, Redis, Celery e punti di integrazione Firebase</li><li>Schema API navigabile e collezione Postman</li></ul><p>Il repository è privato; questa pagina descrive un'implementazione locale verificata senza pubblicare credenziali o codice privato.</p>",seo_title:"StoreMate — backend CRM e gestione inventario | Italy Developers",seo_description:"Un sistema Django REST Framework per autenticazione, profili aziendali, prodotti, magazzino, fornitori, avvisi e comunicazione automatizzata.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"storemate-crm-inventory",lang:"de",title:"StoreMate — CRM- und Inventar-Betriebsbackend",eyebrow:"CRM und Inventar",summary:"Ein Django-REST-Framework-System für Authentifizierung, Geschäftsprofile, Produkte, Lager, Lieferanten, Warnungen und automatisierte Kommunikation.",glance:"Ein dokumentiertes, getestetes Backend mit OTP-Auth und Niedrigbestandswarnungen; das private Repository ist auf Anfrage verfügbar.",body:"<p class=\"lead\">StoreMate zeigt operative API-Arbeit über Identität, Inventar und Kundenkommunikation.</p><h2>Implementierte Funktionen</h2><ul><li>JWT-Authentifizierung und E-Mail-/Telefon-Registrierung</li><li>OTP-Verifizierung und Passwort-Wiederherstellung</li><li>Profile und Geschäftsinformationen</li><li>Inventar, Produkte, Kategorien und Lieferanten</li><li>Niedrigbestands- und Sicherheits-E-Mail-Benachrichtigungen</li><li>PostgreSQL-, Redis-, Celery- und Firebase-Integrationspunkte</li><li>Durchsuchbares API-Schema und Postman-Sammlung</li></ul><p>Das Repository ist privat; diese Seite beschreibt verifizierte lokale Implementierung, ohne Zugangsdaten oder privaten Quellcode zu veröffentlichen.</p>",seo_title:"StoreMate — CRM- und Inventar-Betriebsbackend | Italy Developers",seo_description:"Ein Django-REST-Framework-System für Authentifizierung, Geschäftsprofile, Produkte, Lager, Lieferanten, Warnungen und automatisierte Kommunikation.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"storemate-crm-inventory",lang:"fr",title:"StoreMate — backend CRM et gestion des stocks",eyebrow:"CRM et inventaire",summary:"Un système Django REST Framework pour l'authentification, les profils d'entreprise, les produits, le stock, les fournisseurs, les alertes et la communication automatisée.",glance:"Un backend documenté et testé avec authentification OTP et alertes de stock bas ; le dépôt privé est disponible sur demande.",body:"<p class=\"lead\">StoreMate démontre un travail API opérationnel sur l'identité, l'inventaire et la communication client.</p><h2>Fonctionnalités implémentées</h2><ul><li>Authentification JWT et inscription par e-mail/téléphone</li><li>Vérification OTP et récupération de mot de passe</li><li>Profils et informations d'entreprise</li><li>Inventaire, produits, catégories et fournisseurs</li><li>Notifications par e-mail de stock bas et de sécurité</li><li>Points d'intégration PostgreSQL, Redis, Celery et Firebase</li><li>Schéma API navigable et collection Postman</li></ul><p>Le dépôt est privé ; cette page décrit une implémentation locale vérifiée sans publier d'identifiants ni de code source privé.</p>",seo_title:"StoreMate — backend CRM et gestion des stocks | Italy Developers",seo_description:"Un système Django REST Framework pour l'authentification, les profils d'entreprise, les produits, le stock, les fournisseurs, les alertes et la communication automatisée.",cta:"Discutons d'un projet concret"},
        Translated{slug:"storemate-crm-inventory",lang:"pt",title:"StoreMate — backend de CRM e gestão de estoque",eyebrow:"CRM e estoque",summary:"Um sistema Django REST Framework para autenticação, perfis empresariais, produtos, estoque, fornecedores, alertas e comunicação automatizada.",glance:"Um backend documentado e testado com autenticação OTP e alertas de estoque baixo; o repositório privado está disponível sob solicitação.",body:"<p class=\"lead\">O StoreMate demonstra trabalho operacional de API em identidade, estoque e comunicação com clientes.</p><h2>Recursos implementados</h2><ul><li>Autenticação JWT e registro por e-mail/telefone</li><li>Verificação OTP e recuperação de senha</li><li>Perfis e informações empresariais</li><li>Estoque, produtos, categorias e fornecedores</li><li>Notificações por e-mail de estoque baixo e segurança</li><li>Pontos de integração PostgreSQL, Redis, Celery e Firebase</li><li>Esquema de API navegável e coleção Postman</li></ul><p>O repositório é privado; esta página descreve implementação local verificada sem publicar credenciais ou código-fonte privado.</p>",seo_title:"StoreMate — backend de CRM e gestão de estoque | Italy Developers",seo_description:"Um sistema Django REST Framework para autenticação, perfis empresariais, produtos, estoque, fornecedores, alertas e comunicação automatizada.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"ai-chat-support",lang:"it",title:"Chat e supporto clienti abilitati dall'AI",eyebrow:"Supporto AI",summary:"Una capacità di prodotto di supporto che combina interfacce di conversazione, contesto cliente strutturato, flussi operatore e risposte assistite dall'AI.",glance:"Interfaccia di conversazione più passaggio a un operatore — l'architettura citata nella nostra pagina di servizio automazione AI.",body:"<p class=\"lead\">Il lavoro si concentra su un'assistenza utile con controllo umano, non su un'affermazione infondata di supporto completamente autonomo.</p><h2>Aree di capacità</h2><ul><li>Interfacce di conversazione e messaggistica</li><li>Contesto cliente e cronologia del supporto</li><li>Bozze assistite dall'AI e recupero della conoscenza</li><li>Passaggio a un operatore e flussi di stato</li><li>Configurazione admin e integrazione API</li><li>Confini chiari per la privacy e le decisioni ad alto impatto</li></ul><p>I link a cliente e deployment restano privati; il servizio viene offerto solo dopo aver confermato la fonte dei dati, il costo del modello e il flusso di escalation.</p>",seo_title:"Chat e supporto clienti abilitati dall'AI | Italy Developers",seo_description:"Una capacità di prodotto di supporto che combina interfacce di conversazione, contesto cliente strutturato, flussi operatore e risposte assistite dall'AI.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"ai-chat-support",lang:"de",title:"KI-gestützter Chat und Kundensupport",eyebrow:"KI-Support",summary:"Eine Support-Produktfähigkeit, die Konversationsschnittstellen, strukturierten Kundenkontext, Operator-Workflows und KI-gestützte Antworten kombiniert.",glance:"Konversationsschnittstelle plus Operator-Übergabe — die Architektur, die auf unserer KI-Automatisierungs-Service-Seite referenziert wird.",body:"<p class=\"lead\">Die Arbeit konzentriert sich auf nützliche Unterstützung mit menschlicher Kontrolle — nicht auf eine unbelegte Behauptung vollständig autonomen Supports.</p><h2>Fähigkeitsbereiche</h2><ul><li>Konversations- und Nachrichtenschnittstellen</li><li>Kundenkontext und Support-Historie</li><li>KI-gestütztes Entwerfen und Wissensabruf</li><li>Operator-Übergabe und Statusworkflows</li><li>Admin-Konfiguration und API-Integration</li><li>Klare Grenzen für Datenschutz und folgenreiche Entscheidungen</li></ul><p>Kunden- und Deployment-Links bleiben privat; der Service wird nur nach Bestätigung von Datenquelle, Modellkosten und Eskalationsworkflow angeboten.</p>",seo_title:"KI-gestützter Chat und Kundensupport | Italy Developers",seo_description:"Eine Support-Produktfähigkeit, die Konversationsschnittstellen, strukturierten Kundenkontext, Operator-Workflows und KI-gestützte Antworten kombiniert.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"ai-chat-support",lang:"fr",title:"Chat et support client optimisés par l'IA",eyebrow:"Support IA",summary:"Une capacité produit de support combinant interfaces de conversation, contexte client structuré, flux opérateur et réponses assistées par IA.",glance:"Interface de conversation plus transfert opérateur — l'architecture référencée sur notre page de service automatisation IA.",body:"<p class=\"lead\">Le travail se concentre sur une assistance utile avec contrôle humain — pas sur une affirmation infondée de support entièrement autonome.</p><h2>Domaines de capacité</h2><ul><li>Interfaces de conversation et de messagerie</li><li>Contexte client et historique du support</li><li>Rédaction assistée par IA et recherche de connaissances</li><li>Transfert opérateur et flux de statut</li><li>Configuration admin et intégration API</li><li>Limites claires pour la confidentialité et les décisions à fort impact</li></ul><p>Les liens client et de déploiement restent privés ; le service n'est proposé qu'après confirmation de la source de données, du coût du modèle et du flux d'escalade.</p>",seo_title:"Chat et support client optimisés par l'IA | Italy Developers",seo_description:"Une capacité produit de support combinant interfaces de conversation, contexte client structuré, flux opérateur et réponses assistées par IA.",cta:"Discutons d'un projet concret"},
        Translated{slug:"ai-chat-support",lang:"pt",title:"Chat e suporte ao cliente com IA",eyebrow:"Suporte com IA",summary:"Uma capacidade de produto de suporte combinando interfaces de conversação, contexto de cliente estruturado, fluxos de operador e respostas assistidas por IA.",glance:"Interface de conversação mais transferência para operador — a arquitetura referenciada na nossa página de serviço de automação com IA.",body:"<p class=\"lead\">O trabalho foca em assistência útil com controle humano — não em uma alegação infundada de suporte totalmente autônomo.</p><h2>Áreas de capacidade</h2><ul><li>Interfaces de conversação e mensagens</li><li>Contexto do cliente e histórico de suporte</li><li>Redação assistida por IA e recuperação de conhecimento</li><li>Transferência para operador e fluxos de status</li><li>Configuração de admin e integração de API</li><li>Limites claros para privacidade e decisões de alto impacto</li></ul><p>Os links de cliente e implantação permanecem privados; o serviço é oferecido apenas após confirmar a fonte de dados, o custo do modelo e o fluxo de escalonamento.</p>",seo_title:"Chat e suporte ao cliente com IA | Italy Developers",seo_description:"Uma capacidade de produto de suporte combinando interfaces de conversação, contexto de cliente estruturado, fluxos de operador e respostas assistidas por IA.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"music-application",lang:"it",title:"Applicazione musicale",eyebrow:"Prodotto media",summary:"Un prodotto incentrato sui media che copre scoperta musicale, interfacce orientate alla riproduzione, organizzazione della libreria ed esperienze account.",glance:"Un prodotto di scoperta media che mostra il nostro approccio a libreria, riproduzione e design dell'esperienza account.",body:"<p class=\"lead\">L'app musicale dimostra capacità di design di interfaccia consumer e prodotto media.</p><h2>Aree del prodotto</h2><ul><li>Navigazione di brani e raccolte</li><li>Interfaccia di ricerca e scoperta</li><li>Flussi orientati a libreria utente e playlist</li><li>Esperienza di riproduzione responsive</li><li>Fondamenta di account e amministrazione</li></ul><p>Questo progetto privato è mostrato come capacità di prodotto. Licenze, acquisizione del catalogo e infrastruttura di streaming commerciale sono requisiti aziendali separati e non sono implicati.</p>",seo_title:"Applicazione musicale | Italy Developers",seo_description:"Un prodotto incentrato sui media che copre scoperta musicale, interfacce orientate alla riproduzione, organizzazione della libreria ed esperienze account.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"music-application",lang:"de",title:"Musik-App",eyebrow:"Medienprodukt",summary:"Ein medienorientiertes Produkt für Musikentdeckung, wiedergabeorientierte Interfaces, Bibliotheksorganisation und Kontoerlebnisse.",glance:"Ein Medienentdeckungsprodukt, das unseren Ansatz für Bibliothek, Wiedergabe und Kontoerlebnis-Design zeigt.",body:"<p class=\"lead\">Die Musik-App zeigt Fähigkeiten im Design von Verbraucherschnittstellen und Medienprodukten.</p><h2>Produktbereiche</h2><ul><li>Titel- und Sammlungsdurchsuchung</li><li>Such- und Entdeckungsschnittstelle</li><li>Nutzerbibliotheks- und Playlist-orientierte Abläufe</li><li>Responsives Wiedergabeerlebnis</li><li>Konto- und Verwaltungsgrundlagen</li></ul><p>Dieses private Projekt wird als Produktfähigkeit gezeigt. Lizenzierung, Katalogerwerb und kommerzielle Streaming-Infrastruktur sind separate Geschäftsanforderungen und werden nicht impliziert.</p>",seo_title:"Musik-App | Italy Developers",seo_description:"Ein medienorientiertes Produkt für Musikentdeckung, wiedergabeorientierte Interfaces, Bibliotheksorganisation und Kontoerlebnisse.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"music-application",lang:"fr",title:"Application musicale",eyebrow:"Produit média",summary:"Un produit axé sur les médias couvrant la découverte musicale, les interfaces orientées lecture, l'organisation de bibliothèque et les expériences de compte.",glance:"Un produit de découverte média montrant notre approche de la bibliothèque, de la lecture et de la conception de l'expérience de compte.",body:"<p class=\"lead\">L'application musicale démontre une capacité de conception d'interface grand public et de produit média.</p><h2>Domaines du produit</h2><ul><li>Navigation des pistes et collections</li><li>Interface de recherche et de découverte</li><li>Flux orientés bibliothèque utilisateur et playlist</li><li>Expérience de lecture responsive</li><li>Fondations de compte et d'administration</li></ul><p>Ce projet privé est présenté comme une capacité produit. La licence, l'acquisition de catalogue et l'infrastructure de streaming commerciale sont des exigences commerciales distinctes et ne sont pas impliquées.</p>",seo_title:"Application musicale | Italy Developers",seo_description:"Un produit axé sur les médias couvrant la découverte musicale, les interfaces orientées lecture, l'organisation de bibliothèque et les expériences de compte.",cta:"Discutons d'un projet concret"},
        Translated{slug:"music-application",lang:"pt",title:"Aplicativo de música",eyebrow:"Produto de mídia",summary:"Um produto focado em mídia que abrange descoberta musical, interfaces orientadas à reprodução, organização de biblioteca e experiências de conta.",glance:"Um produto de descoberta de mídia que mostra nossa abordagem para biblioteca, reprodução e design de experiência de conta.",body:"<p class=\"lead\">O aplicativo de música demonstra capacidade de design de interface para consumidores e produtos de mídia.</p><h2>Áreas do produto</h2><ul><li>Navegação de faixas e coleções</li><li>Interface de busca e descoberta</li><li>Fluxos orientados a biblioteca de usuário e playlist</li><li>Experiência de reprodução responsiva</li><li>Fundamentos de conta e administração</li></ul><p>Este projeto privado é mostrado como capacidade de produto. Licenciamento, aquisição de catálogo e infraestrutura de streaming comercial são requisitos de negócio separados e não estão implícitos.</p>",seo_title:"Aplicativo de música | Italy Developers",seo_description:"Um produto focado em mídia que abrange descoberta musical, interfaces orientadas à reprodução, organização de biblioteca e experiências de conta.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"coinprofit-plus",lang:"it",title:"CoinProfit Plus — prodotto dashboard orientato alla finanza",eyebrow:"Prodotto dashboard",summary:"Un'applicazione di portfolio incentrata su dashboard account, registri finanziari, visibilità dello stato e controllo amministrativo.",glance:"Un prodotto dashboard-first che copre registri account e controllo amministrativo — prova per la nostra pagina di servizio software.",body:"<p class=\"lead\">CoinProfit Plus dimostra un'implementazione di dashboard e workflow densi di dati.</p><h2>Capacità dimostrate</h2><ul><li>Flussi account e profilo</li><li>Riepiloghi dashboard e cronologia record</li><li>Viste amministrative e gestione dello stato</li><li>Presentazione dati responsive</li><li>Architettura attenta a validazione e sicurezza</li></ul><p>Questa pagina non fornisce consulenza finanziaria, non promette rendimenti né dichiara servizi finanziari regolamentati. Metriche pubbliche di transazione o performance non sono intenzionalmente inventate.</p>",seo_title:"CoinProfit Plus — prodotto dashboard orientato alla finanza | Italy Developers",seo_description:"Un'applicazione di portfolio incentrata su dashboard account, registri finanziari, visibilità dello stato e controllo amministrativo.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"coinprofit-plus",lang:"de",title:"CoinProfit Plus — finanzorientiertes Dashboard-Produkt",eyebrow:"Dashboard-Produkt",summary:"Eine Portfolio-Anwendung mit Fokus auf Konto-Dashboards, Finanzunterlagen, Statustransparenz und administrative Kontrolle.",glance:"Ein Dashboard-first-Produkt für Kontounterlagen und administrative Kontrolle — Beleg für unsere Software-Service-Seite.",body:"<p class=\"lead\">CoinProfit Plus zeigt datenintensive Dashboard- und Workflow-Implementierung.</p><h2>Gezeigte Fähigkeiten</h2><ul><li>Konto- und Profilabläufe</li><li>Dashboard-Zusammenfassungen und Verlaufshistorie</li><li>Administrative Ansichten und Statusverwaltung</li><li>Responsive Datendarstellung</li><li>Validierungs- und sicherheitsbewusste Architektur</li></ul><p>Diese Seite bietet keine Anlageberatung, verspricht keine Renditen und behauptet keine regulierten Finanzdienstleistungen. Öffentliche Transaktions- oder Performance-Metriken werden bewusst nicht erfunden.</p>",seo_title:"CoinProfit Plus — finanzorientiertes Dashboard-Produkt | Italy Developers",seo_description:"Eine Portfolio-Anwendung mit Fokus auf Konto-Dashboards, Finanzunterlagen, Statustransparenz und administrative Kontrolle.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"coinprofit-plus",lang:"fr",title:"CoinProfit Plus — produit de tableau de bord orienté finance",eyebrow:"Produit tableau de bord",summary:"Une application de portfolio centrée sur les tableaux de bord de compte, les registres financiers, la visibilité du statut et le contrôle administratif.",glance:"Un produit dashboard-first couvrant les registres de compte et le contrôle administratif — preuve pour notre page de service logiciel.",body:"<p class=\"lead\">CoinProfit Plus démontre une implémentation de tableau de bord et de flux de travail dense en données.</p><h2>Capacité démontrée</h2><ul><li>Flux de compte et de profil</li><li>Résumés de tableau de bord et historique des registres</li><li>Vues administratives et gestion du statut</li><li>Présentation de données responsive</li><li>Architecture soucieuse de la validation et de la sécurité</li></ul><p>Cette page ne fournit pas de conseils en investissement, ne promet pas de rendements et ne revendique pas de services financiers réglementés. Les indicateurs publics de transaction ou de performance ne sont intentionnellement pas inventés.</p>",seo_title:"CoinProfit Plus — produit de tableau de bord orienté finance | Italy Developers",seo_description:"Une application de portfolio centrée sur les tableaux de bord de compte, les registres financiers, la visibilité du statut et le contrôle administratif.",cta:"Discutons d'un projet concret"},
        Translated{slug:"coinprofit-plus",lang:"pt",title:"CoinProfit Plus — produto de painel voltado para finanças",eyebrow:"Produto de painel",summary:"Uma aplicação de portfólio centrada em painéis de conta, registros financeiros, visibilidade de status e controle administrativo.",glance:"Um produto dashboard-first que cobre registros de conta e controle administrativo — prova para a nossa página de serviço de software.",body:"<p class=\"lead\">O CoinProfit Plus demonstra implementação de painel e fluxo de trabalho densos em dados.</p><h2>Capacidade demonstrada</h2><ul><li>Fluxos de conta e perfil</li><li>Resumos de painel e histórico de registros</li><li>Visualizações administrativas e gestão de status</li><li>Apresentação de dados responsiva</li><li>Arquitetura atenta à validação e segurança</li></ul><p>Esta página não fornece aconselhamento de investimento, não promete retornos nem alega serviços financeiros regulamentados. Métricas públicas de transação ou desempenho não são intencionalmente inventadas.</p>",seo_title:"CoinProfit Plus — produto de painel voltado para finanças | Italy Developers",seo_description:"Uma aplicação de portfólio centrada em painéis de conta, registros financeiros, visibilidade de status e controle administrativo.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"gaming-platform",lang:"it",title:"Esperienza di piattaforma di gioco",eyebrow:"Prodotto interattivo",summary:"Una capacità applicativa orientata al gioco che copre account giocatore, stato interattivo, viste di progressione e contenuti amministrativi.",glance:"Account giocatore, stato di progressione e contenuti admin — prova di capacità di prodotto interattivo.",body:"<p class=\"lead\">Il lavoro di gaming dimostra interfacce di prodotto consumer stateful e i flussi di backend a supporto.</p><h2>Aree di capacità</h2><ul><li>Identità giocatore ed esperienza profilo</li><li>Stato di gioco e presentazione dei progressi</li><li>Modelli dati pronti per punteggio, premi o classifiche</li><li>Interfaccia interattiva responsive</li><li>Fondamenta di contenuti amministrativi e moderazione</li></ul><p>Meccaniche di gioco specifiche e integrazioni commerciali restano private e vengono definite caso per caso.</p>",seo_title:"Esperienza di piattaforma di gioco | Italy Developers",seo_description:"Una capacità applicativa orientata al gioco che copre account giocatore, stato interattivo, viste di progressione e contenuti amministrativi.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"gaming-platform",lang:"de",title:"Gaming-Plattform-Erlebnis",eyebrow:"Interaktives Produkt",summary:"Eine spielorientierte Anwendungsfähigkeit für Spielerkonten, interaktiven Zustand, Fortschrittsansichten und administrative Inhalte.",glance:"Spielerkonten, Fortschrittszustand und Admin-Inhalte — Beleg für interaktive Produktfähigkeit.",body:"<p class=\"lead\">Die Gaming-Arbeit zeigt zustandsbehaftete Verbraucherprodukt-Interfaces und unterstützende Backend-Workflows.</p><h2>Fähigkeitsbereiche</h2><ul><li>Spieleridentität und Profilerlebnis</li><li>Spielzustand und Fortschrittsdarstellung</li><li>Punktestand-, Belohnungs- oder Rangliste-fertige Datenmodelle</li><li>Responsive interaktive Schnittstelle</li><li>Administrative Inhalts- und Moderationsgrundlagen</li></ul><p>Spezifische Spielmechaniken und kommerzielle Integrationen bleiben privat und werden von Fall zu Fall festgelegt.</p>",seo_title:"Gaming-Plattform-Erlebnis | Italy Developers",seo_description:"Eine spielorientierte Anwendungsfähigkeit für Spielerkonten, interaktiven Zustand, Fortschrittsansichten und administrative Inhalte.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"gaming-platform",lang:"fr",title:"Expérience de plateforme de jeu",eyebrow:"Produit interactif",summary:"Une capacité applicative orientée jeu couvrant les comptes joueurs, l'état interactif, les vues de progression et le contenu administratif.",glance:"Comptes joueurs, état de progression et contenu admin — preuve de capacité de produit interactif.",body:"<p class=\"lead\">Le travail de jeu démontre des interfaces de produit grand public à état et les flux backend associés.</p><h2>Domaines de capacité</h2><ul><li>Identité du joueur et expérience de profil</li><li>État du jeu et présentation de la progression</li><li>Modèles de données prêts pour score, récompense ou classement</li><li>Interface interactive responsive</li><li>Fondations de contenu administratif et de modération</li></ul><p>Les mécaniques de jeu spécifiques et les intégrations commerciales restent privées et sont définies au cas par cas.</p>",seo_title:"Expérience de plateforme de jeu | Italy Developers",seo_description:"Une capacité applicative orientée jeu couvrant les comptes joueurs, l'état interactif, les vues de progression et le contenu administratif.",cta:"Discutons d'un projet concret"},
        Translated{slug:"gaming-platform",lang:"pt",title:"Experiência de plataforma de jogos",eyebrow:"Produto interativo",summary:"Uma capacidade de aplicação orientada a jogos que abrange contas de jogadores, estado interativo, visualizações de progressão e conteúdo administrativo.",glance:"Contas de jogadores, estado de progressão e conteúdo admin — prova de capacidade de produto interativo.",body:"<p class=\"lead\">O trabalho de jogos demonstra interfaces de produto para consumidores com estado e fluxos de backend de suporte.</p><h2>Áreas de capacidade</h2><ul><li>Identidade do jogador e experiência de perfil</li><li>Estado do jogo e apresentação de progresso</li><li>Modelos de dados prontos para pontuação, recompensa ou ranking</li><li>Interface interativa responsiva</li><li>Fundamentos de conteúdo administrativo e moderação</li></ul><p>Mecânicas de jogo específicas e integrações comerciais permanecem privadas e são definidas caso a caso.</p>",seo_title:"Experiência de plataforma de jogos | Italy Developers",seo_description:"Uma capacidade de aplicação orientada a jogos que abrange contas de jogadores, estado interativo, visualizações de progressão e conteúdo administrativo.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"car-parking-system",lang:"it",title:"Sistema di gestione parcheggi",eyebrow:"Sistema operativo",summary:"Un prodotto di workflow per parcheggi che copre spazi, veicoli, registri di entrata/uscita, disponibilità e amministrazione operativa.",glance:"Flusso completo di entrata, uscita e occupazione — lo stesso schema di sistemi operativi dietro la nostra pagina software su misura.",body:"<p class=\"lead\">Il progetto di parcheggio dimostra la modellazione di workflow reali su risorse e transazioni.</p><h2>Aree del prodotto</h2><ul><li>Gestione spazi e zone di parcheggio</li><li>Registri veicoli e utenti</li><li>Stato di entrata, uscita e occupazione</li><li>Dashboard operatore e cronologia ricercabile</li><li>Confini di workflow pronti per il pagamento</li><li>Report e controlli amministrativi</li></ul><p>Barriere hardware, riconoscimento targhe e fornitori di pagamento vengono offerti solo dopo la conferma dei dispositivi e delle integrazioni richieste.</p>",seo_title:"Sistema di gestione parcheggi | Italy Developers",seo_description:"Un prodotto di workflow per parcheggi che copre spazi, veicoli, registri di entrata/uscita, disponibilità e amministrazione operativa.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"car-parking-system",lang:"de",title:"Parkplatzverwaltungssystem",eyebrow:"Betriebssystem",summary:"Ein Parkplatz-Workflow-Produkt für Stellplätze, Fahrzeuge, Ein-/Ausfahrtsprotokolle, Verfügbarkeit und operative Verwaltung.",glance:"Vollständiger Ein-, Ausfahrt- und Belegungsworkflow — dasselbe Betriebssystem-Muster wie auf unserer Individuelle-Software-Seite.",body:"<p class=\"lead\">Das Parkplatzprojekt zeigt reale Ressourcen- und Transaktions-Workflow-Modellierung.</p><h2>Produktbereiche</h2><ul><li>Parkplatz- und Zonenverwaltung</li><li>Fahrzeug- und Nutzerdatensätze</li><li>Ein-, Ausfahrt- und Belegungsstatus</li><li>Operator-Dashboard und durchsuchbare Historie</li><li>Zahlungsbereite Workflow-Grenzen</li><li>Berichte und administrative Kontrollen</li></ul><p>Hardware-Schranken, Kennzeichenerkennung und Zahlungsanbieter werden nur angeboten, wenn die erforderlichen Geräte und Integrationen bestätigt sind.</p>",seo_title:"Parkplatzverwaltungssystem | Italy Developers",seo_description:"Ein Parkplatz-Workflow-Produkt für Stellplätze, Fahrzeuge, Ein-/Ausfahrtsprotokolle, Verfügbarkeit und operative Verwaltung.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"car-parking-system",lang:"fr",title:"Système de gestion de parking",eyebrow:"Système opérationnel",summary:"Un produit de flux de travail pour parkings couvrant places, véhicules, registres d'entrée/sortie, disponibilité et administration opérationnelle.",glance:"Flux complet d'entrée, sortie et occupation — le même schéma de systèmes opérationnels derrière notre page logiciel sur mesure.",body:"<p class=\"lead\">Le projet de parking démontre une modélisation réelle de flux de ressources et de transactions.</p><h2>Domaines du produit</h2><ul><li>Gestion des places et zones de parking</li><li>Registres de véhicules et d'utilisateurs</li><li>Statut d'entrée, de sortie et d'occupation</li><li>Tableau de bord opérateur et historique consultable</li><li>Limites de flux de travail prêtes pour le paiement</li><li>Rapports et contrôles administratifs</li></ul><p>Les barrières matérielles, la reconnaissance de plaques d'immatriculation et les fournisseurs de paiement ne sont proposés qu'après confirmation des appareils et intégrations requis.</p>",seo_title:"Système de gestion de parking | Italy Developers",seo_description:"Un produit de flux de travail pour parkings couvrant places, véhicules, registres d'entrée/sortie, disponibilité et administration opérationnelle.",cta:"Discutons d'un projet concret"},
        Translated{slug:"car-parking-system",lang:"pt",title:"Sistema de gestão de estacionamento",eyebrow:"Sistema operacional",summary:"Um produto de fluxo de trabalho de estacionamento que abrange vagas, veículos, registros de entrada/saída, disponibilidade e administração operacional.",glance:"Fluxo completo de entrada, saída e ocupação — o mesmo padrão de sistemas operacionais por trás da nossa página de software sob medida.",body:"<p class=\"lead\">O projeto de estacionamento demonstra modelagem real de fluxo de trabalho de recursos e transações.</p><h2>Áreas do produto</h2><ul><li>Gestão de vagas e zonas de estacionamento</li><li>Registros de veículos e usuários</li><li>Status de entrada, saída e ocupação</li><li>Painel do operador e histórico pesquisável</li><li>Limites de fluxo de trabalho prontos para pagamento</li><li>Relatórios e controles administrativos</li></ul><p>Cancelas físicas, reconhecimento de placas e provedores de pagamento são oferecidos apenas após a confirmação dos dispositivos e integrações necessários.</p>",seo_title:"Sistema de gestão de estacionamento | Italy Developers",seo_description:"Um produto de fluxo de trabalho de estacionamento que abrange vagas, veículos, registros de entrada/saída, disponibilidade e administração operacional.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"pet-care-ai-upcoming",lang:"it",title:"Pet Care AI — piattaforma di analisi comportamentale in arrivo",eyebrow:"In arrivo · AI responsabile",summary:"Una piattaforma Django in sviluppo per profili di animali, analisi audio, stime probabilistiche dello stato comportamentale del cane e feedback del proprietario.",glance:"Attualmente in sviluppo: analisi audio probabilistica con incertezza visibile, non un'affermazione finita.",body:"<p class=\"lead\"><strong>Progetto in arrivo:</strong> Pet Care AI viene sviluppato come strumento di supporto consapevole dell'incertezza, non come traduttore del linguaggio animale o prodotto di diagnosi veterinaria.</p><h2>Fondamenta implementate</h2><ul><li>Account con login via email e profili animali per proprietario</li><li>Caricamenti audio privati e registri di elaborazione</li><li>Embedding audio del cane con teste di comportamento supervisionate</li><li>Presentazione di distribuzione di probabilità, confidenza e rischio</li><li>Feedback, progresso live, GraphQL ed endpoint di health check</li><li>Architettura Django, PostgreSQL, Celery e Docker</li></ul><h2>Confine dell'AI responsabile</h2><p>I risultati descrivono possibili stati comportamentali ed espongono l'incertezza. Le preoccupazioni mediche devono essere indirizzate a un veterinario qualificato. Il supporto del modello attuale è specifico per i cani e i limiti di licenza sono documentati.</p>",seo_title:"Pet Care AI — piattaforma di analisi comportamentale in arrivo | Italy Developers",seo_description:"Una piattaforma Django in sviluppo per profili di animali, analisi audio, stime probabilistiche dello stato comportamentale del cane e feedback del proprietario.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"pet-care-ai-upcoming",lang:"de",title:"Pet Care AI — kommende Plattform für Verhaltenseinblicke",eyebrow:"Demnächst · Verantwortungsvolle KI",summary:"Eine in Entwicklung befindliche Django-Plattform für Tierprofile, Audioanalyse, probabilistische Verhaltenszustandsschätzungen für Hunde und Besitzer-Feedback.",glance:"Derzeit in Entwicklung: probabilistische Audioanalyse mit sichtbarer Unsicherheit, keine fertige Behauptung.",body:"<p class=\"lead\"><strong>Kommendes Projekt:</strong> Pet Care AI wird als unsicherheitsbewusstes Unterstützungstool entwickelt, nicht als Tiersprachenübersetzer oder tierärztliches Diagnoseprodukt.</p><h2>Implementierte Grundlage</h2><ul><li>E-Mail-Login-Konten und besitzergebundene Tierprofile</li><li>Private Audio-Uploads und Verarbeitungsprotokolle</li><li>Hunde-Audio-Embeddings mit überwachten Verhaltens-Heads</li><li>Darstellung von Wahrscheinlichkeitsverteilung, Konfidenz und Risiko</li><li>Feedback, Live-Fortschritt, GraphQL- und Health-Endpunkte</li><li>Django-, PostgreSQL-, Celery- und Docker-Architektur</li></ul><h2>Grenze verantwortungsvoller KI</h2><p>Ergebnisse beschreiben mögliche Verhaltenszustände und legen Unsicherheit offen. Medizinische Anliegen müssen an einen qualifizierten Tierarzt gehen. Die aktuelle Modellunterstützung ist hundespezifisch, Lizenzgrenzen sind dokumentiert.</p>",seo_title:"Pet Care AI — kommende Plattform für Verhaltenseinblicke | Italy Developers",seo_description:"Eine in Entwicklung befindliche Django-Plattform für Tierprofile, Audioanalyse, probabilistische Verhaltenszustandsschätzungen für Hunde und Besitzer-Feedback.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"pet-care-ai-upcoming",lang:"fr",title:"Pet Care AI — plateforme d'analyse comportementale à venir",eyebrow:"À venir · IA responsable",summary:"Une plateforme Django en développement pour profils d'animaux, analyse audio, estimations probabilistes de l'état comportemental du chien et retours du propriétaire.",glance:"Actuellement en développement : analyse audio probabiliste avec incertitude visible, pas une affirmation finale.",body:"<p class=\"lead\"><strong>Projet à venir :</strong> Pet Care AI est développé comme un outil de support conscient de l'incertitude, pas comme un traducteur du langage animal ni un produit de diagnostic vétérinaire.</p><h2>Fondation implémentée</h2><ul><li>Comptes de connexion par e-mail et profils d'animaux liés au propriétaire</li><li>Téléversements audio privés et registres de traitement</li><li>Embeddings audio de chien avec têtes de comportement supervisées</li><li>Présentation de distribution de probabilité, confiance et risque</li><li>Retours, progression en direct, points de terminaison GraphQL et santé</li><li>Architecture Django, PostgreSQL, Celery et Docker</li></ul><h2>Limite de l'IA responsable</h2><p>Les résultats décrivent des états comportementaux possibles et exposent l'incertitude. Les préoccupations médicales doivent être adressées à un vétérinaire qualifié. Le support actuel du modèle est spécifique au chien et les limites de licence sont documentées.</p>",seo_title:"Pet Care AI — plateforme d'analyse comportementale à venir | Italy Developers",seo_description:"Une plateforme Django en développement pour profils d'animaux, analyse audio, estimations probabilistes de l'état comportemental du chien et retours du propriétaire.",cta:"Discutons d'un projet concret"},
        Translated{slug:"pet-care-ai-upcoming",lang:"pt",title:"Pet Care AI — plataforma de percepção comportamental em desenvolvimento",eyebrow:"Em breve · IA responsável",summary:"Uma plataforma Django em desenvolvimento para perfis de animais, análise de áudio, estimativas probabilísticas do estado comportamental do cão e feedback do proprietário.",glance:"Atualmente em desenvolvimento: análise de áudio probabilística com incerteza visível, não uma alegação finalizada.",body:"<p class=\"lead\"><strong>Projeto em desenvolvimento:</strong> o Pet Care AI está sendo desenvolvido como uma ferramenta de suporte consciente da incerteza, não como um tradutor de linguagem animal ou produto de diagnóstico veterinário.</p><h2>Base implementada</h2><ul><li>Contas de login por e-mail e perfis de animais vinculados ao proprietário</li><li>Uploads de áudio privados e registros de processamento</li><li>Embeddings de áudio de cães com cabeças de comportamento supervisionadas</li><li>Apresentação de distribuição de probabilidade, confiança e risco</li><li>Feedback, progresso ao vivo, endpoints GraphQL e de saúde</li><li>Arquitetura Django, PostgreSQL, Celery e Docker</li></ul><h2>Limite da IA responsável</h2><p>Os resultados descrevem possíveis estados comportamentais e expõem a incerteza. Questões médicas devem ser direcionadas a um veterinário qualificado. O suporte atual do modelo é específico para cães e os limites de licenciamento estão documentados.</p>",seo_title:"Pet Care AI — plataforma de percepção comportamental em desenvolvimento | Italy Developers",seo_description:"Uma plataforma Django em desenvolvimento para perfis de animais, análise de áudio, estimativas probabilísticas do estado comportamental do cão e feedback do proprietário.",cta:"Vamos falar sobre um projeto concreto"},
    ];
    apply_translations(db, "work", &rows).await
}

async fn apply_translations_blog_v1(db: &Database) -> Result<(), AppError> {
    let rows: Vec<Translated> = vec![
        Translated{slug:"rust-actix-production-checklist",lang:"it",title:"Una checklist di produzione per Rust e Actix Web",eyebrow:"Deployment Rust",summary:"I controlli pratici che usiamo prima di mettere un servizio Actix Web dietro un dominio reale.",glance:"",body:"<p class=\"lead\">Una build di release è solo una parte della prontezza per la produzione. Configurazione, comportamento in caso di errore e responsabilità contano altrettanto.</p><h2>Controlli dell'applicazione</h2><ul><li>Valida le variabili d'ambiente all'avvio</li><li>Limita le dimensioni di JSON, form e upload</li><li>Usa cookie di sessione sicuri e HTTP-only</li><li>Applica protezione CSRF ai form che modificano lo stato</li><li>Restituisci segnali di salute live e ready separati</li></ul><h2>Controlli del container</h2><p>Esegui come utente non-root, usa un filesystem in sola lettura dove possibile, mantieni solo i dati necessari e tieni le porte del database fuori dall'host pubblico.</p><h2>Controlli operativi</h2><p>Testa il ripristino, la rotazione dei log, gli aggiornamenti delle dipendenze e un rollback prima del lancio. Un endpoint di salute non è una strategia di backup.</p>",seo_title:"Una checklist di produzione per Rust e Actix Web | Italy Developers",seo_description:"I controlli pratici che usiamo prima di mettere un servizio Actix Web dietro un dominio reale.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"rust-actix-production-checklist",lang:"de",title:"Eine Produktions-Checkliste für Rust und Actix Web",eyebrow:"Rust-Deployment",summary:"Die praktischen Prüfungen, die wir durchführen, bevor ein Actix-Web-Dienst hinter eine echte Domain gestellt wird.",glance:"",body:"<p class=\"lead\">Ein Release-Build ist nur ein Teil der Produktionsreife. Konfiguration, Fehlerverhalten und Zuständigkeit zählen genauso.</p><h2>Anwendungsprüfungen</h2><ul><li>Umgebungsvariablen beim Start validieren</li><li>JSON-, Formular- und Upload-Größen begrenzen</li><li>Sichere, HTTP-only Session-Cookies verwenden</li><li>CSRF-Schutz auf zustandsändernde Formulare anwenden</li><li>Getrennte Live- und Ready-Health-Signale zurückgeben</li></ul><h2>Container-Prüfungen</h2><p>Als Nicht-Root-Nutzer ausführen, wo möglich ein schreibgeschütztes Dateisystem verwenden, nur notwendige Daten persistieren und Datenbank-Ports vom öffentlichen Host fernhalten.</p><h2>Betriebsprüfungen</h2><p>Wiederherstellung, Log-Rotation, Abhängigkeits-Updates und ein Rollback vor dem Start testen. Ein Health-Endpunkt ist keine Backup-Strategie.</p>",seo_title:"Eine Produktions-Checkliste für Rust und Actix Web | Italy Developers",seo_description:"Die praktischen Prüfungen, die wir durchführen, bevor ein Actix-Web-Dienst hinter eine echte Domain gestellt wird.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"rust-actix-production-checklist",lang:"fr",title:"Une checklist de production pour Rust et Actix Web",eyebrow:"Déploiement Rust",summary:"Les vérifications pratiques que nous utilisons avant de mettre un service Actix Web derrière un domaine réel.",glance:"",body:"<p class=\"lead\">Une build de release n'est qu'une partie de la préparation à la production. La configuration, le comportement en cas d'échec et la responsabilité comptent tout autant.</p><h2>Vérifications applicatives</h2><ul><li>Valider les variables d'environnement au démarrage</li><li>Limiter les tailles JSON, formulaire et upload</li><li>Utiliser des cookies de session sécurisés et HTTP-only</li><li>Appliquer une protection CSRF aux formulaires modifiant l'état</li><li>Retourner des signaux de santé live et ready séparés</li></ul><h2>Vérifications du conteneur</h2><p>Exécuter en tant qu'utilisateur non-root, utiliser un système de fichiers en lecture seule si possible, ne persister que les données nécessaires et garder les ports de base de données hors de l'hôte public.</p><h2>Vérifications opérationnelles</h2><p>Tester la restauration, la rotation des logs, les mises à jour de dépendances et un rollback avant le lancement. Un endpoint de santé n'est pas une stratégie de sauvegarde.</p>",seo_title:"Une checklist de production pour Rust et Actix Web | Italy Developers",seo_description:"Les vérifications pratiques que nous utilisons avant de mettre un service Actix Web derrière un domaine réel.",cta:"Discutons d'un projet concret"},
        Translated{slug:"rust-actix-production-checklist",lang:"pt",title:"Uma checklist de produção para Rust e Actix Web",eyebrow:"Implantação Rust",summary:"As verificações práticas que usamos antes de colocar um serviço Actix Web atrás de um domínio real.",glance:"",body:"<p class=\"lead\">Uma build de release é apenas uma parte da prontidão para produção. Configuração, comportamento em falhas e responsabilidade importam igualmente.</p><h2>Verificações da aplicação</h2><ul><li>Validar variáveis de ambiente na inicialização</li><li>Limitar tamanhos de JSON, formulário e upload</li><li>Usar cookies de sessão seguros e HTTP-only</li><li>Aplicar proteção CSRF a formulários que alteram estado</li><li>Retornar sinais de saúde live e ready separados</li></ul><h2>Verificações do container</h2><p>Executar como usuário não-root, usar um sistema de arquivos somente leitura quando possível, persistir apenas dados necessários e manter as portas do banco de dados fora do host público.</p><h2>Verificações operacionais</h2><p>Testar restauração, rotação de logs, atualizações de dependências e um rollback antes do lançamento. Um endpoint de saúde não é uma estratégia de backup.</p>",seo_title:"Uma checklist de produção para Rust e Actix Web | Italy Developers",seo_description:"As verificações práticas que usamos antes de colocar um serviço Actix Web atrás de um domínio real.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"designing-secure-cms",lang:"it",title:"Progettare un CMS sicuro per piccoli team",eyebrow:"Ingegneria CMS",summary:"Come bilanciare la comodità dell'editor con ruoli, validazione, upload sicuri e operazioni recuperabili.",glance:"",body:"<p class=\"lead\">Un CMS è un'applicazione privilegiata. Merita confini più forti delle pagine di marketing pubbliche che controlla.</p><h2>Separa i permessi</h2><p>Gli editor possono creare bozze e aggiornare contenuti, mentre pubblicazione, eliminazione e accesso ai contatti restano ai ruoli fidati. Le sessioni dovrebbero scadere e i cookie non dovrebbero essere leggibili dagli script del browser.</p><h2>Tratta gli upload come input ostile</h2><p>Controlla dimensione, estensione e firme dei file; genera nomi file imprevedibili; archivia fuori dai percorsi eseguibili; e servi con policy di contenuto rigorose.</p><h2>Rendi gli errori recuperabili</h2><p>Mantieni insieme i backup di database e upload, registra le modifiche importanti ed evita azioni di massa distruttive senza conferma.</p>",seo_title:"Progettare un CMS sicuro per piccoli team | Italy Developers",seo_description:"Come bilanciare la comodità dell'editor con ruoli, validazione, upload sicuri e operazioni recuperabili.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"designing-secure-cms",lang:"de",title:"Ein sicheres CMS für kleine Teams entwerfen",eyebrow:"CMS-Engineering",summary:"Wie man Editor-Komfort mit Rollen, Validierung, sicheren Uploads und wiederherstellbaren Abläufen ausbalanciert.",glance:"",body:"<p class=\"lead\">Ein CMS ist eine privilegierte Anwendung. Es verdient stärkere Grenzen als die öffentlichen Marketingseiten, die es steuert.</p><h2>Berechtigungen trennen</h2><p>Redakteure können Inhalte entwerfen und aktualisieren, während Veröffentlichung, Löschung und Zugriff auf Leads bei vertrauenswürdigen Rollen bleiben. Sitzungen sollten ablaufen und Cookies sollten nicht von Browser-Skripten lesbar sein.</p><h2>Uploads als feindliche Eingabe behandeln</h2><p>Größe, Erweiterung und Dateisignaturen prüfen; unvorhersehbare Dateinamen generieren; außerhalb ausführbarer Pfade speichern; und mit strikten Content-Policies ausliefern.</p><h2>Fehler wiederherstellbar machen</h2><p>Datenbank- und Upload-Backups zusammenhalten, wichtige Änderungen protokollieren und destruktive Massenaktionen ohne Bestätigung vermeiden.</p>",seo_title:"Ein sicheres CMS für kleine Teams entwerfen | Italy Developers",seo_description:"Wie man Editor-Komfort mit Rollen, Validierung, sicheren Uploads und wiederherstellbaren Abläufen ausbalanciert.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"designing-secure-cms",lang:"fr",title:"Concevoir un CMS sécurisé pour petites équipes",eyebrow:"Ingénierie CMS",summary:"Comment équilibrer le confort de l'éditeur avec les rôles, la validation, les téléversements sécurisés et des opérations récupérables.",glance:"",body:"<p class=\"lead\">Un CMS est une application privilégiée. Il mérite des limites plus strictes que les pages marketing publiques qu'il contrôle.</p><h2>Séparer les permissions</h2><p>Les éditeurs peuvent rédiger et mettre à jour le contenu, tandis que la publication, la suppression et l'accès aux prospects restent aux rôles de confiance. Les sessions doivent expirer et les cookies ne doivent pas être lisibles par les scripts du navigateur.</p><h2>Traiter les téléversements comme une entrée hostile</h2><p>Vérifier la taille, l'extension et les signatures de fichiers ; générer des noms de fichiers imprévisibles ; stocker en dehors des chemins exécutables ; et servir avec des politiques de contenu strictes.</p><h2>Rendre les erreurs récupérables</h2><p>Garder ensemble les sauvegardes de base de données et de téléversement, journaliser les changements importants et éviter les actions groupées destructrices sans confirmation.</p>",seo_title:"Concevoir un CMS sécurisé pour petites équipes | Italy Developers",seo_description:"Comment équilibrer le confort de l'éditeur avec les rôles, la validation, les téléversements sécurisés et des opérations récupérables.",cta:"Discutons d'un projet concret"},
        Translated{slug:"designing-secure-cms",lang:"pt",title:"Projetando um CMS seguro para equipes pequenas",eyebrow:"Engenharia de CMS",summary:"Como equilibrar a conveniência do editor com funções, validação, uploads seguros e operações recuperáveis.",glance:"",body:"<p class=\"lead\">Um CMS é uma aplicação privilegiada. Ele merece limites mais fortes do que as páginas de marketing públicas que controla.</p><h2>Separe as permissões</h2><p>Editores podem rascunhar e atualizar conteúdo, enquanto publicação, exclusão e acesso a leads permanecem com funções confiáveis. Sessões devem expirar e cookies não devem ser legíveis por scripts do navegador.</p><h2>Trate uploads como entrada hostil</h2><p>Verifique tamanho, extensão e assinaturas de arquivo; gere nomes de arquivo imprevisíveis; armazene fora de caminhos executáveis; e sirva com políticas de conteúdo rígidas.</p><h2>Torne os erros recuperáveis</h2><p>Mantenha backups de banco de dados e uploads juntos, registre mudanças importantes e evite ações em massa destrutivas sem confirmação.</p>",seo_title:"Projetando um CMS seguro para equipes pequenas | Italy Developers",seo_description:"Como equilibrar a conveniência do editor com funções, validação, uploads seguros e operações recuperáveis.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"mongodb-content-modeling",lang:"it",title:"Modellazione dei contenuti MongoDB senza perdere disciplina",eyebrow:"MongoDB",summary:"Un database a documenti è flessibile, ma i sistemi di contenuto utili hanno comunque bisogno di forme esplicite, indici e strategia di migrazione.",glance:"",body:"<p class=\"lead\">La flessibilità aiuta i contenuti a evolversi; non dovrebbe significare che ogni documento abbia uno schema accidentale.</p><h2>Modella intorno ai pattern di accesso</h2><p>Mantieni contenuti, utenti, richieste e commenti in collezioni separate. Interroga per campi stabili come tipo, slug, stato di pubblicazione e data di creazione.</p><h2>Usa la validazione dell'applicazione</h2><p>Le strutture Rust tipizzate forniscono default per i documenti più vecchi, mentre i form admin impongono regole su titolo, slug, riepilogo, SEO e immagini.</p><h2>Migra intenzionalmente</h2><p>Versiona i seed editoriali e rendi le migrazioni idempotenti. Non sovrascrivere mai record creati arbitrariamente dagli editor solo perché il servizio è ripartito.</p>",seo_title:"Modellazione dei contenuti MongoDB senza perdere disciplina | Italy Developers",seo_description:"Un database a documenti è flessibile, ma i sistemi di contenuto utili hanno comunque bisogno di forme esplicite, indici e strategia di migrazione.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"mongodb-content-modeling",lang:"de",title:"MongoDB-Content-Modellierung ohne Disziplinverlust",eyebrow:"MongoDB",summary:"Eine Dokumentendatenbank ist flexibel, aber nützliche Content-Systeme brauchen dennoch explizite Formen, Indizes und eine Migrationsstrategie.",glance:"",body:"<p class=\"lead\">Flexibilität hilft Inhalten, sich weiterzuentwickeln; sie sollte nicht bedeuten, dass jedes Dokument ein zufälliges Schema hat.</p><h2>Nach Zugriffsmustern modellieren</h2><p>Inhalte, Nutzer, Anfragen und Kommentare in separaten Collections halten. Nach stabilen Feldern wie Typ, Slug, Veröffentlichungsstatus und Erstellungsdatum abfragen.</p><h2>Anwendungsvalidierung nutzen</h2><p>Typisierte Rust-Strukturen liefern Defaults für ältere Dokumente, während Admin-Formulare Titel-, Slug-, Zusammenfassungs-, SEO- und Bildregeln durchsetzen.</p><h2>Absichtlich migrieren</h2><p>Redaktionelle Seeds versionieren und Migrationen idempotent machen. Niemals beliebige, von Redakteuren erstellte Datensätze überschreiben, nur weil der Dienst neu gestartet wurde.</p>",seo_title:"MongoDB-Content-Modellierung ohne Disziplinverlust | Italy Developers",seo_description:"Eine Dokumentendatenbank ist flexibel, aber nützliche Content-Systeme brauchen dennoch explizite Formen, Indizes und eine Migrationsstrategie.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"mongodb-content-modeling",lang:"fr",title:"Modélisation de contenu MongoDB sans perdre en rigueur",eyebrow:"MongoDB",summary:"Une base de données documentaire est flexible, mais les systèmes de contenu utiles ont quand même besoin de formes explicites, d'index et d'une stratégie de migration.",glance:"",body:"<p class=\"lead\">La flexibilité aide le contenu à évoluer ; cela ne devrait pas signifier que chaque document a un schéma accidentel.</p><h2>Modéliser autour des schémas d'accès</h2><p>Garder contenu, utilisateurs, demandes et commentaires dans des collections séparées. Interroger par des champs stables comme le type, le slug, l'état de publication et la date de création.</p><h2>Utiliser la validation applicative</h2><p>Les structures Rust typées fournissent des valeurs par défaut pour les anciens documents, tandis que les formulaires admin imposent des règles de titre, slug, résumé, SEO et image.</p><h2>Migrer intentionnellement</h2><p>Versionner les seeds éditoriaux et rendre les migrations idempotentes. Ne jamais écraser des enregistrements créés arbitrairement par les éditeurs simplement parce que le service a redémarré.</p>",seo_title:"Modélisation de contenu MongoDB sans perdre en rigueur | Italy Developers",seo_description:"Une base de données documentaire est flexible, mais les systèmes de contenu utiles ont quand même besoin de formes explicites, d'index et d'une stratégie de migration.",cta:"Discutons d'un projet concret"},
        Translated{slug:"mongodb-content-modeling",lang:"pt",title:"Modelagem de conteúdo MongoDB sem perder disciplina",eyebrow:"MongoDB",summary:"Um banco de dados de documentos é flexível, mas sistemas de conteúdo úteis ainda precisam de formas explícitas, índices e estratégia de migração.",glance:"",body:"<p class=\"lead\">A flexibilidade ajuda o conteúdo a evoluir; não deveria significar que todo documento tem um esquema acidental.</p><h2>Modele em torno de padrões de acesso</h2><p>Mantenha conteúdo, usuários, solicitações e comentários em coleções separadas. Consulte por campos estáveis como tipo, slug, estado de publicação e data de criação.</p><h2>Use validação da aplicação</h2><p>Estruturas Rust tipadas fornecem padrões para documentos mais antigos, enquanto formulários admin impõem regras de título, slug, resumo, SEO e imagem.</p><h2>Migre intencionalmente</h2><p>Versione os seeds editoriais e torne as migrações idempotentes. Nunca sobrescreva registros criados arbitrariamente por editores só porque o serviço reiniciou.</p>",seo_title:"Modelagem de conteúdo MongoDB sem perder disciplina | Italy Developers",seo_description:"Um banco de dados de documentos é flexível, mas sistemas de conteúdo úteis ainda precisam de formas explícitas, índices e estratégia de migração.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"django-rest-framework-dynamic-serializers",lang:"it",title:"Quando i serializer dinamici di Django REST Framework aiutano",eyebrow:"Django REST Framework",summary:"Come la selezione dei campi a runtime può ridurre la duplicazione senza trasformare le risposte API in un liberi-tutti non documentato.",glance:"",body:"<p class=\"lead\">Endpoint di lista, dettaglio, esportazione e consapevoli dei permessi spesso necessitano rappresentazioni diverse dello stesso modello.</p><h2>Il problema della duplicazione</h2><p>Creare una classe serializer per ogni piccola variazione aumenta la manutenzione e rende ripetitive le modifiche annidate.</p><h2>Un approccio dinamico controllato</h2><p>Permetti di configurare a runtime campi noti, rinomine, attributi e serializer annidati, mantenendo esplicite le regole di modello e permessi.</p><h2>Usalo con intenzionalità</h2><p>Documenta le forme di risposta supportate, testa l'annidamento ed evita che parametri di query non fidati espongano campi arbitrari. Guarda l'implementazione in <a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">DRF Shapeless Serializers</a>.</p>",seo_title:"Quando i serializer dinamici di Django REST Framework aiutano | Italy Developers",seo_description:"Come la selezione dei campi a runtime può ridurre la duplicazione senza trasformare le risposte API in un liberi-tutti non documentato.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"django-rest-framework-dynamic-serializers",lang:"de",title:"Wann dynamische Django-REST-Framework-Serializer helfen",eyebrow:"Django REST Framework",summary:"Wie die Laufzeit-Feldauswahl Duplizierung reduzieren kann, ohne API-Antworten in ein undokumentiertes Durcheinander zu verwandeln.",glance:"",body:"<p class=\"lead\">Listen-, Detail-, Export- und berechtigungsbewusste Endpunkte benötigen oft unterschiedliche Darstellungen desselben Modells.</p><h2>Das Duplizierungsproblem</h2><p>Für jede kleine Variation eine eigene Serializer-Klasse zu erstellen, erhöht den Wartungsaufwand und macht verschachtelte Änderungen repetitiv.</p><h2>Ein kontrollierter dynamischer Ansatz</h2><p>Erlauben Sie, bekannte Felder, Umbenennungen, Attribute und verschachtelte Serializer zur Laufzeit zu konfigurieren, während Modell- und Berechtigungsregeln explizit bleiben.</p><h2>Bewusst einsetzen</h2><p>Unterstützte Antwortformen dokumentieren, Verschachtelung testen und verhindern, dass nicht vertrauenswürdige Query-Parameter beliebige Felder offenlegen. Siehe die Implementierung in <a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">DRF Shapeless Serializers</a>.</p>",seo_title:"Wann dynamische Django-REST-Framework-Serializer helfen | Italy Developers",seo_description:"Wie die Laufzeit-Feldauswahl Duplizierung reduzieren kann, ohne API-Antworten in ein undokumentiertes Durcheinander zu verwandeln.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"django-rest-framework-dynamic-serializers",lang:"fr",title:"Quand les sérialiseurs dynamiques Django REST Framework aident",eyebrow:"Django REST Framework",summary:"Comment la sélection de champs à l'exécution peut réduire la duplication sans transformer les réponses API en fourre-tout non documenté.",glance:"",body:"<p class=\"lead\">Les endpoints de liste, détail, export et sensibles aux permissions ont souvent besoin de représentations différentes du même modèle.</p><h2>Le problème de duplication</h2><p>Créer une classe de sérialiseur pour chaque petite variation augmente la maintenance et rend les modifications imbriquées répétitives.</p><h2>Une approche dynamique contrôlée</h2><p>Permettre de configurer à l'exécution les champs connus, les renommages, les attributs et les sérialiseurs imbriqués, tout en gardant explicites les règles de modèle et de permission.</p><h2>Utiliser cela délibérément</h2><p>Documenter les formes de réponse prises en charge, tester l'imbrication et éviter que des paramètres de requête non fiables n'exposent des champs arbitraires. Voir l'implémentation dans <a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">DRF Shapeless Serializers</a>.</p>",seo_title:"Quand les sérialiseurs dynamiques Django REST Framework aident | Italy Developers",seo_description:"Comment la sélection de champs à l'exécution peut réduire la duplication sans transformer les réponses API en fourre-tout non documenté.",cta:"Discutons d'un projet concret"},
        Translated{slug:"django-rest-framework-dynamic-serializers",lang:"pt",title:"Quando serializers dinâmicos do Django REST Framework ajudam",eyebrow:"Django REST Framework",summary:"Como a seleção de campos em tempo de execução pode reduzir duplicação sem transformar respostas de API em uma bagunça não documentada.",glance:"",body:"<p class=\"lead\">Endpoints de lista, detalhe, exportação e conscientes de permissões costumam precisar de representações diferentes do mesmo modelo.</p><h2>O problema da duplicação</h2><p>Criar uma classe de serializer para cada pequena variação aumenta a manutenção e torna as mudanças aninhadas repetitivas.</p><h2>Uma abordagem dinâmica controlada</h2><p>Permita configurar em tempo de execução campos conhecidos, renomeações, atributos e serializers aninhados, mantendo explícitas as regras de modelo e permissão.</p><h2>Use isso deliberadamente</h2><p>Documente os formatos de resposta suportados, teste o aninhamento e evite que parâmetros de consulta não confiáveis exponham campos arbitrários. Veja a implementação em <a href=\"https://github.com/khajanksj/drf-shapeless-serializers\" target=\"_blank\" rel=\"noopener\">DRF Shapeless Serializers</a>.</p>",seo_title:"Quando serializers dinâmicos do Django REST Framework ajudam | Italy Developers",seo_description:"Como a seleção de campos em tempo de execução pode reduzir duplicação sem transformar respostas de API em uma bagunça não documentada.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"nested-comments-data-model",lang:"it",title:"Costruire commenti annidati e like senza un framework front-end",eyebrow:"Funzionalità community",summary:"Un modello pratico reso lato server per risposte, reazioni, validazione e miglioramento progressivo.",glance:"",body:"<p class=\"lead\">Le funzionalità di discussione non richiedono una grande applicazione client. Form standard e redirect restano affidabili anche con JavaScript disabilitato.</p><h2>Memorizza la relazione</h2><p>Ogni commento mantiene uno slug del post e un identificatore genitore opzionale. Il rendering parte dai commenti radice e attraversa i figli con una profondità massima per proteggere la pagina.</p><h2>Rendi i like idempotenti</h2><p>Memorizza una reazione per visitatore e target, poi alternala. Il contatore visualizzato viene aggiornato con la reazione così i clic ripetuti non creano like illimitati.</p><h2>Proteggi ogni scrittura</h2><p>Usa token CSRF, limiti di lunghezza dell'input, output con escape e rate limiting. Moderazione e segnalazione abusi sono i prossimi requisiti per un lancio pubblico aperto.</p>",seo_title:"Costruire commenti annidati e like senza un framework front-end | Italy Developers",seo_description:"Un modello pratico reso lato server per risposte, reazioni, validazione e miglioramento progressivo.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"nested-comments-data-model",lang:"de",title:"Verschachtelte Kommentare und Likes ohne Frontend-Framework bauen",eyebrow:"Community-Funktionen",summary:"Ein praktisches serverseitig gerendertes Modell für Antworten, Reaktionen, Validierung und progressive Erweiterung.",glance:"",body:"<p class=\"lead\">Diskussionsfunktionen erfordern keine große Client-Anwendung. Standardformulare und Weiterleitungen bleiben auch mit deaktiviertem JavaScript zuverlässig.</p><h2>Die Beziehung speichern</h2><p>Jeder Kommentar behält einen Post-Slug und eine optionale übergeordnete Kennung. Das Rendering beginnt bei Root-Kommentaren und durchläuft Kinder mit einer maximalen Tiefe, um die Seite zu schützen.</p><h2>Likes idempotent machen</h2><p>Eine Reaktion pro Besucher und Ziel speichern, dann umschalten. Der angezeigte Zähler wird mit der Reaktion aktualisiert, sodass wiederholte Klicks keine unbegrenzten Likes erzeugen.</p><h2>Jeden Schreibvorgang schützen</h2><p>CSRF-Tokens, Eingabelängenbegrenzungen, escapte Ausgabe und Rate-Limiting verwenden. Moderation und Missbrauchsmeldung sind die nächsten Anforderungen für einen offenen öffentlichen Start.</p>",seo_title:"Verschachtelte Kommentare und Likes ohne Frontend-Framework bauen | Italy Developers",seo_description:"Ein praktisches serverseitig gerendertes Modell für Antworten, Reaktionen, Validierung und progressive Erweiterung.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"nested-comments-data-model",lang:"fr",title:"Construire des commentaires imbriqués et des likes sans framework front-end",eyebrow:"Fonctionnalités communautaires",summary:"Un modèle pratique rendu côté serveur pour les réponses, réactions, validation et amélioration progressive.",glance:"",body:"<p class=\"lead\">Les fonctionnalités de discussion ne nécessitent pas une grande application cliente. Les formulaires standards et redirections restent fiables avec JavaScript désactivé.</p><h2>Stocker la relation</h2><p>Chaque commentaire garde un slug d'article et un identifiant parent optionnel. Le rendu commence par les commentaires racines et parcourt les enfants avec une profondeur maximale pour protéger la page.</p><h2>Rendre les likes idempotents</h2><p>Stocker une réaction par visiteur et cible, puis la basculer. Le compteur affiché est mis à jour avec la réaction afin que les clics répétés ne créent pas de likes illimités.</p><h2>Protéger chaque écriture</h2><p>Utiliser des jetons CSRF, des limites de longueur d'entrée, une sortie échappée et une limitation de débit. La modération et le signalement d'abus sont les prochaines exigences pour un lancement public ouvert.</p>",seo_title:"Construire des commentaires imbriqués et des likes sans framework front-end | Italy Developers",seo_description:"Un modèle pratique rendu côté serveur pour les réponses, réactions, validation et amélioration progressive.",cta:"Discutons d'un projet concret"},
        Translated{slug:"nested-comments-data-model",lang:"pt",title:"Construindo comentários aninhados e curtidas sem um framework front-end",eyebrow:"Recursos de comunidade",summary:"Um modelo prático renderizado no servidor para respostas, reações, validação e aprimoramento progressivo.",glance:"",body:"<p class=\"lead\">Recursos de discussão não exigem uma grande aplicação cliente. Formulários padrão e redirecionamentos permanecem confiáveis com JavaScript desativado.</p><h2>Armazene o relacionamento</h2><p>Cada comentário mantém um slug de post e um identificador de pai opcional. A renderização começa nos comentários raiz e percorre os filhos com uma profundidade máxima para proteger a página.</p><h2>Torne as curtidas idempotentes</h2><p>Armazene uma reação por visitante e alvo, depois alterne-a. O contador exibido é atualizado com a reação para que cliques repetidos não criem curtidas ilimitadas.</p><h2>Proteja cada escrita</h2><p>Use tokens CSRF, limites de comprimento de entrada, saída com escape e limitação de taxa. Moderação e denúncia de abuso são os próximos requisitos para um lançamento público aberto.</p>",seo_title:"Construindo comentários aninhados e curtidas sem um framework front-end | Italy Developers",seo_description:"Um modelo prático renderizado no servidor para respostas, reações, validação e aprimoramento progressivo.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"docker-compose-production-overrides",lang:"it",title:"Override di Docker Compose che si comportano bene in produzione",eyebrow:"Docker",summary:"Perché liste unite, volumi persistenti e interpolazione dell'ambiente meritano test prima del deployment.",glance:"",body:"<p class=\"lead\">Compose unisce più file, ma non ogni campo sostituisce il valore precedente. Liste come le porte possono produrre duplicati sorprendenti.</p><h2>Ispeziona il risultato unito</h2><p>Esegui <code>docker compose config</code> con la combinazione di file reale. Sovrascrivi esplicitamente le mappature delle porte quando una mappatura di sviluppo deve sparire.</p><h2>Separa i segreti dai template</h2><p>Committa un file d'ambiente di esempio, ignora quello reale e fai fallire l'avvio quando restano i placeholder.</p><h2>Rispetta i volumi esistenti</h2><p>Le variabili di inizializzazione del database di solito si applicano solo a una directory dati vuota. Pianifica migrazioni di autenticazione invece di eliminare dati per far partire un container.</p>",seo_title:"Override di Docker Compose che si comportano bene in produzione | Italy Developers",seo_description:"Perché liste unite, volumi persistenti e interpolazione dell'ambiente meritano test prima del deployment.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"docker-compose-production-overrides",lang:"de",title:"Docker-Compose-Overrides, die sich in der Produktion richtig verhalten",eyebrow:"Docker",summary:"Warum zusammengeführte Listen, persistente Volumes und Umgebungsinterpolation Tests vor dem Deployment verdienen.",glance:"",body:"<p class=\"lead\">Compose führt mehrere Dateien zusammen, aber nicht jedes Feld ersetzt den vorherigen Wert. Listen wie Ports können überraschende Duplikate erzeugen.</p><h2>Das zusammengeführte Ergebnis prüfen</h2><p><code>docker compose config</code> mit der echten Dateikombination ausführen. Port-Zuordnungen explizit überschreiben, wenn eine Entwicklungszuordnung verschwinden muss.</p><h2>Geheimnisse von Vorlagen trennen</h2><p>Eine Beispiel-Umgebungsdatei committen, die echte ignorieren und den Start fehlschlagen lassen, wenn Platzhalter übrig bleiben.</p><h2>Bestehende Volumes respektieren</h2><p>Datenbank-Initialisierungsvariablen gelten normalerweise nur für ein leeres Datenverzeichnis. Authentifizierungs-Migrationen planen, statt Daten zu löschen, um einen Container zu starten.</p>",seo_title:"Docker-Compose-Overrides, die sich in der Produktion richtig verhalten | Italy Developers",seo_description:"Warum zusammengeführte Listen, persistente Volumes und Umgebungsinterpolation Tests vor dem Deployment verdienen.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"docker-compose-production-overrides",lang:"fr",title:"Surcharges Docker Compose qui se comportent bien en production",eyebrow:"Docker",summary:"Pourquoi les listes fusionnées, les volumes persistants et l'interpolation d'environnement méritent des tests avant le déploiement.",glance:"",body:"<p class=\"lead\">Compose fusionne plusieurs fichiers, mais chaque champ ne remplace pas la valeur précédente. Des listes comme les ports peuvent produire des doublons surprenants.</p><h2>Inspecter le résultat fusionné</h2><p>Exécuter <code>docker compose config</code> avec la combinaison réelle de fichiers. Surcharger explicitement les mappages de ports lorsqu'un mappage de développement doit disparaître.</p><h2>Séparer les secrets des modèles</h2><p>Committer un fichier d'environnement exemple, ignorer le vrai et faire échouer le démarrage lorsque des espaces réservés subsistent.</p><h2>Respecter les volumes existants</h2><p>Les variables d'initialisation de base de données ne s'appliquent généralement qu'à un répertoire de données vide. Planifier des migrations d'authentification plutôt que de supprimer des données pour démarrer un conteneur.</p>",seo_title:"Surcharges Docker Compose qui se comportent bien en production | Italy Developers",seo_description:"Pourquoi les listes fusionnées, les volumes persistants et l'interpolation d'environnement méritent des tests avant le déploiement.",cta:"Discutons d'un projet concret"},
        Translated{slug:"docker-compose-production-overrides",lang:"pt",title:"Overrides do Docker Compose que se comportam bem em produção",eyebrow:"Docker",summary:"Por que listas mescladas, volumes persistentes e interpolação de ambiente merecem testes antes da implantação.",glance:"",body:"<p class=\"lead\">O Compose mescla vários arquivos, mas nem todo campo substitui o valor anterior. Listas como portas podem produzir duplicatas surpreendentes.</p><h2>Inspecione o resultado mesclado</h2><p>Execute <code>docker compose config</code> com a combinação real de arquivos. Substitua explicitamente os mapeamentos de porta quando um mapeamento de desenvolvimento precisar desaparecer.</p><h2>Separe segredos de templates</h2><p>Faça commit de um arquivo de ambiente de exemplo, ignore o real e falhe a inicialização quando placeholders permanecerem.</p><h2>Respeite volumes existentes</h2><p>Variáveis de inicialização de banco de dados geralmente se aplicam apenas a um diretório de dados vazio. Planeje migrações de autenticação em vez de excluir dados para fazer um container iniciar.</p>",seo_title:"Overrides do Docker Compose que se comportam bem em produção | Italy Developers",seo_description:"Por que listas mescladas, volumes persistentes e interpolação de ambiente merecem testes antes da implantação.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"server-rendered-seo-basics",lang:"it",title:"Fondamenti SEO per siti aziendali resi lato server",eyebrow:"SEO tecnica",summary:"Le basi tecniche ed editoriali che rendono le pagine individuabili senza rincorrere trucchi per i motori di ricerca.",glance:"",body:"<p class=\"lead\">La visibilità nei motori di ricerca inizia con pagine utili che si caricano in modo affidabile, rispondono a un bisogno chiaro e possono essere comprese senza eseguire JavaScript.</p><h2>Dai a ogni pagina un solo compito</h2><p>Usa un titolo descrittivo, un riepilogo utile, intestazioni logiche, link interni e un'azione successiva chiara. Evita di clonare pagine sottili per ogni variazione di parola chiave.</p><h2>Fornisci metadati completi</h2><p>URL canonici, descrizioni, immagini social, dati strutturati, voci sitemap e regole di crawl dovrebbero riflettere il contenuto pubblicato.</p><h2>Misura le azioni di business</h2><p>Traccia richieste qualificate, prenotazioni o download, non solo screenshot di posizionamento. Migliora le pagine con domande reali dei clienti.</p>",seo_title:"Fondamenti SEO per siti aziendali resi lato server | Italy Developers",seo_description:"Le basi tecniche ed editoriali che rendono le pagine individuabili senza rincorrere trucchi per i motori di ricerca.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"server-rendered-seo-basics",lang:"de",title:"SEO-Grundlagen für serverseitig gerenderte Unternehmenswebsites",eyebrow:"Technisches SEO",summary:"Die technischen und redaktionellen Grundlagen, die Seiten auffindbar machen, ohne Suchmaschinen-Tricks nachzujagen.",glance:"",body:"<p class=\"lead\">Suchsichtbarkeit beginnt mit nützlichen Seiten, die zuverlässig laden, ein klares Bedürfnis beantworten und ohne JavaScript-Ausführung verstanden werden können.</p><h2>Jeder Seite eine Aufgabe geben</h2><p>Einen aussagekräftigen Titel, eine nützliche Zusammenfassung, logische Überschriften, interne Links und eine klare nächste Handlung verwenden. Das Klonen dünner Seiten für jede Keyword-Variation vermeiden.</p><h2>Vollständige Metadaten liefern</h2><p>Kanonische URLs, Beschreibungen, Social-Bilder, strukturierte Daten, Sitemap-Einträge und Crawl-Regeln sollten den veröffentlichten Inhalt widerspiegeln.</p><h2>Geschäftsaktionen messen</h2><p>Qualifizierte Anfragen, Buchungen oder Downloads verfolgen — nicht nur Ranking-Screenshots. Seiten mit echten Kundenfragen verbessern.</p>",seo_title:"SEO-Grundlagen für serverseitig gerenderte Unternehmenswebsites | Italy Developers",seo_description:"Die technischen und redaktionellen Grundlagen, die Seiten auffindbar machen, ohne Suchmaschinen-Tricks nachzujagen.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"server-rendered-seo-basics",lang:"fr",title:"Fondamentaux SEO pour sites d'entreprise rendus côté serveur",eyebrow:"SEO technique",summary:"Les bases techniques et éditoriales qui rendent les pages détectables sans courir après les astuces des moteurs de recherche.",glance:"",body:"<p class=\"lead\">La visibilité dans les recherches commence par des pages utiles qui se chargent de manière fiable, répondent à un besoin clair et peuvent être comprises sans exécuter JavaScript.</p><h2>Donner une seule tâche à chaque page</h2><p>Utiliser un titre descriptif, un résumé utile, des titres logiques, des liens internes et une action suivante claire. Éviter de cloner des pages minces pour chaque variation de mot-clé.</p><h2>Livrer des métadonnées complètes</h2><p>URLs canoniques, descriptions, images sociales, données structurées, entrées de sitemap et règles de crawl devraient refléter le contenu publié.</p><h2>Mesurer les actions commerciales</h2><p>Suivre les demandes qualifiées, réservations ou téléchargements — pas seulement des captures d'écran de classement. Améliorer les pages avec de vraies questions de clients.</p>",seo_title:"Fondamentaux SEO pour sites d'entreprise rendus côté serveur | Italy Developers",seo_description:"Les bases techniques et éditoriales qui rendent les pages détectables sans courir après les astuces des moteurs de recherche.",cta:"Discutons d'un projet concret"},
        Translated{slug:"server-rendered-seo-basics",lang:"pt",title:"Fundamentos de SEO para sites empresariais renderizados no servidor",eyebrow:"SEO técnico",summary:"As bases técnicas e editoriais que tornam as páginas descobríveis sem perseguir truques de mecanismos de busca.",glance:"",body:"<p class=\"lead\">A visibilidade em buscas começa com páginas úteis que carregam de forma confiável, respondem a uma necessidade clara e podem ser entendidas sem executar JavaScript.</p><h2>Dê a cada página uma única tarefa</h2><p>Use um título descritivo, um resumo útil, cabeçalhos lógicos, links internos e uma próxima ação clara. Evite clonar páginas rasas para cada variação de palavra-chave.</p><h2>Entregue metadados completos</h2><p>URLs canônicas, descrições, imagens sociais, dados estruturados, entradas de sitemap e regras de rastreamento devem refletir o conteúdo publicado.</p><h2>Meça ações de negócio</h2><p>Acompanhe solicitações qualificadas, reservas ou downloads — não apenas capturas de tela de ranking. Melhore páginas com perguntas reais de clientes.</p>",seo_title:"Fundamentos de SEO para sites empresariais renderizados no servidor | Italy Developers",seo_description:"As bases técnicas e editoriais que tornam as páginas descobríveis sem perseguir truques de mecanismos de busca.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"accessible-admin-forms",lang:"it",title:"Form admin accessibili di cui gli editor possono fidarsi",eyebrow:"Accessibilità",summary:"Pattern per validazione, focus, etichette, errori e descrizioni immagine nelle interfacce di gestione contenuti.",glance:"",body:"<p class=\"lead\">L'accessibilità dell'admin migliora la precisione per ogni editor, specialmente in form lunghi con validazione e contenuti ricchi.</p><h2>Mantieni etichette ed errori specifici</h2><p>Ogni controllo ha bisogno di un'etichetta persistente. Metti l'errore accanto al campo, spiega l'intervallo valido e conserva i valori inseriti dopo un rifiuto.</p><h2>Supporta i flussi da tastiera</h2><p>Usa pulsanti semantici, focus visibile, ordine di tabulazione prevedibile e dialoghi che restituiscono il focus. Non rendere il drag-and-drop l'unico percorso di upload.</p><h2>Rendi visibile la qualità del contenuto</h2><p>Contatori di caratteri, anteprime e suggerimenti per il testo alternativo aiutano gli editor a pubblicare pagine migliori senza trasformare le linee guida in congetture.</p>",seo_title:"Form admin accessibili di cui gli editor possono fidarsi | Italy Developers",seo_description:"Pattern per validazione, focus, etichette, errori e descrizioni immagine nelle interfacce di gestione contenuti.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"accessible-admin-forms",lang:"de",title:"Barrierefreie Admin-Formulare, denen Redakteure vertrauen können",eyebrow:"Barrierefreiheit",summary:"Muster für Validierung, Fokus, Beschriftungen, Fehler und Bildbeschreibungen in Content-Management-Oberflächen.",glance:"",body:"<p class=\"lead\">Admin-Barrierefreiheit verbessert die Genauigkeit für jeden Redakteur, besonders in langen Formularen mit Validierung und reichhaltigem Inhalt.</p><h2>Beschriftungen und Fehler spezifisch halten</h2><p>Jedes Steuerelement braucht eine dauerhafte Beschriftung. Den Fehler neben das Feld setzen, den gültigen Bereich erklären und eingegebene Werte nach Ablehnung erhalten.</p><h2>Tastatur-Workflows unterstützen</h2><p>Semantische Buttons, sichtbaren Fokus, vorhersehbare Tab-Reihenfolge und Dialoge verwenden, die den Fokus zurückgeben. Drag-and-Drop nicht zum einzigen Upload-Weg machen.</p><h2>Inhaltsqualität sichtbar machen</h2><p>Zeichenzähler, Vorschauen und Alt-Text-Hinweise helfen Redakteuren, bessere Seiten zu veröffentlichen, ohne Richtlinien zum Rätselraten zu machen.</p>",seo_title:"Barrierefreie Admin-Formulare, denen Redakteure vertrauen können | Italy Developers",seo_description:"Muster für Validierung, Fokus, Beschriftungen, Fehler und Bildbeschreibungen in Content-Management-Oberflächen.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"accessible-admin-forms",lang:"fr",title:"Formulaires admin accessibles auxquels les éditeurs peuvent faire confiance",eyebrow:"Accessibilité",summary:"Modèles pour la validation, le focus, les étiquettes, les erreurs et les descriptions d'images dans les interfaces de gestion de contenu.",glance:"",body:"<p class=\"lead\">L'accessibilité admin améliore la précision pour chaque éditeur, surtout dans les longs formulaires avec validation et contenu riche.</p><h2>Garder les étiquettes et erreurs spécifiques</h2><p>Chaque contrôle a besoin d'une étiquette persistante. Placer l'erreur à côté du champ, expliquer la plage valide et conserver les valeurs saisies après rejet.</p><h2>Soutenir les flux au clavier</h2><p>Utiliser des boutons sémantiques, un focus visible, un ordre de tabulation prévisible et des dialogues qui restituent le focus. Ne pas faire du glisser-déposer le seul chemin de téléversement.</p><h2>Rendre visible la qualité du contenu</h2><p>Les compteurs de caractères, aperçus et invites de texte alternatif aident les éditeurs à publier de meilleures pages sans transformer les directives en devinettes.</p>",seo_title:"Formulaires admin accessibles auxquels les éditeurs peuvent faire confiance | Italy Developers",seo_description:"Modèles pour la validation, le focus, les étiquettes, les erreurs et les descriptions d'images dans les interfaces de gestion de contenu.",cta:"Discutons d'un projet concret"},
        Translated{slug:"accessible-admin-forms",lang:"pt",title:"Formulários admin acessíveis em que editores podem confiar",eyebrow:"Acessibilidade",summary:"Padrões para validação, foco, rótulos, erros e descrições de imagem em interfaces de gestão de conteúdo.",glance:"",body:"<p class=\"lead\">A acessibilidade do admin melhora a precisão para cada editor, especialmente em formulários longos com validação e conteúdo rico.</p><h2>Mantenha rótulos e erros específicos</h2><p>Cada controle precisa de um rótulo persistente. Coloque o erro ao lado do campo, explique o intervalo válido e preserve os valores inseridos após rejeição.</p><h2>Suporte fluxos de teclado</h2><p>Use botões semânticos, foco visível, ordem de tabulação previsível e diálogos que devolvem o foco. Não torne o arrastar-e-soltar o único caminho de upload.</p><h2>Torne a qualidade do conteúdo visível</h2><p>Contadores de caracteres, prévias e sugestões de texto alternativo ajudam editores a publicar páginas melhores sem transformar diretrizes em adivinhação.</p>",seo_title:"Formulários admin acessíveis em que editores podem confiar | Italy Developers",seo_description:"Padrões para validação, foco, rótulos, erros e descrições de imagem em interfaces de gestão de conteúdo.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"api-ready-python-dashboard",lang:"it",title:"Progettare una dashboard Python pronta per le API",eyebrow:"Python e Flet",summary:"Come componenti, modelli tipizzati e confini di servizio preparano un'interfaccia Flet per dati Django reali.",glance:"",body:"<p class=\"lead\">Un prototipo di dashboard diventa più facile da collegare quando i dati dimostrativi locali sono già separati dai componenti UI.</p><h2>Dividi le responsabilità</h2><p>Mantieni routing, tema, componenti, pagine, modelli di dominio e servizi dati in moduli separati. Le pagine dovrebbero richiedere dati invece di possedere i dettagli di trasporto.</p><h2>Progetta stati di caricamento e fallimento</h2><p>Un'interfaccia connessa a API necessita di comportamento vuoto, di caricamento, di validazione, di autorizzazione e di retry, non solo una tabella riuscita.</p><h2>Connetti tramite un livello di servizio</h2><p>Mappa le risposte Django REST in modelli client tipizzati. Questo impedisce che dizionari grezzi e dettagli di autenticazione trapelino attraverso ogni componente.</p>",seo_title:"Progettare una dashboard Python pronta per le API | Italy Developers",seo_description:"Come componenti, modelli tipizzati e confini di servizio preparano un'interfaccia Flet per dati Django reali.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"api-ready-python-dashboard",lang:"de",title:"Ein API-fertiges Python-Dashboard entwerfen",eyebrow:"Python und Flet",summary:"Wie Komponenten, typisierte Modelle und Servicegrenzen eine Flet-Oberfläche für echte Django-Daten vorbereiten.",glance:"",body:"<p class=\"lead\">Ein Dashboard-Prototyp lässt sich leichter anbinden, wenn lokale Demonstrationsdaten bereits von UI-Komponenten getrennt sind.</p><h2>Zuständigkeiten aufteilen</h2><p>Routing, Theme, Komponenten, Seiten, Domänenmodelle und Datendienste in separaten Modulen halten. Seiten sollten Daten anfordern, statt Transportdetails selbst zu besitzen.</p><h2>Lade- und Fehlerzustände entwerfen</h2><p>Eine API-verbundene Oberfläche braucht Leer-, Lade-, Validierungs-, Autorisierungs- und Retry-Verhalten — nicht nur eine erfolgreiche Tabelle.</p><h2>Über eine Serviceschicht verbinden</h2><p>Django-REST-Antworten in typisierte Client-Modelle abbilden. Das verhindert, dass rohe Dictionaries und Authentifizierungsdetails durch jede Komponente durchsickern.</p>",seo_title:"Ein API-fertiges Python-Dashboard entwerfen | Italy Developers",seo_description:"Wie Komponenten, typisierte Modelle und Servicegrenzen eine Flet-Oberfläche für echte Django-Daten vorbereiten.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"api-ready-python-dashboard",lang:"fr",title:"Concevoir un tableau de bord Python prêt pour l'API",eyebrow:"Python et Flet",summary:"Comment les composants, modèles typés et limites de service préparent une interface Flet pour de vraies données Django.",glance:"",body:"<p class=\"lead\">Un prototype de tableau de bord devient plus facile à connecter lorsque les données de démonstration locales sont déjà séparées des composants d'interface.</p><h2>Diviser les responsabilités</h2><p>Garder le routage, le thème, les composants, les pages, les modèles de domaine et les services de données dans des modules séparés. Les pages devraient demander des données plutôt que posséder les détails de transport.</p><h2>Concevoir les états de chargement et d'échec</h2><p>Une interface connectée à une API a besoin de comportements vide, de chargement, de validation, d'autorisation et de réessai — pas seulement d'un tableau réussi.</p><h2>Se connecter via une couche de service</h2><p>Mapper les réponses Django REST vers des modèles clients typés. Cela empêche les dictionnaires bruts et les détails d'authentification de fuiter à travers chaque composant.</p>",seo_title:"Concevoir un tableau de bord Python prêt pour l'API | Italy Developers",seo_description:"Comment les composants, modèles typés et limites de service préparent une interface Flet pour de vraies données Django.",cta:"Discutons d'un projet concret"},
        Translated{slug:"api-ready-python-dashboard",lang:"pt",title:"Projetando um painel Python pronto para API",eyebrow:"Python e Flet",summary:"Como componentes, modelos tipados e limites de serviço preparam uma interface Flet para dados reais do Django.",glance:"",body:"<p class=\"lead\">Um protótipo de painel fica mais fácil de conectar quando os dados de demonstração locais já estão separados dos componentes de UI.</p><h2>Divida responsabilidades</h2><p>Mantenha roteamento, tema, componentes, páginas, modelos de domínio e serviços de dados em módulos separados. Páginas devem solicitar dados em vez de possuir detalhes de transporte.</p><h2>Projete estados de carregamento e falha</h2><p>Uma interface conectada a API precisa de comportamento vazio, de carregamento, de validação, de autorização e de nova tentativa — não apenas uma tabela bem-sucedida.</p><h2>Conecte através de uma camada de serviço</h2><p>Mapeie respostas Django REST para modelos de cliente tipados. Isso evita que dicionários brutos e detalhes de autenticação vazem por cada componente.</p>",seo_title:"Projetando um painel Python pronto para API | Italy Developers",seo_description:"Como componentes, modelos tipados e limites de serviço preparam uma interface Flet para dados reais do Django.",cta:"Vamos falar sobre um projeto concreto"},

        Translated{slug:"small-business-website-scope",lang:"it",title:"Come definire l'ambito di un sito web utile per piccole imprese",eyebrow:"Pianificazione progetto",summary:"Un modo pratico per scegliere pagine, prove e flussi di lavoro senza promettere funzionalità che il team non può mantenere.",glance:"",body:"<p class=\"lead\">Inizia con la decisione del cliente e l'azione che l'azienda può realizzare in modo affidabile.</p><h2>Mappa le domande essenziali</h2><p>Per chi è il servizio? Quale problema risolve? Dove è disponibile? Quali prove costruiscono fiducia? Cosa succede dopo il contatto?</p><h2>Dai priorità al nucleo funzionante</h2><p>Lancia le pagine servizio più forti, il lavoro reale, le informazioni chi siamo e un percorso di richiesta affidabile prima della personalizzazione avanzata o dell'automazione.</p><h2>Mantieni chiara la proprietà</h2><p>L'azienda dovrebbe controllare il proprio dominio, contenuti, account e dati. Documenta i costi ricorrenti e scegli tecnologie che il team può supportare.</p>",seo_title:"Come definire l'ambito di un sito web utile per piccole imprese | Italy Developers",seo_description:"Un modo pratico per scegliere pagine, prove e flussi di lavoro senza promettere funzionalità che il team non può mantenere.",cta:"Parliamo di un progetto concreto"},
        Translated{slug:"small-business-website-scope",lang:"de",title:"Wie man den Umfang einer nützlichen Kleinunternehmens-Website festlegt",eyebrow:"Projektplanung",summary:"Ein praktischer Weg, Seiten, Belege und Workflows auszuwählen, ohne Funktionen zu versprechen, die das Team nicht pflegen kann.",glance:"",body:"<p class=\"lead\">Beginnen Sie mit der Kundenentscheidung und der Handlung, die das Unternehmen zuverlässig erfüllen kann.</p><h2>Die wesentlichen Fragen abbilden</h2><p>Für wen ist der Service? Welches Problem löst er? Wo ist er verfügbar? Welche Belege schaffen Vertrauen? Was passiert nach dem Kontakt?</p><h2>Den funktionierenden Kern priorisieren</h2><p>Die stärksten Servicesseiten, echte Arbeit, Über-uns-Informationen und einen zuverlässigen Anfrageweg starten, bevor fortgeschrittene Personalisierung oder Automatisierung folgen.</p><h2>Eigentum klar halten</h2><p>Das Unternehmen sollte seine Domain, Inhalte, Konten und Daten kontrollieren. Laufende Kosten dokumentieren und Technologie wählen, die das Team unterstützen kann.</p>",seo_title:"Wie man den Umfang einer nützlichen Kleinunternehmens-Website festlegt | Italy Developers",seo_description:"Ein praktischer Weg, Seiten, Belege und Workflows auszuwählen, ohne Funktionen zu versprechen, die das Team nicht pflegen kann.",cta:"Besprechen Sie ein konkretes Projekt"},
        Translated{slug:"small-business-website-scope",lang:"fr",title:"Comment définir le périmètre d'un site web utile pour petite entreprise",eyebrow:"Planification de projet",summary:"Une façon pratique de choisir pages, preuves et flux de travail sans promettre des fonctionnalités que l'équipe ne peut pas maintenir.",glance:"",body:"<p class=\"lead\">Commencer par la décision du client et l'action que l'entreprise peut accomplir de manière fiable.</p><h2>Cartographier les questions essentielles</h2><p>Pour qui est le service ? Quel problème résout-il ? Où est-il disponible ? Quelles preuves construisent la confiance ? Que se passe-t-il après le contact ?</p><h2>Prioriser le noyau fonctionnel</h2><p>Lancer les pages de service les plus solides, le travail réel, les informations à propos et un parcours de demande fiable avant la personnalisation avancée ou l'automatisation.</p><h2>Garder la propriété claire</h2><p>L'entreprise devrait contrôler son domaine, contenu, comptes et données. Documenter les coûts récurrents et choisir une technologie que l'équipe peut soutenir.</p>",seo_title:"Comment définir le périmètre d'un site web utile pour petite entreprise | Italy Developers",seo_description:"Une façon pratique de choisir pages, preuves et flux de travail sans promettre des fonctionnalités que l'équipe ne peut pas maintenir.",cta:"Discutons d'un projet concret"},
        Translated{slug:"small-business-website-scope",lang:"pt",title:"Como definir o escopo de um site útil para pequenas empresas",eyebrow:"Planejamento de projeto",summary:"Uma forma prática de escolher páginas, provas e fluxos de trabalho sem prometer recursos que a equipe não pode manter.",glance:"",body:"<p class=\"lead\">Comece com a decisão do cliente e a ação que a empresa pode cumprir de forma confiável.</p><h2>Mapeie as perguntas essenciais</h2><p>Para quem é o serviço? Que problema ele resolve? Onde está disponível? Que evidências constroem confiança? O que acontece depois do contato?</p><h2>Priorize o núcleo funcional</h2><p>Lance as páginas de serviço mais fortes, trabalho real, informações institucionais e um caminho de solicitação confiável antes da personalização avançada ou automação.</p><h2>Mantenha a propriedade clara</h2><p>A empresa deve controlar seu domínio, conteúdo, contas e dados. Documente custos contínuos e escolha tecnologia que a equipe possa suportar.</p>",seo_title:"Como definir o escopo de um site útil para pequenas empresas | Italy Developers",seo_description:"Uma forma prática de escolher páginas, provas e fluxos de trabalho sem prometer recursos que a equipe não pode manter.",cta:"Vamos falar sobre um projeto concreto"},
    ];
    apply_translations(db, "blog", &rows).await
}

async fn apply_project_proof_v9(db: &Database, now: DateTime) -> Result<(), AppError> {
    content(db).delete_many(doc! {"kind":"testimonial"}).await?;
    let proof = [
        ("italy-developers-proof","Italy Developers CMS","Rust · Actix · MongoDB","Production-ready CMS with role-based editing, uploads, SEO, leads, nested comments, likes and Docker deployment.","/static/images/small-business-websites.png","italy-developers-cms"),
        ("doappointment-proof","DoAppointment","Scheduling · Profiles · Availability","Appointment platform capability covering professional profiles, working hours, customer accounts and booking workflows.","/media/covers/work/doappointment-platform.svg","doappointment-platform"),
        ("storemate-proof","StoreMate","CRM · Inventory · APIs","Operational backend with authentication, OTP, products, stock, suppliers, alerts, background jobs and documented APIs.","/static/images/workflow-automation.png","storemate-crm-inventory"),
        ("drf-shapeless-proof","DRF Shapeless Serializers","Open source · Python · Django REST","Published package for runtime fields, renaming, conditional data and deeply nested serializer configuration.","/static/images/digital-strategy.png","drf-shapeless-serializers"),
        ("jgob-proof","JGOB Platform","Community · Commerce · Payments","Content, causes, volunteers, product catalogue, cart, checkout and payment integration in one Django platform.","/static/images/lean-ecommerce.png","jgob-commerce-community"),
        ("pet-care-proof","Pet Care AI","Upcoming · Responsible AI","Owner-scoped pet profiles and probabilistic dog audio analysis designed with visible uncertainty and clear safety boundaries.","/media/covers/work/pet-care-ai-upcoming.svg","pet-care-ai-upcoming")
    ];
    for (order, (slug, title, eyebrow, summary, image, work_slug)) in proof.into_iter().enumerate() {
        let item = ContentItem { id:None, kind:"testimonial".into(), slug:slug.into(), lang:"en".into(), title:title.into(), eyebrow:eyebrow.into(), summary:summary.into(), glance:String::new(), body:"Verified portfolio evidence. No client quote or performance claim is implied.".into(), image:image.into(), image_alt:format!("{title} project preview"), seo_title:String::new(), seo_description:String::new(), keywords:String::new(), cta:"View the work".into(), link:format!("/work/{work_slug}"), featured:true, published:true, order:order as i32, created_at:now, updated_at:now };
        content(db).replace_one(doc! {"kind":"testimonial","slug":slug,"lang":"en"}, item).upsert(true).await?;
    }
    Ok(())
}

/// Keeps, per slug, the entry matching `lang` if present, else the English one.
/// Input must already be sorted by (order, created_at) — order is preserved.
fn prefer_lang(items: Vec<ContentItem>, lang: &str) -> Vec<ContentItem> {
    let mut by_slug: std::collections::HashMap<String, ContentItem> = std::collections::HashMap::new();
    for item in items {
        match by_slug.get(&item.slug) {
            Some(existing) if existing.lang == lang => {}
            _ => {
                by_slug.insert(item.slug.clone(), item);
            }
        }
    }
    let mut out: Vec<ContentItem> = by_slug.into_values().collect();
    out.sort_by(|a, b| a.order.cmp(&b.order).then(b.created_at.cmp(&a.created_at)));
    out
}
async fn list_kind(db: &Database, kind: &str, lang: &str) -> Result<Vec<ContentItem>, AppError> {
    ensure_seed(db).await?;
    let langs: Vec<&str> = if lang == "en" { vec!["en"] } else { vec![lang, "en"] };
    let items: Vec<ContentItem> = content(db)
        .find(doc! {"kind":kind,"published":true,"lang":{"$in":langs}})
        .sort(doc! {"order":1,"created_at":-1})
        .await?
        .try_collect()
        .await?;
    Ok(prefer_lang(items, lang))
}
async fn list_home_kind(
    db: &Database,
    kind: &str,
    lang: &str,
    enabled: bool,
    limit: i64,
) -> Result<Vec<ContentItem>, AppError> {
    if !enabled || limit <= 0 {
        return Ok(Vec::new());
    }
    ensure_seed(db).await?;
    let langs: Vec<&str> = if lang == "en" { vec!["en"] } else { vec![lang, "en"] };
    let items: Vec<ContentItem> = content(db)
        .find(doc! {"kind":kind,"published":true,"featured":true,"lang":{"$in":langs}})
        .sort(doc! {"order":1,"created_at":-1})
        .await?
        .try_collect()
        .await?;
    let mut merged = prefer_lang(items, lang);
    merged.truncate(limit.clamp(1, 24) as usize);
    Ok(merged)
}

/// Every "Published + Show on home" item is a *candidate* for its home-page section,
/// but each section only renders its top N by Display order (its configured limit).
/// Toggling "Show on home" on a lower-priority item silently does nothing if higher-
/// priority items already fill the section — this computes which candidates lose out,
/// so the admin table can explain why an item isn't actually appearing.
async fn hidden_by_home_limit(
    db: &Database,
    settings: &HomeSettings,
) -> Result<HashSet<String>, AppError> {
    async fn overflow(
        db: &Database,
        filter: Document,
        enabled: bool,
        limit: i64,
    ) -> Result<Vec<String>, AppError> {
        if !enabled {
            return Ok(Vec::new());
        }
        let mut all: Vec<ContentItem> = content(db).find(filter).await?.try_collect().await?;
        all.sort_by_key(|item| item.order);
        Ok(all
            .into_iter()
            .skip(limit.clamp(0, 24) as usize)
            .filter_map(|item| item.id.map(|id| id.to_hex()))
            .collect())
    }
    let mut hidden = HashSet::new();
    hidden.extend(
        overflow(
            db,
            doc! {"kind":"service","published":true,"featured":true,"lang":"en"},
            settings.show_services,
            settings.service_limit,
        )
        .await?,
    );
    hidden.extend(
        overflow(
            db,
            doc! {"kind":"work","published":true,"featured":true,"lang":"en"},
            settings.show_work,
            settings.work_limit,
        )
        .await?,
    );
    hidden.extend(
        overflow(
            db,
            doc! {"kind":"testimonial","published":true,"featured":true,"lang":"en"},
            settings.show_testimonials,
            settings.testimonial_limit,
        )
        .await?,
    );
    hidden.extend(
        overflow(
            db,
            doc! {"kind":{"$in":["insight","blog"]},"published":true,"featured":true,"lang":"en"},
            settings.show_insights,
            settings.insight_limit,
        )
        .await?,
    );
    Ok(hidden)
}
async fn one(db: &Database, kind: &str, slug: &str, lang: &str) -> Result<ContentItem, AppError> {
    ensure_seed(db).await?;
    if let Some(item) = content(db)
        .find_one(doc! {"kind":kind,"slug":slug,"lang":lang,"published":true})
        .await?
    {
        return Ok(item);
    }
    if lang != "en" {
        if let Some(item) = content(db)
            .find_one(doc! {"kind":kind,"slug":slug,"lang":"en","published":true})
            .await?
        {
            return Ok(item);
        }
    }
    Err(AppError::NotFound)
}
async fn home_page(db: &Database, lang: &str) -> Result<HttpResponse, AppError> {
    let settings = home_settings(db).await?;
    let mut insights = list_home_kind(db, "insight", lang, settings.show_insights, settings.insight_limit).await?;
    insights.extend(list_home_kind(db, "blog", lang, settings.show_insights, settings.insight_limit).await?);
    insights.sort_by_key(|item| item.order);
    insights.truncate(settings.insight_limit.clamp(0, 24) as usize);
    let t = i18n::ui(lang);
    html(HomeTemplate {
        services: list_home_kind(db, "service", lang, settings.show_services, settings.service_limit).await?,
        work: list_home_kind(db, "work", lang, settings.show_work, settings.work_limit).await?,
        insights,
        testimonials: list_home_kind(db, "testimonial", lang, settings.show_testimonials, settings.testimonial_limit).await?,
        lang: t.lang.into(),
        prefix: i18n::prefix_for(t.lang),
        path_no_prefix: "/".into(),
        t,
    })
}

/// For unprefixed pages (contact/login/register) that still show localized
/// chrome based on the visitor's last-chosen language, if any.
fn lang_from_cookie(req: &HttpRequest) -> &'static str {
    req.cookie("lang")
        .map(|c| i18n::normalize(c.value()))
        .unwrap_or("en")
}
fn lang_cookie(lang: &str) -> actix_web::cookie::Cookie<'static> {
    actix_web::cookie::Cookie::build("lang", lang.to_string())
        .path("/")
        .max_age(actix_web::cookie::time::Duration::days(365))
        .finish()
}

/// `/` — detects a language from the `lang` cookie or `Accept-Language`
/// header and 302s to `/{lang}/`; otherwise serves English directly. Deep
/// links to interior pages never force-redirect like this.
async fn root(req: HttpRequest, db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let redirect_lang: Option<&'static str> = match req.cookie("lang") {
        Some(c) if i18n::is_supported_locale(c.value()) => Some(i18n::normalize(c.value())),
        Some(_) => None,
        None => req
            .headers()
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .and_then(i18n::best_supported_from_accept_language),
    };
    if let Some(lang) = redirect_lang {
        return Ok(HttpResponse::Found()
            .append_header((header::LOCATION, format!("/{lang}/")))
            .cookie(lang_cookie(lang))
            .finish());
    }
    let mut resp = home_page(&db, "en").await?;
    let _ = resp.add_cookie(&lang_cookie("en"));
    Ok(resp)
}
async fn home_localized(db: web::Data<Database>, path: web::Path<String>) -> Result<HttpResponse, AppError> {
    let lang = i18n::normalize(&path);
    let mut resp = home_page(&db, lang).await?;
    let _ = resp.add_cookie(&lang_cookie(lang));
    Ok(resp)
}

async fn collection_page(
    db: &Database,
    kind: &str,
    lang: &str,
    copy: i18n::CollectionCopy,
    path: &str,
) -> Result<HttpResponse, AppError> {
    let t = i18n::ui(lang);
    let prefix = i18n::prefix_for(lang);
    html(CollectionTemplate {
        title: copy.title.into(),
        description: copy.description.into(),
        canonical: format!("{prefix}{path}"),
        eyebrow: copy.eyebrow.into(),
        heading: copy.heading.into(),
        intro: copy.intro.into(),
        kind: kind.into(),
        items: list_kind(db, kind, lang).await?,
        lang: t.lang.into(),
        path_no_prefix: path.into(),
        prefix,
        t,
    })
}
async fn services(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db, "service", "en", i18n::ui("en").services, "/services").await
}
async fn services_i18n(db: web::Data<Database>, lang: web::Path<String>) -> Result<HttpResponse, AppError> {
    let lang = i18n::normalize(&lang);
    collection_page(&db, "service", lang, i18n::ui(lang).services, "/services").await
}
async fn work(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db, "work", "en", i18n::ui("en").work, "/work").await
}
async fn work_i18n(db: web::Data<Database>, lang: web::Path<String>) -> Result<HttpResponse, AppError> {
    let lang = i18n::normalize(&lang);
    collection_page(&db, "work", lang, i18n::ui(lang).work, "/work").await
}
async fn insights(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db, "insight", "en", i18n::ui("en").insights, "/insights").await
}
async fn insights_i18n(db: web::Data<Database>, lang: web::Path<String>) -> Result<HttpResponse, AppError> {
    let lang = i18n::normalize(&lang);
    collection_page(&db, "insight", lang, i18n::ui(lang).insights, "/insights").await
}
async fn blog(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db, "blog", "en", i18n::ui("en").blog, "/blog").await
}
async fn blog_i18n(db: web::Data<Database>, lang: web::Path<String>) -> Result<HttpResponse, AppError> {
    let lang = i18n::normalize(&lang);
    collection_page(&db, "blog", lang, i18n::ui(lang).blog, "/blog").await
}
async fn tech_stack(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db, "tech", "en", i18n::ui("en").tech, "/tech-stack").await
}
async fn tech_stack_i18n(db: web::Data<Database>, lang: web::Path<String>) -> Result<HttpResponse, AppError> {
    let lang = i18n::normalize(&lang);
    collection_page(&db, "tech", lang, i18n::ui(lang).tech, "/tech-stack").await
}
async fn about(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db, "about", "en", i18n::ui("en").about, "/about").await
}
async fn about_i18n(db: web::Data<Database>, lang: web::Path<String>) -> Result<HttpResponse, AppError> {
    let lang = i18n::normalize(&lang);
    collection_page(&db, "about", lang, i18n::ui(lang).about, "/about").await
}
async fn detail(
    db: &Database,
    session: &Session,
    kind: &str,
    slug: &str,
    lang: &str,
    path: &str,
    schema: &str,
) -> Result<HttpResponse, AppError> {
    let item = one(db, kind, slug, lang).await?;
    let comments = if kind == "blog" { ensure_official_starter(db, slug).await?; comment_views(db, slug).await? } else { Vec::new() };
    let post_likes = if kind == "blog" {
        blog_reactions(db).count_documents(doc! {"target":format!("post:{slug}")}).await? as i64
    } else { 0 };
    let t = i18n::ui(lang);
    let prefix = i18n::prefix_for(lang);
    html(DetailTemplate {
        canonical: format!("{prefix}{path}/{}", item.slug),
        path_no_prefix: format!("{path}/{}", item.slug),
        lang: t.lang.into(),
        prefix,
        item,
        schema_type: schema.into(),
        csrf: csrf(session)?,
        comments,
        post_likes,
        authenticated: authenticated(session),
        viewer_name: session.get::<String>("name").ok().flatten().unwrap_or_default(),
        t,
    })
}
async fn service_detail(session: Session, db: web::Data<Database>, slug: web::Path<String>) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "service", &slug, "en", "/services", "Service").await
}
async fn service_detail_i18n(session: Session, db: web::Data<Database>, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    let (lang, slug) = path.into_inner();
    detail(&db, &session, "service", &slug, i18n::normalize(&lang), "/services", "Service").await
}
async fn work_detail(session: Session, db: web::Data<Database>, slug: web::Path<String>) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "work", &slug, "en", "/work", "CreativeWork").await
}
async fn work_detail_i18n(session: Session, db: web::Data<Database>, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    let (lang, slug) = path.into_inner();
    detail(&db, &session, "work", &slug, i18n::normalize(&lang), "/work", "CreativeWork").await
}
async fn insight_detail(session: Session, db: web::Data<Database>, slug: web::Path<String>) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "insight", &slug, "en", "/insights", "Article").await
}
async fn insight_detail_i18n(session: Session, db: web::Data<Database>, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    let (lang, slug) = path.into_inner();
    detail(&db, &session, "insight", &slug, i18n::normalize(&lang), "/insights", "Article").await
}
async fn blog_detail(session: Session, db: web::Data<Database>, slug: web::Path<String>) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "blog", &slug, "en", "/blog", "BlogPosting").await
}
async fn blog_detail_i18n(session: Session, db: web::Data<Database>, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    let (lang, slug) = path.into_inner();
    detail(&db, &session, "blog", &slug, i18n::normalize(&lang), "/blog", "BlogPosting").await
}
async fn tech_detail(session: Session, db: web::Data<Database>, slug: web::Path<String>) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "tech", &slug, "en", "/tech-stack", "TechArticle").await
}
async fn tech_detail_i18n(session: Session, db: web::Data<Database>, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    let (lang, slug) = path.into_inner();
    detail(&db, &session, "tech", &slug, i18n::normalize(&lang), "/tech-stack", "TechArticle").await
}
async fn about_detail(session: Session, db: web::Data<Database>, slug: web::Path<String>) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "about", &slug, "en", "/about", "AboutPage").await
}
async fn about_detail_i18n(session: Session, db: web::Data<Database>, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    let (lang, slug) = path.into_inner();
    detail(&db, &session, "about", &slug, i18n::normalize(&lang), "/about", "AboutPage").await
}

async fn comment_views(db: &Database, slug: &str) -> Result<Vec<CommentView>, AppError> {
    let rows: Vec<BlogComment> = blog_comments(db)
        .find(doc! {"post_slug":slug,"published":true})
        .sort(doc! {"created_at":1})
        .await?
        .try_collect()
        .await?;
    fn append(parent: Option<ObjectId>, depth: usize, rows: &[BlogComment], out: &mut Vec<CommentView>) {
        if depth > 6 { return; }
        for row in rows.iter().filter(|row| row.parent_id == parent) {
            let Some(id) = row.id else { continue };
            out.push(CommentView { id:id.to_hex(), author:row.author.clone(), body:row.body.clone(), likes:row.likes, depth });
            append(Some(id), depth + 1, rows, out);
        }
    }
    let mut result = Vec::new();
    append(None, 0, &rows, &mut result);
    Ok(result)
}

async fn ensure_official_starter(db: &Database, slug: &str) -> Result<(), AppError> {
    if blog_comments(db).find_one(doc! {"post_slug":slug,"user_id":"system:italy-developers"}).await?.is_none() {
        blog_comments(db).insert_one(BlogComment {
            id:None,
            post_slug:slug.into(),
            parent_id:None,
            user_id:"system:italy-developers".into(),
            author_email:"hello@italydevelopers.com".into(),
            author:"Italy Developers".into(),
            body:"What would you like us to explain, test or expand in this guide? Share your situation and we’ll keep the discussion practical.".into(),
            likes:0,
            published:true,
            created_at:DateTime::now(),
        }).await?;
    }
    Ok(())
}

fn authenticated(session: &Session) -> bool {
    session.get::<String>("user_id").ok().flatten().filter(|id| !id.is_empty()).is_some()
}

fn member_id(session: &Session) -> Result<String, AppError> {
    session.get::<String>("user_id").map_err(|_| AppError::Forbidden)?.filter(|id| !id.is_empty()).ok_or(AppError::Forbidden)
}

fn valid_csrf(session: &Session, received: &str) -> Result<(), AppError> {
    let expected = session.get::<String>("csrf").map_err(|_| AppError::Forbidden)?.ok_or(AppError::Forbidden)?;
    if !security::csrf_valid(&expected, received) { return Err(AppError::Forbidden); }
    Ok(())
}

#[derive(Deserialize)]
struct CommentForm { body: String, parent_id: Option<String>, csrf: String }

async fn add_blog_comment(req: HttpRequest, session: Session, db: web::Data<Database>, slug: web::Path<String>, form: web::Form<CommentForm>) -> Result<HttpResponse, AppError> {
    valid_csrf(&session, &form.csrf)?;
    let user_id = member_id(&session)?;
    one(&db, "blog", &slug, "en").await?;
    let author = session.get::<String>("name").map_err(|_| AppError::Forbidden)?.unwrap_or_default();
    let body = form.body.trim();
    if !(2..=80).contains(&author.chars().count()) || !(3..=2000).contains(&body.chars().count()) { return Err(AppError::BadRequest); }
    let parent_id = form.parent_id.as_deref().filter(|v| !v.is_empty()).map(ObjectId::parse_str).transpose().map_err(|_| AppError::BadRequest)?;
    if let Some(parent) = parent_id {
        if blog_comments(&db).find_one(doc! {"_id":parent,"post_slug":slug.as_str(),"published":true}).await?.is_none() { return Err(AppError::BadRequest); }
    }
    let author_email = session.get::<String>("email").map_err(|_| AppError::Forbidden)?.unwrap_or_default();
    let inserted = blog_comments(&db).insert_one(BlogComment { id:None, post_slug:slug.to_string(), parent_id, user_id, author_email, author:author.clone(), body:body.into(), likes:0, published:true, created_at:DateTime::now() }).await?;
    if wants_json(&req) {
        let id = inserted.inserted_id.as_object_id().map(|value| value.to_hex()).unwrap_or_default();
        return Ok(HttpResponse::Created().json(serde_json::json!({"id":id,"author":author,"body":body,"parent_id":parent_id.map(|value| value.to_hex()),"likes":0})));
    }
    Ok(HttpResponse::SeeOther().insert_header((header::LOCATION, format!("/blog/{}#discussion", slug))).finish())
}

#[derive(Deserialize)]
struct LikeForm { csrf: String }

async fn toggle_reaction(session: &Session, db: &Database, target: String) -> Result<bool, AppError> {
    let visitor = member_id(session)?;
    if let Some(existing) = blog_reactions(db).find_one(doc! {"target":&target,"visitor":&visitor}).await? {
        if let Some(id) = existing.id { blog_reactions(db).delete_one(doc! {"_id":id}).await?; }
        Ok(false)
    } else {
        blog_reactions(db).insert_one(BlogReaction { id:None, target, visitor, created_at:DateTime::now() }).await?;
        Ok(true)
    }
}

fn wants_json(req: &HttpRequest) -> bool { req.headers().get(header::ACCEPT).and_then(|v| v.to_str().ok()).map(|v| v.contains("application/json")).unwrap_or(false) }

async fn toggle_blog_like(req: HttpRequest, session: Session, db: web::Data<Database>, slug: web::Path<String>, form: web::Form<LikeForm>) -> Result<HttpResponse, AppError> {
    valid_csrf(&session, &form.csrf)?;
    one(&db, "blog", &slug, "en").await?;
    let active = toggle_reaction(&session, &db, format!("post:{}", slug)).await?;
    let count = blog_reactions(&db).count_documents(doc! {"target":format!("post:{}", slug)}).await?;
    if wants_json(&req) { return Ok(HttpResponse::Ok().json(serde_json::json!({"active":active,"count":count}))); }
    Ok(HttpResponse::SeeOther().insert_header((header::LOCATION, format!("/blog/{}#discussion", slug))).finish())
}

async fn toggle_comment_like(req: HttpRequest, session: Session, db: web::Data<Database>, path: web::Path<(String,String)>, form: web::Form<LikeForm>) -> Result<HttpResponse, AppError> {
    valid_csrf(&session, &form.csrf)?;
    let (slug, id) = path.into_inner();
    let oid = ObjectId::parse_str(&id).map_err(|_| AppError::BadRequest)?;
    if blog_comments(&db).find_one(doc! {"_id":oid,"post_slug":&slug,"published":true}).await?.is_none() { return Err(AppError::NotFound); }
    let added = toggle_reaction(&session, &db, format!("comment:{id}")).await?;
    let delta = if added { 1 } else { -1 };
    blog_comments(&db).update_one(doc! {"_id":oid}, doc! {"$inc":{"likes":delta}}).await?;
    let count = blog_comments(&db).find_one(doc! {"_id":oid}).await?.map(|comment| comment.likes).unwrap_or(0);
    if wants_json(&req) { return Ok(HttpResponse::Ok().json(serde_json::json!({"active":added,"count":count}))); }
    Ok(HttpResponse::SeeOther().insert_header((header::LOCATION, format!("/blog/{slug}#comment-{id}"))).finish())
}

fn csrf(session: &Session) -> Result<String, AppError> {
    let token = session
        .get::<String>("csrf")
        .map_err(|_| AppError::BadRequest)?
        .unwrap_or_else(|| {
            format!(
                "{:x}{:x}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )
        });
    session
        .insert("csrf", &token)
        .map_err(|_| AppError::BadRequest)?;
    Ok(token)
}
async fn contact_page(req: HttpRequest, session: Session) -> Result<HttpResponse, AppError> {
    let lang = lang_from_cookie(&req);
    html(ContactTemplate {
        csrf: csrf(&session)?,
        success: false,
        lang: lang.into(),
        prefix: String::new(),
        path_no_prefix: "/contact".into(),
        t: i18n::ui(lang),
    })
}
#[derive(Deserialize, Validate)]
struct ContactForm {
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
async fn submit_contact(
    req: HttpRequest,
    session: Session,
    db: web::Data<Database>,
    form: web::Form<ContactForm>,
) -> Result<HttpResponse, AppError> {
    form.validate().map_err(|_| AppError::BadRequest)?;
    let expected = session
        .get::<String>("csrf")
        .map_err(|_| AppError::Forbidden)?
        .ok_or(AppError::Forbidden)?;
    if !security::csrf_valid(&expected, &form.csrf) {
        return Err(AppError::Forbidden);
    };
    leads(&db)
        .insert_one(Lead {
            id: None,
            name: form.name.trim().into(),
            email: form.email.trim().to_lowercase(),
            company: form.company.clone().unwrap_or_default(),
            service: form.service.clone().unwrap_or_default(),
            message: form.message.trim().into(),
            status: "new".into(),
            created_at: DateTime::now(),
        })
        .await?;
    session.remove("csrf");
    let lang = lang_from_cookie(&req);
    html(ContactTemplate {
        csrf: String::new(),
        success: true,
        lang: lang.into(),
        prefix: String::new(),
        path_no_prefix: "/contact".into(),
        t: i18n::ui(lang),
    })
}

fn role(session: &Session) -> String {
    session
        .get::<String>("role")
        .ok()
        .flatten()
        .unwrap_or_default()
}
fn can_edit(session: &Session) -> bool {
    matches!(role(session).as_str(), "superuser" | "admin" | "staff")
}
fn can_manage(session: &Session) -> bool {
    matches!(role(session).as_str(), "superuser" | "admin")
}
async fn admin_login() -> Result<HttpResponse, AppError> {
    html(AdminLoginTemplate {
        email: String::new(),
        email_error: String::new(),
        password_error: String::new(),
        form_error: String::new(),
    })
}
#[derive(Deserialize)]
struct LoginForm {
    email: String,
    password: String,
}
async fn admin_auth(
    session: Session,
    db: web::Data<Database>,
    form: web::Form<LoginForm>,
) -> Result<HttpResponse, AppError> {
    let email = form.email.trim().to_lowercase();
    let mut email_error = String::new();
    let mut password_error = String::new();
    if !email.contains('@') {
        email_error = "Enter a valid email address.".into()
    }
    if form.password.is_empty() {
        password_error = "Password is required.".into()
    }
    if !email_error.is_empty() || !password_error.is_empty() {
        return Ok(HttpResponse::UnprocessableEntity()
            .content_type("text/html; charset=utf-8")
            .body(
                AdminLoginTemplate {
                    email,
                    email_error,
                    password_error,
                    form_error: String::new(),
                }
                .render()?,
            ));
    }
    let user = users(&db)
        .find_one(doc! {"email":&email,"active":true})
        .await?;
    if let Some(user) = user {
        if bcrypt::verify(&form.password, &user.password_hash).unwrap_or(false) {
            session.renew();
            session
                .insert("user_id", user.id.map(|v| v.to_hex()).unwrap_or_default())
                .map_err(|_| AppError::BadRequest)?;
            session
                .insert("email", &user.email)
                .map_err(|_| AppError::BadRequest)?;
            session.insert("name", if user.name.is_empty() { user.email.split('@').next().unwrap_or("Member") } else { &user.name }).map_err(|_| AppError::BadRequest)?;
            session
                .insert("role", &user.role)
                .map_err(|_| AppError::BadRequest)?;
            return Ok(HttpResponse::SeeOther()
                .insert_header((header::LOCATION, "/admin?toast=welcome"))
                .finish());
        }
    }
    Ok(HttpResponse::Unauthorized()
        .content_type("text/html; charset=utf-8")
        .body(
            AdminLoginTemplate {
                email,
                email_error: String::new(),
                password_error: String::new(),
                form_error: "Email or password is incorrect.".into(),
            }
            .render()?,
        ))
}
async fn admin_logout(session: Session) -> HttpResponse {
    session.purge();
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/"))
        .finish()
}

#[derive(Deserialize, Default)]
struct MemberAuthQuery { next: Option<String> }

#[derive(Deserialize)]
struct MemberLoginForm { email: String, password: String, next: String, csrf: String }

#[derive(Deserialize)]
struct MemberRegisterForm { name: String, email: String, password: String, next: String, csrf: String }

fn safe_next(value: &str) -> String {
    if value.starts_with('/') && !value.starts_with("//") { value.into() } else { "/".into() }
}

#[allow(clippy::too_many_arguments)]
fn member_auth_template(req: &HttpRequest, register: bool, next: String, error: String, csrf: String) -> MemberAuthTemplate {
    let lang = lang_from_cookie(req);
    let path = if register { "/register" } else { "/login" };
    MemberAuthTemplate { register, next, error, csrf, lang: lang.into(), prefix: String::new(), path_no_prefix: path.into(), t: i18n::ui(lang) }
}

async fn member_login(req: HttpRequest, session: Session, query: web::Query<MemberAuthQuery>) -> Result<HttpResponse, AppError> {
    html(member_auth_template(&req, false, safe_next(query.next.as_deref().unwrap_or("/")), String::new(), csrf(&session)?))
}

async fn member_register(req: HttpRequest, session: Session, query: web::Query<MemberAuthQuery>) -> Result<HttpResponse, AppError> {
    html(member_auth_template(&req, true, safe_next(query.next.as_deref().unwrap_or("/")), String::new(), csrf(&session)?))
}

async fn member_auth(req: HttpRequest, session: Session, db: web::Data<Database>, form: web::Form<MemberLoginForm>) -> Result<HttpResponse, AppError> {
    valid_csrf(&session, &form.csrf)?;
    let email = form.email.trim().to_lowercase();
    if let Some(user) = users(&db).find_one(doc! {"email":&email,"active":true}).await? {
        if bcrypt::verify(&form.password, &user.password_hash).unwrap_or(false) {
            session.renew();
            session.insert("user_id", user.id.map(|id| id.to_hex()).unwrap_or_default()).map_err(|_| AppError::BadRequest)?;
            session.insert("email", &user.email).map_err(|_| AppError::BadRequest)?;
            session.insert("name", if user.name.is_empty() { user.email.split('@').next().unwrap_or("Member") } else { &user.name }).map_err(|_| AppError::BadRequest)?;
            session.insert("role", &user.role).map_err(|_| AppError::BadRequest)?;
            return Ok(HttpResponse::SeeOther().insert_header((header::LOCATION, safe_next(&form.next))).finish());
        }
    }
    Ok(HttpResponse::Unauthorized().content_type("text/html; charset=utf-8").body(member_auth_template(&req, false, safe_next(&form.next), "Email or password is incorrect.".into(), csrf(&session)?).render()?))
}

async fn member_create(req: HttpRequest, session: Session, db: web::Data<Database>, form: web::Form<MemberRegisterForm>) -> Result<HttpResponse, AppError> {
    valid_csrf(&session, &form.csrf)?;
    let name = form.name.trim();
    let email = form.email.trim().to_lowercase();
    if !(2..=80).contains(&name.chars().count()) || !email.contains('@') || email.len() > 254 || form.password.len() < 12 {
        return Ok(HttpResponse::UnprocessableEntity().content_type("text/html; charset=utf-8").body(member_auth_template(&req, true, safe_next(&form.next), "Use your real name, a valid email and a password of at least 12 characters.".into(), csrf(&session)?).render()?));
    }
    if users(&db).count_documents(doc! {"email":&email}).await? > 0 {
        return Ok(HttpResponse::Conflict().content_type("text/html; charset=utf-8").body(member_auth_template(&req, true, safe_next(&form.next), "An account already exists for this email.".into(), csrf(&session)?).render()?));
    }
    let inserted = users(&db).insert_one(AdminUser { id:None, name:name.into(), email:email.clone(), password_hash:bcrypt::hash(&form.password, bcrypt::DEFAULT_COST).map_err(|_| AppError::BadRequest)?, role:"member".into(), active:true, created_at:DateTime::now() }).await?;
    session.renew();
    session.insert("user_id", inserted.inserted_id.as_object_id().map(|id| id.to_hex()).unwrap_or_default()).map_err(|_| AppError::BadRequest)?;
    session.insert("email", email).map_err(|_| AppError::BadRequest)?;
    session.insert("name", name).map_err(|_| AppError::BadRequest)?;
    session.insert("role", "member").map_err(|_| AppError::BadRequest)?;
    Ok(HttpResponse::SeeOther().insert_header((header::LOCATION, safe_next(&form.next))).finish())
}

async fn member_logout(session: Session) -> HttpResponse { session.purge(); HttpResponse::SeeOther().insert_header((header::LOCATION, "/")).finish() }

async fn social_redirect(db: web::Data<Database>, platform: web::Path<String>) -> Result<HttpResponse, AppError> {
    let settings = home_settings(&db).await?;
    let url = match platform.as_str() { "github" => settings.github_url, "linkedin" => settings.linkedin_url, "instagram" => settings.instagram_url, "youtube" => settings.youtube_url, _ => return Err(AppError::NotFound) };
    if !(url.starts_with("https://") || url.starts_with("http://")) { return Err(AppError::NotFound); }
    Ok(HttpResponse::TemporaryRedirect().insert_header((header::LOCATION, url)).finish())
}

async fn social_links(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let settings = home_settings(&db).await?;
    let mut links = serde_json::Map::new();
    for (name, url) in [("github", settings.github_url), ("linkedin", settings.linkedin_url), ("instagram", settings.instagram_url), ("youtube", settings.youtube_url)] {
        if url.starts_with("https://") || url.starts_with("http://") { links.insert(name.into(), serde_json::Value::String(format!("/social/{name}"))); }
    }
    Ok(HttpResponse::Ok().insert_header((header::CACHE_CONTROL, "no-cache")).json(links))
}
#[derive(Deserialize, Default)]
struct AdminQuery {
    toast: Option<String>,
}
async fn admin_dashboard(
    session: Session,
    db: web::Data<Database>,
    query: web::Query<AdminQuery>,
) -> Result<HttpResponse, AppError> {
    if !can_edit(&session) {
        return Ok(HttpResponse::SeeOther()
            .insert_header((header::LOCATION, "/admin/login"))
            .finish());
    }
    ensure_seed(&db).await?;
    let items = content(&db)
        .find(doc! {})
        .sort(doc! {"kind":1,"order":1})
        .await?
        .try_collect()
        .await?;
    let enquiries = if can_manage(&session) {
        leads(&db)
            .find(doc! {})
            .sort(doc! {"created_at":-1})
            .await?
            .try_collect()
            .await?
    } else {
        Vec::new()
    };
    let home_settings_current = home_settings(&db).await?;
    let hidden_by_limit = hidden_by_home_limit(&db, &home_settings_current).await?;
    html(AdminDashboardTemplate {
        items,
        leads: enquiries,
        actor_email: session
            .get::<String>("email")
            .ok()
            .flatten()
            .unwrap_or_default(),
        role: role(&session),
        can_delete: can_manage(&session),
        toast: query.toast.clone().unwrap_or_default(),
        hidden_by_limit,
    })
}
async fn admin_new(session: Session) -> Result<HttpResponse, AppError> {
    if !can_edit(&session) {
        return Err(AppError::Forbidden);
    };
    html(AdminEditorTemplate {
        shared: ContentItem::default(),
        langs: lang_tabs_from(&[]),
        is_new: true,
        errors: EditorErrors::default(),
        can_publish: can_manage(&session),
    })
}

async fn admin_homepage(
    session: Session,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    if !can_manage(&session) {
        return Err(AppError::Forbidden);
    }
    html(AdminHomepageTemplate {
        settings: home_settings(&db).await?,
    })
}

#[derive(Deserialize)]
struct HomeSettingsForm {
    show_services: Option<String>,
    service_limit: i64,
    show_work: Option<String>,
    work_limit: i64,
    show_insights: Option<String>,
    insight_limit: i64,
    show_testimonials: Option<String>,
    testimonial_limit: i64,
    github_url: String,
    linkedin_url: String,
    instagram_url: String,
    youtube_url: String,
}

fn clean_social_url(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() || value.starts_with("https://") || value.starts_with("http://") { value.into() } else { String::new() }
}

async fn admin_homepage_save(
    session: Session,
    db: web::Data<Database>,
    form: web::Form<HomeSettingsForm>,
) -> Result<HttpResponse, AppError> {
    if !can_manage(&session) {
        return Err(AppError::Forbidden);
    }
    let settings = HomeSettings {
        key: "home".into(),
        show_services: form.show_services.is_some(),
        service_limit: form.service_limit.clamp(1, 12),
        show_work: form.show_work.is_some(),
        work_limit: form.work_limit.clamp(1, 12),
        show_insights: form.show_insights.is_some(),
        insight_limit: form.insight_limit.clamp(1, 12),
        show_testimonials: form.show_testimonials.is_some(),
        testimonial_limit: form.testimonial_limit.clamp(1, 12),
        github_url: clean_social_url(&form.github_url),
        linkedin_url: clean_social_url(&form.linkedin_url),
        instagram_url: clean_social_url(&form.instagram_url),
        youtube_url: clean_social_url(&form.youtube_url),
    };
    home_settings_collection(&db)
        .replace_one(doc! {"key":"home"}, settings)
        .upsert(true)
        .await?;
    Ok(HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/admin/homepage?toast=saved"))
        .finish())
}
async fn admin_edit(
    session: Session,
    db: web::Data<Database>,
    id: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    if !can_edit(&session) {
        return Err(AppError::Forbidden);
    };
    let oid = ObjectId::parse_str(id.as_str()).map_err(|_| AppError::BadRequest)?;
    let item = content(&db)
        .find_one(doc! {"_id":oid})
        .await?
        .ok_or(AppError::NotFound)?;
    let siblings: Vec<ContentItem> = content(&db)
        .find(doc! {"kind":&item.kind,"slug":&item.slug})
        .await?
        .try_collect()
        .await?;
    let shared = siblings
        .iter()
        .find(|s| s.lang == "en")
        .cloned()
        .unwrap_or_else(|| item.clone());
    html(AdminEditorTemplate {
        shared,
        langs: lang_tabs_from(&siblings),
        is_new: false,
        errors: EditorErrors::default(),
        can_publish: can_manage(&session),
    })
}

fn valid_image(bytes: &[u8], ext: &str) -> bool {
    match ext {
        "jpg" | "jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "webp" => bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        "gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "avif" => {
            bytes.len() > 12
                && &bytes[4..8] == b"ftyp"
                && bytes[8..12]
                    .windows(4)
                    .any(|v| v == b"avif" || v == b"avis")
        }
        _ => false,
    }
}
async fn multipart_data(
    mut payload: Multipart,
    config: &Config,
) -> Result<(std::collections::HashMap<String, String>, String), AppError> {
    let mut fields = std::collections::HashMap::new();
    let mut uploaded = String::new();
    while let Some(part) = payload.next().await {
        let mut part = part.map_err(|_| AppError::BadRequest)?;
        let name = part.name().unwrap_or("").to_string();
        let filename = part
            .content_disposition()
            .and_then(|d| d.get_filename())
            .map(sanitize_filename::sanitize);
        let mut bytes = Vec::new();
        while let Some(chunk) = part.next().await {
            let chunk = chunk.map_err(|_| AppError::BadRequest)?;
            if bytes.len() + chunk.len() > 8 * 1024 * 1024 {
                return Err(AppError::BadRequest);
            }
            bytes.extend_from_slice(&chunk)
        }
        if let Some(file) = filename.filter(|f| !f.is_empty()) {
            let ext = std::path::Path::new(&file)
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or("bin")
                .to_ascii_lowercase();
            if !valid_image(&bytes, &ext) {
                return Err(AppError::BadRequest);
            }
            let stored = format!(
                "{}-{}.{}",
                DateTime::now().timestamp_millis(),
                ObjectId::new().to_hex(),
                ext
            );
            std::fs::write(
                std::path::Path::new(&config.upload_dir).join(&stored),
                bytes,
            )?;
            uploaded = format!("/uploads/{}", stored)
        } else {
            fields.insert(
                name,
                String::from_utf8(bytes).map_err(|_| AppError::BadRequest)?,
            );
        }
    }
    Ok((fields, uploaded))
}
fn truthy(v: Option<&String>) -> bool {
    v.map(|s| s == "on" || s == "true" || s == "1")
        .unwrap_or(false)
}
fn slug_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
/// Reads the per-language fields for `code` (e.g. "it") out of the tabbed
/// editor form, namespaced as `title_it`, `body_it`, etc.
fn lang_item_from_form(f: &std::collections::HashMap<String, String>, code: &str, shared: &ContentItem) -> ContentItem {
    // English keeps the original, unsuffixed field names so the existing
    // rich editor / counters / slug-gen JS (keyed to fixed ids) keeps working
    // unmodified; other languages use simpler `field_xx` namespaced inputs.
    let suffix = if code == "en" { String::new() } else { format!("_{code}") };
    let get = |field: &str| f.get(&format!("{field}{suffix}")).cloned().unwrap_or_default();
    ContentItem {
        id: None,
        kind: shared.kind.clone(),
        slug: shared.slug.clone(),
        lang: code.into(),
        title: get("title").trim().into(),
        eyebrow: get("eyebrow"),
        summary: get("summary").trim().into(),
        glance: get("glance").trim().into(),
        body: get("body"),
        image: shared.image.clone(),
        image_alt: get("image_alt"),
        seo_title: get("seo_title").trim().into(),
        seo_description: get("seo_description").trim().into(),
        keywords: get("keywords"),
        cta: get("cta"),
        link: {
            let v = get("link");
            if v.trim().is_empty() { shared.link.clone() } else { v.trim().into() }
        },
        featured: shared.featured,
        published: shared.published,
        order: shared.order,
        created_at: default_datetime(),
        updated_at: default_datetime(),
    }
}
async fn admin_save(
    session: Session,
    db: web::Data<Database>,
    config: web::Data<Config>,
    payload: Multipart,
) -> Result<HttpResponse, AppError> {
    if !can_edit(&session) {
        return Err(AppError::Forbidden);
    };
    let (f, uploaded) = multipart_data(payload, &config).await?;
    let now = DateTime::now();
    let original_kind = f.get("original_kind").cloned().unwrap_or_default();
    let original_slug = f.get("original_slug").cloned().unwrap_or_default();
    let is_new = original_slug.is_empty();
    let existing_shared = if is_new {
        None
    } else {
        content(&db).find_one(doc! {"kind":&original_kind,"slug":&original_slug,"lang":"en"}).await?
    };
    let image = if uploaded.is_empty() {
        f.get("existing_image").cloned().unwrap_or_default()
    } else {
        uploaded
    };
    let published = if can_manage(&session) {
        truthy(f.get("published"))
    } else {
        existing_shared.as_ref().map(|v| v.published).unwrap_or(false)
    };
    let mut shared = ContentItem {
        kind: f.get("kind").cloned().unwrap_or_default(),
        slug: f.get("slug").cloned().unwrap_or_default().trim().to_lowercase(),
        image,
        featured: truthy(f.get("featured")),
        published,
        order: f.get("order").and_then(|v| v.parse().ok()).unwrap_or(0),
        ..ContentItem::default()
    };
    if shared.image.is_empty() {
        shared.image = match shared.kind.as_str() {
            "blog" | "insight" => "/static/images/generated/blog-website-scope.webp",
            "tech" => "/static/images/generated/tech-python.webp",
            "work" | "testimonial" => "/static/images/generated/work-doappointment.webp",
            "about" => "/static/images/generated/about-community.webp",
            _ => "/static/images/small-business-websites.png",
        }.into();
    }

    let mut errors = EditorErrors::default();
    if !["service", "work", "tech", "about", "insight", "blog", "testimonial"].contains(&shared.kind.as_str()) {
        errors.form = "Choose a valid website content type.".into()
    }
    if !slug_valid(&shared.slug) {
        errors.slug = "Use lowercase letters, numbers and single hyphens only.".into()
    } else if is_new
        && content(&db).count_documents(doc! {"kind":&shared.kind,"slug":&shared.slug}).await? > 0
    {
        errors.slug = "This URL slug is already in use for this content type.".into()
    }

    let mut submitted: Vec<ContentItem> = Vec::new();
    let mut lang_errors: Vec<String> = Vec::new();
    for (code, label) in LANG_TABS {
        let item = lang_item_from_form(&f, code, &shared);
        if item.title.trim().is_empty() {
            continue;
        }
        let mut msgs = Vec::new();
        if item.title.len() < 3 || item.title.len() > 140 {
            msgs.push("title must be 3-140 characters");
        }
        if item.summary.len() < 20 || item.summary.len() > 400 {
            msgs.push("summary must be 20-400 characters");
        }
        if shared.kind != "testimonial" && item.body.trim().len() < 20 {
            msgs.push("main content must be at least 20 characters");
        }
        if shared.kind != "testimonial" && (item.seo_title.len() < 20 || item.seo_title.len() > 70) {
            msgs.push("SEO title should be 20-70 characters");
        }
        if shared.kind != "testimonial" && (item.seo_description.len() < 70 || item.seo_description.len() > 160) {
            msgs.push("meta description should be 70-160 characters");
        }
        if code == "en" {
            if msgs.iter().any(|m| m.starts_with("title")) { errors.title = "Use 3-140 characters.".into(); }
            if msgs.iter().any(|m| m.starts_with("summary")) { errors.summary = "Write a useful summary between 20 and 400 characters.".into(); }
            if msgs.iter().any(|m| m.starts_with("main")) { errors.body = "Main content must contain at least 20 characters.".into(); }
            if msgs.iter().any(|m| m.starts_with("SEO title")) { errors.seo_title = "SEO title should be 20-70 characters.".into(); }
            if msgs.iter().any(|m| m.starts_with("meta")) { errors.seo_description = "SEO description should be 70-160 characters.".into(); }
        }
        if !msgs.is_empty() {
            lang_errors.push(format!("{label}: {}.", msgs.join(", ")));
        }
        submitted.push(item);
    }
    if !lang_errors.is_empty() {
        let combined = lang_errors.join(" ");
        errors.form = if errors.form.is_empty() { combined } else { format!("{} {combined}", errors.form) };
    }
    if submitted.is_empty() && errors.form.is_empty() {
        errors.form = "Fill in at least one language tab, starting with English.".into();
    }

    let has_errors = [&errors.title, &errors.slug, &errors.summary, &errors.body, &errors.seo_title, &errors.seo_description, &errors.image, &errors.form]
        .iter()
        .any(|v| !v.is_empty());
    if has_errors {
        let langs = LANG_TABS
            .iter()
            .map(|&(code, label)| {
                let item = submitted.iter().find(|s| s.lang == code).cloned().unwrap_or_else(|| ContentItem { lang: code.into(), ..ContentItem::default() });
                LangTab { code, label, item }
            })
            .collect();
        return Ok(HttpResponse::UnprocessableEntity()
            .content_type("text/html; charset=utf-8")
            .body(AdminEditorTemplate { shared, langs, is_new, errors, can_publish: can_manage(&session) }.render()?));
    }

    if !is_new && (original_kind != shared.kind || original_slug != shared.slug) {
        content(&db).delete_many(doc! {"kind":&original_kind,"slug":&original_slug}).await?;
    }
    for mut item in submitted {
        let prior = content(&db).find_one(doc! {"kind":&shared.kind,"slug":&shared.slug,"lang":&item.lang}).await?;
        item.image_alt = if item.image_alt.trim().is_empty() {
            format!("Editorial image for {}", item.title)
        } else {
            item.image_alt
        };
        item.created_at = prior.as_ref().map(|p| p.created_at).unwrap_or(now);
        item.updated_at = now;
        content(&db)
            .replace_one(doc! {"kind":&shared.kind,"slug":&shared.slug,"lang":&item.lang}, item)
            .upsert(true)
            .await?;
    }
    Ok(HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/admin?toast=saved"))
        .finish())
}
async fn admin_delete(
    session: Session,
    db: web::Data<Database>,
    id: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    if !can_manage(&session) {
        return Err(AppError::Forbidden);
    };
    let oid = ObjectId::parse_str(id.as_str()).map_err(|_| AppError::BadRequest)?;
    content(&db).delete_one(doc! {"_id":oid}).await?;
    Ok(HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/admin?toast=deleted"))
        .finish())
}

/// Inline htmx toggle for the "Published" / "Show on home" switches in the content table.
/// Flips the field server-side (ignoring whatever the checkbox posted) and returns the
/// updated switch fragment so htmx can swap it in place without a page reload.
async fn admin_toggle(
    session: Session,
    db: web::Data<Database>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    if !can_edit(&session) {
        return Err(AppError::Forbidden);
    }
    let (id, field) = path.into_inner();
    if field != "published" && field != "featured" {
        return Err(AppError::BadRequest);
    }
    if field == "published" && !can_manage(&session) {
        return Err(AppError::Forbidden);
    }
    let oid = ObjectId::parse_str(&id).map_err(|_| AppError::BadRequest)?;
    let item = content(&db).find_one(doc! {"_id":oid}).await?.ok_or(AppError::NotFound)?;
    let now = DateTime::now();
    let checked = if field == "published" {
        let value = !item.published;
        content(&db).update_one(doc! {"_id":oid}, doc! {"$set":{"published":value,"updated_at":now}}).await?;
        value
    } else {
        let value = !item.featured;
        content(&db).update_one(doc! {"_id":oid}, doc! {"$set":{"featured":value,"updated_at":now}}).await?;
        value
    };
    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(table_switch(&id, &field, checked, can_manage(&session))))
}

fn table_switch(id: &str, field: &str, checked: bool, can_manage: bool) -> String {
    let disabled = field == "published" && !can_manage;
    format!(
        r##"<label class="table-switch" id="switch-{field}-{id}"><input type="checkbox" hx-post="/admin/content/{id}/toggle/{field}" hx-target="#switch-{field}-{id}" hx-swap="outerHTML"{checked}{disabled}><i></i></label>"##,
        field = field,
        id = id,
        checked = if checked { " checked" } else { "" },
        disabled = if disabled { " disabled" } else { "" },
    )
}

#[derive(Deserialize)]
struct StatusForm {
    status: String,
}
async fn admin_lead_status(
    session: Session,
    db: web::Data<Database>,
    id: web::Path<String>,
    form: web::Form<StatusForm>,
) -> Result<HttpResponse, AppError> {
    if !can_manage(&session)
        || !["new", "contacted", "closed", "spam"].contains(&form.status.as_str())
    {
        return Err(AppError::Forbidden);
    }
    let oid = ObjectId::parse_str(id.as_str()).map_err(|_| AppError::BadRequest)?;
    leads(&db)
        .update_one(doc! {"_id":oid}, doc! {"$set":{"status":&form.status}})
        .await?;
    Ok(HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/admin?toast=lead-updated"))
        .finish())
}
async fn admin_lead_delete(
    session: Session,
    db: web::Data<Database>,
    id: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    if !can_manage(&session) {
        return Err(AppError::Forbidden);
    }
    let oid = ObjectId::parse_str(id.as_str()).map_err(|_| AppError::BadRequest)?;
    leads(&db).delete_one(doc! {"_id":oid}).await?;
    Ok(HttpResponse::SeeOther()
        .insert_header((header::LOCATION, "/admin?toast=lead-deleted"))
        .finish())
}

fn xml_text(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

async fn content_cover(db: web::Data<Database>, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    use std::hash::{Hash, Hasher};
    let (kind, slug) = path.into_inner();
    if !["service","work","tech","about","insight","blog","testimonial"].contains(&kind.as_str()) || !slug_valid(&slug) { return Err(AppError::NotFound); }
    let item = one(&db, &kind, &slug, "en").await?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{kind}:{slug}").hash(&mut hasher);
    let hash = hasher.finish();
    let palettes = [
        ("#123c2b", "#39a96b", "#ef3d45", "#f5f0e6"),
        ("#16213e", "#2f80ed", "#f2994a", "#f7f2e8"),
        ("#3a172d", "#a33c6f", "#f0a84b", "#f6eee3"),
        ("#24211d", "#767f45", "#d94a3d", "#f3eddf"),
        ("#18343b", "#2f8f9d", "#d9a441", "#f5efe5"),
        ("#2e2140", "#7356a8", "#ef6b4a", "#f5efe6"),
    ];
    let (ink, primary, accent, paper) = palettes[(hash as usize) % palettes.len()];
    let x = 120 + (hash % 260) as i32;
    let y = 90 + ((hash >> 8) % 180) as i32;
    let number = format!("{:02}", (hash % 97) + 1);
    let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 900" role="img" aria-labelledby="title desc"><title id="title">{}</title><desc id="desc">Unique editorial cover for {}</desc><rect width="1200" height="900" fill="{}"/><path d="M0 0H700L1030 900H0Z" fill="{}"/><circle cx="{}" cy="{}" r="310" fill="{}" opacity=".92"/><path d="M760 0h440v440L980 660 760 440Z" fill="{}"/><path d="M0 720h1200v180H0Z" fill="{}" opacity=".96"/><g fill="{}" font-family="Arial,Helvetica,sans-serif"><text x="64" y="78" font-size="22" font-weight="700" letter-spacing="5">ITALY DEVELOPERS · {}</text><text x="64" y="820" font-size="25" font-weight="700" letter-spacing="3">{} · {}</text></g></svg>"#, xml_text(&item.title), xml_text(&item.image_alt), paper, ink, x, y, primary, accent, paper, ink, xml_text(&kind.to_uppercase()), number, xml_text(&slug));
    Ok(HttpResponse::Ok().content_type("image/svg+xml; charset=utf-8").insert_header((header::CACHE_CONTROL,"public, max-age=3600")).body(svg))
}

async fn live() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status":"ok"}))
}
async fn ready(db: web::Data<Database>) -> HttpResponse {
    match db.run_command(doc! {"ping":1}).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"status":"ready"})),
        Err(_) => {
            HttpResponse::ServiceUnavailable().json(serde_json::json!({"status":"unavailable"}))
        }
    }
}
async fn robots(config: web::Data<Config>) -> HttpResponse {
    HttpResponse::Ok().content_type("text/plain; charset=utf-8").body(format!("User-agent: *\nAllow: /\nDisallow: /admin\nDisallow: /health/\nSitemap: {}/sitemap.xml\n",config.public_url.trim_end_matches('/')))
}
/// Emits one `<url>` per locale for `suffix` (e.g. "/services/some-slug"),
/// each carrying `hreflang` siblings to the other locales — the English
/// fallback in `one()`/`list_kind()` guarantees every variant resolves.
fn sitemap_url_block(root: &str, suffix: &str) -> String {
    const ALL: [&str; 5] = ["en", "it", "de", "fr", "pt"];
    let mut out = String::new();
    for lang in ALL {
        let loc = format!("{root}{}{suffix}", i18n::prefix_for(lang));
        out.push_str("<url><loc>");
        out.push_str(&loc);
        out.push_str("</loc>");
        for alt in ALL {
            out.push_str(&format!(
                "<xhtml:link rel=\"alternate\" hreflang=\"{alt}\" href=\"{root}{}{suffix}\"/>",
                i18n::prefix_for(alt)
            ));
        }
        out.push_str(&format!(
            "<xhtml:link rel=\"alternate\" hreflang=\"x-default\" href=\"{root}{suffix}\"/>"
        ));
        out.push_str("</url>");
    }
    out
}
async fn sitemap(
    db: web::Data<Database>,
    config: web::Data<Config>,
) -> Result<HttpResponse, AppError> {
    ensure_seed(&db).await?;
    let docs: Vec<ContentItem> = content(&db)
        .find(doc! {"published":true})
        .await?
        .try_collect()
        .await?;
    let root = config.public_url.trim_end_matches('/');
    let mut urls = String::new();
    urls.push_str(&sitemap_url_block(root, ""));
    for suffix in ["/services", "/work", "/about", "/tech-stack", "/insights", "/blog"] {
        urls.push_str(&sitemap_url_block(root, suffix));
    }
    urls.push_str(&format!("<url><loc>{root}/contact</loc></url>"));
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for item in docs {
        let base = match item.kind.as_str() {
            "service" => "services",
            "work" => "work",
            "insight" => "insights",
            "blog" => "blog",
            "tech" => "tech-stack",
            "about" => "about",
            _ => continue,
        };
        if !seen.insert((item.kind.clone(), item.slug.clone())) {
            continue;
        }
        urls.push_str(&sitemap_url_block(root, &format!("/{base}/{}", item.slug)));
    }
    Ok(HttpResponse::Ok().content_type("application/xml; charset=utf-8").body(format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">{}</urlset>",
        urls
    )))
}
