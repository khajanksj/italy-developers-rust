use actix_multipart::Multipart;
use actix_session::Session;
use actix_web::{http::header, web, HttpResponse};
use askama::Template;
use futures_util::{StreamExt, TryStreamExt};
use mongodb::{
    bson::{doc, oid::ObjectId, DateTime},
    options::IndexOptions,
    Database,
    IndexModel,
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
struct BlogComment {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    post_slug: String,
    parent_id: Option<ObjectId>,
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
    csrf: String,
    comments: Vec<CommentView>,
    post_likes: i64,
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
        .route("/blog/{slug}/comment", web::post().to(add_blog_comment))
        .route("/blog/{slug}/like", web::post().to(toggle_blog_like))
        .route("/blog/{slug}/comments/{id}/like", web::post().to(toggle_comment_like))
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
    if migrations.find_one(doc! {"key":"editorial-v8"}).await?.is_some() {
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
                title: title.into(),
                eyebrow: eyebrow.into(),
                summary: summary.into(),
                body: body.into(),
                image: format!("/media/covers/{kind}/{slug}.svg"),
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
    apply_editorial_v3(db, now).await?;
    apply_service_v8(db, now).await?;
    migrations.insert_one(doc! {"key":"editorial-v3","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v4","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v5","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v6","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v7","applied_at":now}).await?;
    migrations.insert_one(doc! {"key":"editorial-v8","applied_at":now}).await?;
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
        let item = ContentItem { id:None, kind:kind.into(), slug:slug.into(), title:title.into(), eyebrow:eyebrow.into(), summary:summary.into(), body:body.into(), image:format!("/media/covers/{kind}/{slug}.svg"), image_alt:format!("Editorial illustration for {title}"), seo_title:format!("{title} | Italy Developers"), seo_description:summary.into(), keywords:"Rust, Python, Django, APIs, CMS, Docker, web development".into(), cta:"Discuss a practical project".into(), featured:kind == "service" || kind == "work" || (kind == "blog" && order < 18), published:true, order:order as i32, created_at:now, updated_at:now };
        content(db).replace_one(doc! {"kind":kind,"slug":slug}, item).upsert(true).await?;
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
        let item = ContentItem { id:None, kind:"service".into(), slug:slug.into(), title:title.into(), eyebrow:eyebrow.into(), summary:summary.into(), body:body.into(), image:image.into(), image_alt:format!("Italy Developers service: {title}"), seo_title:format!("{title} | Italy Developers"), seo_description:summary.into(), keywords:"custom software Italy, application development, AI integration, websites, APIs".into(), cta:"Tell us what you want to build".into(), featured:true, published:true, order:order as i32, created_at:now, updated_at:now };
        content(db).replace_one(doc! {"kind":"service","slug":slug}, item).upsert(true).await?;
    }
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
    collection_page(&db,"service","Custom Software and Digital Product Services in Italy | Italy Developers","From websites and business systems to apps, AI and integrations: end-to-end digital product delivery for people and organisations in Italy.","/services","What we can build","Bring the problem. We will shape the right product.","Websites, commerce, custom software, mobile and desktop apps, AI features, APIs and product rescue—planned around the outcome you need, not a fixed technology menu.").await
}
async fn work(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    collection_page(&db,"work","Application and Software Portfolio | Italy Developers","Real application, API, dashboard and platform work that shows what the Italy Developers community can deliver for Italy.","/work","Community portfolio","Real products. Verified capabilities. No invented results.","Worldwide developer experience brought together for Italy’s local businesses and individuals.").await
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
    collection_page(&db,"about","About Italy Developers | Global Community Serving Italy","Italy Developers is a worldwide developer community working exclusively for Italy’s local businesses and individuals.","/about","About the community","Developers worldwide. One mission: technology for Italy.","Anyone with useful technical skills can join from anywhere. Every client project remains dedicated exclusively to people and organisations in Italy.").await
}
async fn detail(
    db: &Database,
    session: &Session,
    kind: &str,
    slug: &str,
    path: &str,
    schema: &str,
) -> Result<HttpResponse, AppError> {
    let item = one(db, kind, slug).await?;
    let comments = if kind == "blog" { comment_views(db, slug).await? } else { Vec::new() };
    let post_likes = if kind == "blog" {
        blog_reactions(db).count_documents(doc! {"target":format!("post:{slug}")}).await? as i64
    } else { 0 };
    html(DetailTemplate {
        canonical: format!("{}/{}", path, item.slug),
        item,
        schema_type: schema.into(),
        csrf: csrf(session)?,
        comments,
        post_likes,
    })
}
async fn service_detail(
    session: Session,
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "service", &slug, "/services", "Service").await
}
async fn work_detail(
    session: Session,
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "work", &slug, "/work", "CreativeWork").await
}
async fn insight_detail(
    session: Session,
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "insight", &slug, "/insights", "Article").await
}
async fn blog_detail(
    session: Session,
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "blog", &slug, "/blog", "BlogPosting").await
}
async fn tech_detail(
    session: Session,
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "tech", &slug, "/tech-stack", "TechArticle").await
}
async fn about_detail(
    session: Session,
    db: web::Data<Database>,
    slug: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    detail(&db, &session, "about", &slug, "/about", "AboutPage").await
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

fn visitor_id(session: &Session) -> Result<String, AppError> {
    let id = session.get::<String>("visitor").map_err(|_| AppError::BadRequest)?.unwrap_or_else(|| ObjectId::new().to_hex());
    session.insert("visitor", &id).map_err(|_| AppError::BadRequest)?;
    Ok(id)
}

fn valid_csrf(session: &Session, received: &str) -> Result<(), AppError> {
    let expected = session.get::<String>("csrf").map_err(|_| AppError::Forbidden)?.ok_or(AppError::Forbidden)?;
    if !security::csrf_valid(&expected, received) { return Err(AppError::Forbidden); }
    Ok(())
}

#[derive(Deserialize)]
struct CommentForm { author: String, body: String, parent_id: Option<String>, csrf: String }

async fn add_blog_comment(session: Session, db: web::Data<Database>, slug: web::Path<String>, form: web::Form<CommentForm>) -> Result<HttpResponse, AppError> {
    valid_csrf(&session, &form.csrf)?;
    one(&db, "blog", &slug).await?;
    let author = form.author.trim();
    let body = form.body.trim();
    if !(2..=80).contains(&author.chars().count()) || !(3..=2000).contains(&body.chars().count()) { return Err(AppError::BadRequest); }
    let parent_id = form.parent_id.as_deref().filter(|v| !v.is_empty()).map(ObjectId::parse_str).transpose().map_err(|_| AppError::BadRequest)?;
    if let Some(parent) = parent_id {
        if blog_comments(&db).find_one(doc! {"_id":parent,"post_slug":slug.as_str(),"published":true}).await?.is_none() { return Err(AppError::BadRequest); }
    }
    blog_comments(&db).insert_one(BlogComment { id:None, post_slug:slug.to_string(), parent_id, author:author.into(), body:body.into(), likes:0, published:true, created_at:DateTime::now() }).await?;
    Ok(HttpResponse::SeeOther().insert_header((header::LOCATION, format!("/blog/{}#discussion", slug))).finish())
}

#[derive(Deserialize)]
struct LikeForm { csrf: String }

async fn toggle_reaction(session: &Session, db: &Database, target: String) -> Result<bool, AppError> {
    let visitor = visitor_id(session)?;
    if let Some(existing) = blog_reactions(db).find_one(doc! {"target":&target,"visitor":&visitor}).await? {
        if let Some(id) = existing.id { blog_reactions(db).delete_one(doc! {"_id":id}).await?; }
        Ok(false)
    } else {
        blog_reactions(db).insert_one(BlogReaction { id:None, target, visitor, created_at:DateTime::now() }).await?;
        Ok(true)
    }
}

async fn toggle_blog_like(session: Session, db: web::Data<Database>, slug: web::Path<String>, form: web::Form<LikeForm>) -> Result<HttpResponse, AppError> {
    valid_csrf(&session, &form.csrf)?;
    one(&db, "blog", &slug).await?;
    toggle_reaction(&session, &db, format!("post:{}", slug)).await?;
    Ok(HttpResponse::SeeOther().insert_header((header::LOCATION, format!("/blog/{}#discussion", slug))).finish())
}

async fn toggle_comment_like(session: Session, db: web::Data<Database>, path: web::Path<(String,String)>, form: web::Form<LikeForm>) -> Result<HttpResponse, AppError> {
    valid_csrf(&session, &form.csrf)?;
    let (slug, id) = path.into_inner();
    let oid = ObjectId::parse_str(&id).map_err(|_| AppError::BadRequest)?;
    if blog_comments(&db).find_one(doc! {"_id":oid,"post_slug":&slug,"published":true}).await?.is_none() { return Err(AppError::NotFound); }
    let added = toggle_reaction(&session, &db, format!("comment:{id}")).await?;
    let delta = if added { 1 } else { -1 };
    blog_comments(&db).update_one(doc! {"_id":oid}, doc! {"$inc":{"likes":delta}}).await?;
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
    let mut item = ContentItem {
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
    if item.image.is_empty() {
        item.image = format!("/media/covers/{}/{}.svg", item.kind, item.slug);
    }
    if item.image_alt.trim().is_empty() {
        item.image_alt = format!("Editorial image for {}", item.title);
    }
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

fn xml_text(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

async fn content_cover(db: web::Data<Database>, path: web::Path<(String, String)>) -> Result<HttpResponse, AppError> {
    use std::hash::{Hash, Hasher};
    let (kind, slug) = path.into_inner();
    if !["service","work","tech","about","insight","blog","testimonial"].contains(&kind.as_str()) || !slug_valid(&slug) { return Err(AppError::NotFound); }
    let item = one(&db, &kind, &slug).await?;
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
    let title: String = item.title.chars().take(48).collect();
    let number = format!("{:02}", (hash % 97) + 1);
    let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 900" role="img" aria-labelledby="title desc"><title id="title">{}</title><desc id="desc">Unique editorial cover for {}</desc><rect width="1200" height="900" fill="{}"/><path d="M0 0H700L1030 900H0Z" fill="{}"/><circle cx="{}" cy="{}" r="310" fill="{}" opacity=".92"/><path d="M760 0h440v440L980 660 760 440Z" fill="{}"/><path d="M0 720h1200v180H0Z" fill="{}" opacity=".96"/><g fill="{}" font-family="Arial,Helvetica,sans-serif"><text x="64" y="78" font-size="22" font-weight="700" letter-spacing="5">ITALY DEVELOPERS · {}</text><text x="64" y="690" font-size="68" font-weight="800" letter-spacing="-3">{}</text><text x="64" y="820" font-size="25" font-weight="700" letter-spacing="3">{} · {}</text></g></svg>"#, xml_text(&item.title), xml_text(&item.image_alt), paper, ink, x, y, primary, accent, paper, ink, xml_text(&kind.to_uppercase()), xml_text(&title), number, xml_text(&slug));
    Ok(HttpResponse::Ok().content_type("image/svg+xml; charset=utf-8").insert_header((header::CACHE_CONTROL,"public, max-age=31536000, immutable")).body(svg))
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
