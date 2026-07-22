use actix_multipart::Multipart;
use actix_session::Session;
use actix_web::{http::header, web, HttpResponse};
use askama::Template;
use futures_util::{StreamExt, TryStreamExt};
use mongodb::{
    bson::{doc, oid::ObjectId, DateTime},
    Database,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{config::Config, error::AppError, security};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentItem {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub eyebrow: String,
    #[serde(default)]
    pub summary: String,
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
impl Default for ContentItem {
    fn default() -> Self {
        Self {
            id: None,
            kind: String::new(),
            slug: String::new(),
            title: String::new(),
            eyebrow: String::new(),
            summary: String::new(),
            body: String::new(),
            image: String::new(),
            image_alt: String::new(),
            seo_title: String::new(),
            seo_description: String::new(),
            keywords: String::new(),
            cta: String::new(),
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
struct AdminUser {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
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
}

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
}
#[derive(Template)]
#[template(path = "detail.html")]
struct DetailTemplate {
    item: ContentItem,
    canonical: String,
    schema_type: String,
}
#[derive(Template)]
#[template(path = "contact.html")]
struct ContactTemplate {
    csrf: String,
    success: bool,
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
}
#[derive(Template)]
#[template(path = "admin/editor.html")]
struct AdminEditorTemplate {
    item: ContentItem,
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
    cfg.route("/", web::get().to(home))
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
        .route("/contact", web::get().to(contact_page))
        .route("/contact", web::post().to(submit_contact))
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
    if migrations.find_one(doc! {"key":"editorial-v2"}).await?.is_some() {
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
        ("about","our-approach","Small team, useful outcomes","Direct collaboration, honest scope and technology chosen for long-term value.","<p class=\"lead\">Italy Developers is presented as a focused digital studio for Italian micro and small businesses. The promise is simple: understand the commercial job, build the useful core and leave the owner in control.</p><h2>How we work</h2><p>Discovery begins with customers, current operations, constraints and a comfortable investment range. We turn that into a written launch scope, separate assumptions and optional phases, then work in visible weekly increments.</p><div class=\"fact-grid\"><p><strong>Communication</strong><br>Direct and plain-language</p><p><strong>Delivery</strong><br>Small releases with acceptance criteria</p></div><h2>Principles behind the work</h2><ul><li>Evidence before feature volume</li><li>Accessible, responsive experiences by default</li><li>Company-controlled domains, accounts and data</li><li>Privacy and security proportional to real risk</li><li>Documentation that another professional can use</li></ul><h2>Honesty in the portfolio</h2><p>Concept work is labelled as demonstration work. We do not invent client testimonials, performance figures or partnerships. Published results should always have a source and the client’s permission.</p><h3>What a good handover includes</h3><p>Owners receive access, documentation, training, backup guidance and a clear list of recurring services. Continuing support is optional and separately scoped.</p>","Our approach","/static/images/small-business-websites.png")
    ];
    let docs = data
        .into_iter()
        .enumerate()
        .map(
            |(i, (kind, slug, title, summary, body, eyebrow, image))| ContentItem {
                id: None,
                kind: kind.into(),
                slug: slug.into(),
                title: title.into(),
                eyebrow: eyebrow.into(),
                summary: summary.into(),
                body: body.into(),
                image: image.into(),
                image_alt: format!("Editorial image for {}", title),
                seo_title: format!("{} | Italy Developers", title),
                seo_description: summary.into(),
                keywords: "piccole imprese Italia, sito web economico, sviluppo web Italia".into(),
                cta: "Request a practical proposal".into(),
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
    Ok(())
}

async fn list_kind(db: &Database, kind: &str) -> Result<Vec<ContentItem>, AppError> {
    ensure_seed(db).await?;
    Ok(content(db)
        .find(doc! {"kind":kind,"published":true})
        .sort(doc! {"order":1,"created_at":-1})
        .await?
        .try_collect()
        .await?)
}
async fn list_home_kind(
    db: &Database,
    kind: &str,
    enabled: bool,
    limit: i64,
) -> Result<Vec<ContentItem>, AppError> {
    if !enabled || limit <= 0 {
        return Ok(Vec::new());
    }
    ensure_seed(db).await?;
    Ok(content(db)
        .find(doc! {"kind":kind,"published":true,"featured":true})
        .sort(doc! {"order":1,"created_at":-1})
        .limit(limit.clamp(1, 24))
        .await?
        .try_collect()
        .await?)
}
async fn one(db: &Database, kind: &str, slug: &str) -> Result<ContentItem, AppError> {
    ensure_seed(db).await?;
    content(db)
        .find_one(doc! {"kind":kind,"slug":slug,"published":true})
        .await?
        .ok_or(AppError::NotFound)
}

async fn home(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let settings = home_settings(&db).await?;
    let mut insights = list_home_kind(
        &db,
        "insight",
        settings.show_insights,
        settings.insight_limit,
    )
    .await?;
    insights.extend(
        list_home_kind(
            &db,
            "blog",
            settings.show_insights,
            settings.insight_limit,
        )
        .await?,
    );
    insights.sort_by_key(|item| item.order);
    insights.truncate(settings.insight_limit.clamp(0, 24) as usize);
    html(HomeTemplate {
        services: list_home_kind(
            &db,
            "service",
            settings.show_services,
            settings.service_limit,
        )
        .await?,
        work: list_home_kind(&db, "work", settings.show_work, settings.work_limit).await?,
        insights,
        testimonials: list_home_kind(
            &db,
            "testimonial",
            settings.show_testimonials,
            settings.testimonial_limit,
        )
        .await?,
    })
}
async fn collection_page(
    db: &Database,
    kind: &str,
    title: &str,
    description: &str,
    path: &str,
    eyebrow: &str,
    heading: &str,
    intro: &str,
) -> Result<HttpResponse, AppError> {
    html(CollectionTemplate {
        title: title.into(),
        description: description.into(),
        canonical: path.into(),
        eyebrow: eyebrow.into(),
        heading: heading.into(),
        intro: intro.into(),
        kind: kind.into(),
        items: list_kind(db, kind).await?,
    })
}
async fn services(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db,"service","Affordable Web Services for Italian Small Businesses | Italy Developers","Practical websites, e-commerce and automation for Italian small businesses with realistic budgets.","/services","Services","Useful digital services, fairly scoped.","Start with what creates enquiries or saves time. Grow only when the evidence says it is worthwhile.").await
}
async fn work(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db,"work","Small Business Website Portfolio Italy | Italy Developers","Website and software case studies for Italian restaurants, trades and professional businesses.","/work","Selected work","Small budgets can still produce serious results.","Focused projects for local businesses, designed around trust, visibility and a clear customer action.").await
}
async fn insights(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db,"insight","Digital Guides for Italian Small Businesses | Italy Developers","Straightforward website, cost and digital strategy guides for Italian small-business owners.","/insights","Insights","Make better digital decisions.","Clear guidance for owners who need value, not technical theatre.").await
}
async fn blog(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(
        &db,
        "blog",
        "Small Business Web & SEO Blog Italy | Italy Developers",
        "Practical local SEO, website and online growth articles for small businesses in Italy.",
        "/blog",
        "The blog",
        "Practical ideas for earning attention online.",
        "Useful, jargon-free articles for Italian entrepreneurs and individual professionals.",
    )
    .await
}
async fn tech_stack(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(
        &db,
        "tech",
        "Technology Stack | Italy Developers",
        "The secure, efficient technology behind our affordable websites and software.",
        "/tech-stack",
        "Technology",
        "Modern tools, selected for value.",
        "A maintainable stack that keeps sites fast, secure and affordable to operate.",
    )
    .await
}
async fn about(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db,"about","About Italy Developers | Digital Partner for Small Business","A direct, practical web studio serving budget-conscious Italian small businesses and independent professionals.","/about","About us","Small team. Straight answers. Useful work.","We help Italian entrepreneurs build credibility and win customers without inflated agency overhead.").await
}
async fn detail(
    db: &Database,
    kind: &str,
    slug: &str,
    path: &str,
    schema: &str,
) -> Result<HttpResponse, AppError> {
    let item = one(db, kind, slug).await?;
    html(DetailTemplate {
        canonical: format!("{}/{}", path, item.slug),
        item,
        schema_type: schema.into(),
    })
}
async fn service_detail(
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, "service", &slug, "/services", "Service").await
}
async fn work_detail(
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, "work", &slug, "/work", "CreativeWork").await
}
async fn insight_detail(
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, "insight", &slug, "/insights", "Article").await
}
async fn blog_detail(
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, "blog", &slug, "/blog", "BlogPosting").await
}
async fn tech_detail(
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, "tech", &slug, "/tech-stack", "TechArticle").await
}
async fn about_detail(
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, "about", &slug, "/about", "AboutPage").await
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
async fn contact_page(session: Session) -> Result<HttpResponse, AppError> {
    html(ContactTemplate {
        csrf: csrf(&session)?,
        success: false,
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
    html(ContactTemplate {
        csrf: String::new(),
        success: true,
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
    })
}
async fn admin_new(session: Session) -> Result<HttpResponse, AppError> {
    if !can_edit(&session) {
        return Err(AppError::Forbidden);
    };
    html(AdminEditorTemplate {
        item: ContentItem::default(),
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
    html(AdminEditorTemplate {
        item,
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
    let existing = f.get("id").and_then(|v| ObjectId::parse_str(v).ok());
    let previous = if let Some(id) = existing {
        content(&db).find_one(doc! {"_id":id}).await?
    } else {
        None
    };
    let image = if uploaded.is_empty() {
        f.get("existing_image").cloned().unwrap_or_default()
    } else {
        uploaded
    };
    let published = if can_manage(&session) {
        truthy(f.get("published"))
    } else {
        previous.as_ref().map(|v| v.published).unwrap_or(false)
    };
    let item = ContentItem {
        id: existing,
        kind: f.get("kind").cloned().unwrap_or_default(),
        slug: f
            .get("slug")
            .cloned()
            .unwrap_or_default()
            .trim()
            .to_lowercase(),
        title: f.get("title").cloned().unwrap_or_default().trim().into(),
        eyebrow: f.get("eyebrow").cloned().unwrap_or_default(),
        summary: f.get("summary").cloned().unwrap_or_default().trim().into(),
        body: f.get("body").cloned().unwrap_or_default(),
        image,
        image_alt: f.get("image_alt").cloned().unwrap_or_default(),
        seo_title: f
            .get("seo_title")
            .cloned()
            .unwrap_or_default()
            .trim()
            .into(),
        seo_description: f
            .get("seo_description")
            .cloned()
            .unwrap_or_default()
            .trim()
            .into(),
        keywords: f.get("keywords").cloned().unwrap_or_default(),
        cta: f.get("cta").cloned().unwrap_or_default(),
        featured: truthy(f.get("featured")),
        published,
        order: f.get("order").and_then(|v| v.parse().ok()).unwrap_or(0),
        created_at: previous.as_ref().map(|v| v.created_at).unwrap_or(now),
        updated_at: now,
    };
    let mut errors = EditorErrors::default();
    if ![
        "service",
        "work",
        "tech",
        "about",
        "insight",
        "blog",
        "testimonial",
    ]
        .contains(&item.kind.as_str())
    {
        errors.form = "Choose a valid website content type.".into()
    }
    if item.title.len() < 3 || item.title.len() > 140 {
        errors.title = "Use 3–140 characters.".into()
    }
    if !slug_valid(&item.slug) {
        errors.slug = "Use lowercase letters, numbers and single hyphens only.".into()
    } else {
        let mut filter = doc! {"slug":&item.slug,"kind":&item.kind};
        if let Some(id) = existing {
            filter.insert("_id", doc! {"$ne":id});
        }
        if content(&db).count_documents(filter).await? > 0 {
            errors.slug = "This URL slug is already in use for this content type.".into()
        }
    }
    if item.summary.len() < 20 || item.summary.len() > 400 {
        errors.summary = "Write a useful summary between 20 and 400 characters.".into()
    }
    if item.kind != "testimonial" && item.body.trim().len() < 20 {
        errors.body = "Main content must contain at least 20 characters.".into()
    }
    if item.kind != "testimonial" && (item.seo_title.len() < 20 || item.seo_title.len() > 70) {
        errors.seo_title = "SEO title should be 20–70 characters.".into()
    }
    if item.kind != "testimonial"
        && (item.seo_description.len() < 70 || item.seo_description.len() > 160)
    {
        errors.seo_description = "SEO description should be 70–160 characters.".into()
    }
    if !item.image.is_empty() && item.image_alt.trim().len() < 5 {
        errors.image = "Add descriptive alt text for the uploaded image.".into()
    }
    let has_errors = [
        &errors.title,
        &errors.slug,
        &errors.summary,
        &errors.body,
        &errors.seo_title,
        &errors.seo_description,
        &errors.image,
        &errors.form,
    ]
    .iter()
    .any(|v| !v.is_empty());
    if has_errors {
        return Ok(HttpResponse::UnprocessableEntity()
            .content_type("text/html; charset=utf-8")
            .body(
                AdminEditorTemplate {
                    item,
                    is_new: existing.is_none(),
                    errors,
                    can_publish: can_manage(&session),
                }
                .render()?,
            ));
    }
    if let Some(id) = existing {
        content(&db).replace_one(doc! {"_id":id}, item).await?;
    } else {
        content(&db).insert_one(item).await?;
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
    let mut urls = vec![
        "/",
        "/services",
        "/work",
        "/about",
        "/tech-stack",
        "/insights",
        "/blog",
        "/contact",
    ]
    .into_iter()
    .map(|p| format!("<url><loc>{}{}</loc></url>", root, p))
    .collect::<String>();
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
        urls.push_str(&format!(
            "<url><loc>{}/{}/{}</loc></url>",
            root, base, item.slug
        ));
    }
    Ok(HttpResponse::Ok().content_type("application/xml; charset=utf-8").body(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">{}</urlset>",urls)))
}
