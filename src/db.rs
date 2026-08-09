use chrono::{Duration, Utc};
use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::IndexOptions,
    Client, Database, IndexModel,
};
use serde::de::DeserializeOwned;

use crate::{
    auth,
    config::Config,
    models::{Admin, Faq, Insight, Service, SiteSettings, TeamMember, Testimonial, WorkItem},
};

pub async fn connect(config: &Config) -> anyhow::Result<Database> {
    let client = Client::with_uri_str(&config.mongodb_uri).await?;
    let db = client.database(&config.mongodb_db);
    db.run_command(doc! {"ping": 1}).await?;
    ensure_indexes(&db).await?;
    seed(&db, config).await?;
    Ok(db)
}

/// Fetches documents from `collection`, sorted by `order` ascending, optionally capped at `limit`.
pub async fn list_ordered<T>(db: &Database, collection: &str, filter: Document, limit: Option<i64>) -> Result<Vec<T>, mongodb::error::Error>
where
    T: DeserializeOwned + Unpin + Send + Sync,
{
    let coll = db.collection::<T>(collection);
    let mut find = coll.find(filter).sort(doc! {"order": 1});
    if let Some(l) = limit {
        find = find.limit(l);
    }
    find.await?.try_collect().await
}

pub async fn get_settings(db: &Database) -> Result<SiteSettings, mongodb::error::Error> {
    Ok(db.collection::<SiteSettings>("site_settings").find_one(doc! {}).await?.unwrap_or_default())
}

async fn ensure_indexes(db: &Database) -> anyhow::Result<()> {
    db.collection::<Admin>("admins")
        .create_index(
            IndexModel::builder()
                .keys(doc! {"username": 1})
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await?;
    db.collection::<mongodb::bson::Document>("leads")
        .create_indexes([
            IndexModel::builder().keys(doc! {"created_at": -1}).build(),
            IndexModel::builder().keys(doc! {"status": 1}).build(),
        ])
        .await?;
    db.collection::<Insight>("insights")
        .create_index(
            IndexModel::builder()
                .keys(doc! {"slug": 1})
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await?;
    for name in ["services", "work_items", "testimonials", "team_members", "faqs", "insights"] {
        db.collection::<mongodb::bson::Document>(name)
            .create_index(IndexModel::builder().keys(doc! {"order": 1}).build())
            .await?;
    }
    db.collection::<mongodb::bson::Document>("faqs")
        .create_index(IndexModel::builder().keys(doc! {"page": 1}).build())
        .await?;
    db.collection::<mongodb::bson::Document>("comments")
        .create_indexes([
            IndexModel::builder().keys(doc! {"slug": 1, "created_at": 1}).build(),
            IndexModel::builder().keys(doc! {"parent_id": 1}).build(),
        ])
        .await?;
    Ok(())
}

async fn seed(db: &Database, config: &Config) -> anyhow::Result<()> {
    seed_admin(db, config).await?;
    seed_settings(db).await?;
    seed_services(db).await?;
    seed_work(db).await?;
    seed_testimonials(db).await?;
    seed_team(db).await?;
    seed_faqs(db).await?;
    seed_insights(db).await?;
    Ok(())
}

async fn seed_admin(db: &Database, config: &Config) -> anyhow::Result<()> {
    let coll = db.collection::<Admin>("admins");
    if coll.count_documents(doc! {}).await? > 0 {
        return Ok(());
    }
    let (Some(username), Some(password)) = (&config.bootstrap_admin_username, &config.bootstrap_admin_password) else {
        tracing::warn!("no admins exist and ADMIN_USERNAME/ADMIN_PASSWORD are not set — the admin panel is inaccessible until an admin is created");
        return Ok(());
    };
    use secrecy::ExposeSecret;
    let password_hash = auth::hash_password(password.expose_secret())?;
    coll.insert_one(Admin { id: None, username: username.clone(), password_hash, created_at: Utc::now() }).await?;
    tracing::info!(username, "bootstrap admin created");
    Ok(())
}

async fn seed_settings(db: &Database) -> anyhow::Result<()> {
    let coll = db.collection::<SiteSettings>("site_settings");
    if coll.count_documents(doc! {}).await? > 0 {
        return Ok(());
    }
    coll.insert_one(SiteSettings::default()).await?;
    Ok(())
}

async fn seed_services(db: &Database) -> anyhow::Result<()> {
    let coll = db.collection::<Service>("services");
    if coll.count_documents(doc! {}).await? > 0 {
        return Ok(());
    }
    let items = [
        ("Website development", "High-performance, responsive and SEO-ready websites that turn visitors into enquiries, built on server-rendered Rust for speed search engines reward.", "Actix Web · Askama · Core Web Vitals", 1),
        ("Web applications & dashboards", "Secure portals, internal tools and SaaS products designed around real operating workflows, not generic templates.", "Rust · MongoDB · REST APIs", 2),
        ("Custom software & automation", "Purpose-built systems and automations that remove repetitive work and connect the tools your team already relies on.", "Automation · Integrations · Background jobs", 3),
        ("API & systems integration", "Well-documented, versioned APIs and integrations between the platforms your business depends on.", "REST · Webhooks · Auth", 4),
        ("Security review & hardening", "Independent review of authentication, session handling, input validation and infrastructure configuration.", "Threat modelling · Pen-testing basics · Hardening", 5),
    ];
    let docs = items.into_iter().map(|(title, summary, stack, order)| Service { id: None, title: title.into(), summary: summary.into(), stack: stack.into(), order }).collect::<Vec<_>>();
    coll.insert_many(docs).await?;
    Ok(())
}

async fn seed_work(db: &Database) -> anyhow::Result<()> {
    let coll = db.collection::<WorkItem>("work_items");
    if coll.count_documents(doc! {}).await? > 0 {
        return Ok(());
    }
    let items = [
        ("Direct-booking platform for a boutique hotel group", "A reservation website that cut third-party OTA commission by giving guests a fast, trustworthy way to book directly, with live availability and secure payments.", "Actix Web · MongoDB · Payments", "/static/images/work-hospitality.jpg", 1),
        ("Operations dashboard for a regional logistics company", "Replaced a tangle of spreadsheets with a live dashboard for dispatch, fleet status and delivery exceptions, cutting daily reporting time from hours to minutes.", "Rust · APIs · Automation", "/static/images/work-operations.jpg", 2),
        ("Headless storefront for a specialty foods brand", "A fast, secure storefront and checkout flow engineered for conversion, built to survive seasonal traffic spikes without falling over.", "Performance · Security · SEO", "/static/images/work-commerce.jpg", 3),
        ("Claims intake tool for an insurance broker", "An internal tool that replaced manual email intake with structured claims submission, validation and routing — reducing data-entry errors and turnaround time.", "Rust · MongoDB · Workflow automation", "", 4),
    ];
    let docs = items.into_iter().map(|(title, summary, stack, image, order)| WorkItem { id: None, title: title.into(), summary: summary.into(), stack: stack.into(), image: image.into(), order }).collect::<Vec<_>>();
    coll.insert_many(docs).await?;
    Ok(())
}

async fn seed_testimonials(db: &Database) -> anyhow::Result<()> {
    let coll = db.collection::<Testimonial>("testimonials");
    if coll.count_documents(doc! {}).await? > 0 {
        return Ok(());
    }
    let items = [
        ("They rebuilt our booking flow in weeks, not months, and direct bookings overtook OTA bookings within a quarter.", "Marco Ferrari", "General Manager", "Villa Serena Hotel Group", 1),
        ("The dashboard finally gave dispatch a single source of truth. Our exception-handling time dropped dramatically.", "Priya Nair", "Operations Director", "Meridian Logistics", 2),
        ("Security review caught issues our previous agency never mentioned, and they explained every fix in plain language.", "Alessio Romano", "CTO", "Nord Insurance Brokers", 3),
        ("Fast, clear communication and a site that has never gone down during a launch, even under heavy traffic.", "Kavya Iyer", "Founder", "Terra Foods Co.", 4),
        ("Rare to find a team that writes secure code by default instead of bolting security on at the end.", "Luca Bianchi", "Head of Engineering", "Adria Systems", 5),
    ];
    let docs = items.into_iter().map(|(quote, author, role, company, order)| Testimonial { id: None, quote: quote.into(), author: author.into(), role: role.into(), company: company.into(), photo: String::new(), order }).collect::<Vec<_>>();
    coll.insert_many(docs).await?;
    Ok(())
}

async fn seed_team(db: &Database) -> anyhow::Result<()> {
    let coll = db.collection::<TeamMember>("team_members");
    if coll.count_documents(doc! {}).await? > 0 {
        return Ok(());
    }
    let items = [
        ("Aarav Mehta", "Founder & Lead Engineer", "Leads architecture and delivery across every engagement, with a background in high-availability backend systems.", "", 1),
        ("Giulia Conti", "Product & Design Lead", "Shapes interfaces and content that convert, grounded in research rather than trend-chasing.", "", 2),
        ("Rohan Kapoor", "Security Engineer", "Reviews every project for authentication, data-handling and infrastructure risk before it ships.", "", 3),
        ("Chiara Esposito", "Client Partner", "Keeps projects on schedule and makes sure engineering decisions map back to business outcomes.", "", 4),
    ];
    let docs = items.into_iter().map(|(name, role, bio, image, order)| TeamMember { id: None, name: name.into(), role: role.into(), bio: bio.into(), image: image.into(), order }).collect::<Vec<_>>();
    coll.insert_many(docs).await?;
    Ok(())
}

async fn seed_faqs(db: &Database) -> anyhow::Result<()> {
    let coll = db.collection::<Faq>("faqs");
    if coll.count_documents(doc! {}).await? > 0 {
        return Ok(());
    }
    let items = [
        ("home", "Who is Italy Developers for?", "Ambitious companies that need a website, application or internal tool built properly the first time — not another generic template.", 1),
        ("home", "Do you only work with clients in Italy?", "No. We're based between India and Italy and work with clients worldwide, entirely remotely.", 2),
        ("services", "Do you build the whole product or just the website?", "Both. We take projects from first idea through design, engineering and launch, and can keep supporting them afterwards.", 1),
        ("services", "What does a typical engagement look like?", "A short discovery phase, a fixed-scope proposal, then focused build sprints with regular working demos — not silence until launch day.", 2),
        ("work", "Can you share more detail on past projects?", "Some engagements are under NDA, but we're happy to walk through relevant case studies and architecture decisions on a call.", 1),
        ("work", "Do you work with existing codebases?", "Yes — audits, security reviews and incremental improvements to an existing product are common engagements.", 2),
        ("about", "How big is the team?", "Small and senior by design — every project is built by people who also review architecture and security, not junior hand-offs.", 1),
        ("about", "Why Rust?", "Memory safety, predictable performance and a compiler that catches entire classes of bugs before they reach production.", 2),
        ("contact", "How quickly will I hear back?", "Within two working days, usually sooner.", 1),
        ("contact", "What should I include in my enquiry?", "What you're building, where you're stuck, and what success looks like. No need for a full spec up front.", 2),
    ];
    let docs = items.into_iter().map(|(page, question, answer, order)| Faq { id: None, page: page.into(), question: question.into(), answer: answer.into(), order }).collect::<Vec<_>>();
    coll.insert_many(docs).await?;
    Ok(())
}

async fn seed_insights(db: &Database) -> anyhow::Result<()> {
    let coll = db.collection::<Insight>("insights");
    if coll.count_documents(doc! {}).await? > 0 {
        return Ok(());
    }
    let now = Utc::now();
    let items = [
        (
            "security-belongs-in-product-design",
            "Security belongs in product design, not a final review",
            "Authentication, authorization, data boundaries and abuse cases should be designed before a line of implementation code is written.",
            "Security",
            "Security is usually treated as a checklist run at the end of a project: a penetration test, a scan, a sign-off. By the time that happens, the cost of fixing what's found has multiplied — the data model is fixed, the API shape is fixed, and every fix competes with the launch date.\n\nThe cheaper path is to design for the failure cases up front. Before writing a login form, decide what happens when a session is stolen, when a password is reused elsewhere, when a rate limit is bypassed. Before storing a customer record, decide who can read it, who can write it, and what happens when that access needs to be revoked.\n\nOn every engagement we run a short threat-modelling pass before implementation: who are the plausible attackers, what do they want, and which parts of the system would hurt the most if they were wrong. It rarely takes more than a day, and it consistently reshapes the schema and the API surface in ways that are far cheaper to change on paper than in production.\n\nSecure defaults help too — session cookies that are `HttpOnly`, `Secure` and `SameSite=Strict` by default; a strict Content-Security-Policy from day one instead of retrofitted later; input validation at the boundary rather than scattered through the codebase. None of this is exotic. It's just sequencing: security as a design input, not a final inspection.",
            7,
        ),
        (
            "when-rust-creates-leverage",
            "When Rust creates real leverage for a product team",
            "Rust is a strong choice for reliable APIs, predictable performance and memory safety — but it isn't the right tool for every team or every deadline.",
            "Engineering",
            "Rust gets recommended for reasons that don't always apply to the project in front of you. It's worth being specific about where it actually pays off.\n\nIt pays off when correctness and uptime matter more than shipping speed on day one — a payments flow, an authentication system, a service that has to run unattended for months. The compiler removes whole categories of bugs (null pointer errors, data races, use-after-free) before they reach a user.\n\nIt pays off when performance is a product requirement, not a nice-to-have — when a service needs to handle spikes in traffic without scaling out horizontally just to stay responsive, or when infrastructure cost per request actually matters at your volume.\n\nIt does not automatically pay off when the team is small, timelines are tight, and the domain is still being discovered. The strictness that prevents bugs later also slows down early exploration. In those cases we'll sometimes prototype faster and port the parts that matter once the shape of the product is settled.\n\nThe honest pitch for Rust is not 'it's the best language.' It's: for the specific failure modes that are expensive to have in production — crashes, memory corruption, data races — Rust removes them at compile time instead of catching them in an incident review.",
            6,
        ),
        (
            "seo-foundations-that-ship",
            "SEO foundations that ship with the product, not after it",
            "Semantic HTML, crawlable routes, real metadata, useful content and fast pages are product requirements, not a marketing add-on bolted on later.",
            "SEO",
            "Most SEO problems are architecture problems wearing a marketing hat. A single-page app that renders content client-side and never gives crawlers a real HTML response. Routes that don't correspond to real, linkable, bookmarkable pages. Metadata that's identical across every page because nobody wired it up per-route.\n\nServer-rendered HTML with a real `<title>`, description and canonical URL per page solves most of this by construction. So does using semantic elements — `h1` for the actual page subject, `nav` for navigation, real anchor tags for links a crawler can follow — instead of `div`s with click handlers.\n\nPage speed is part of this too, and it compounds: a fast first response, minimal render-blocking JavaScript, and images sized and compressed for the layout they're used in. None of this is a plugin you install afterwards; it's a byproduct of building the page correctly the first time.\n\nThe content still has to be worth ranking — genuinely useful, specific, written for the person searching rather than for a keyword density target. But the technical foundation determines whether that content ever gets the chance to be read.",
            5,
        ),
        (
            "moving-from-postgres-to-mongodb",
            "What actually changes when you move from a relational database to MongoDB",
            "Document databases aren't a drop-in replacement for a relational schema — they reward a different way of thinking about data ownership and access patterns.",
            "Engineering",
            "Migrating a schema from a relational database to MongoDB is not a mechanical translation of tables to collections. The two model data around different questions.\n\nA relational schema is organised around avoiding duplication: normalise data into tables, join at query time. A document schema is organised around access patterns: what does a single screen need to render, and can that be fetched in one round trip. That usually means embedding related data that's read together and referencing data that's owned, sized or updated independently.\n\nThe practical rule we use: embed when the child data is small, bounded and always read with its parent (a testimonial's author name and role, say). Reference when the data is large, unbounded, or has its own lifecycle (a customer's order history, which keeps growing and gets queried on its own).\n\nIndexes still matter just as much as in a relational database — a document database without the right indexes degrades the same way a relational one does. And transactions are available when an operation genuinely needs atomicity across documents, but reaching for them by default usually signals the schema should be reshaped instead.\n\nThe migration effort is mostly in this modelling decision, not in swapping a driver. Get the access patterns right and the queries stay simple and fast.",
            4,
        ),
        (
            "building-an-admin-panel-you-can-trust",
            "Building an admin panel you can actually trust with production data",
            "An internal admin panel has the same attack surface as the public product — sometimes worse, because it has more privilege.",
            "Security",
            "Admin panels get built last and reviewed least, which is backwards given what they can do: read every customer record, change any piece of content, delete anything. A public-facing bug might expose one account. An admin-panel bug can expose all of them.\n\nA few defaults we treat as non-negotiable: every admin session is checked server-side on every request, not just at login. Every state-changing action requires a CSRF token, exactly like the public-facing forms. Passwords are hashed with a slow, salted algorithm (Argon2), never stored or logged in plain text. And admin actions that matter — deleting a record, changing a status — are deliberate single actions, not something a stray click can trigger.\n\nThe other habit worth keeping: the admin panel should use the same validation and the same data layer as the public site, not a separate, less-audited path into the same collections. Two ways to write to the same data means two places bugs can hide.\n\nNone of this is exotic engineering. It's just treating the highest-privilege part of the system as the highest-priority part to get right.",
            3,
        ),
    ];
    let docs = items
        .into_iter()
        .map(|(slug, title, summary, category, body, days_ago)| Insight {
            id: None,
            slug: slug.into(),
            title: title.into(),
            summary: summary.into(),
            body: body.into(),
            category: category.into(),
            published_at: now - Duration::days(days_ago),
            order: days_ago as i32,
        })
        .collect::<Vec<_>>();
    coll.insert_many(docs).await?;
    Ok(())
}
