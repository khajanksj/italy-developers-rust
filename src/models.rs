use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

fn default_status() -> String { "new".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub email: String,
    pub company: Option<String>,
    pub service: Option<String>,
    pub message: String,
    #[serde(default = "default_status")]
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Admin {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub title: String,
    pub summary: String,
    pub stack: String,
    #[serde(default)]
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub title: String,
    pub summary: String,
    pub stack: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Testimonial {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub quote: String,
    pub author: String,
    pub role: String,
    pub company: String,
    #[serde(default)]
    pub photo: String,
    #[serde(default)]
    pub order: i32,
}

/// Template-facing view of a testimonial with a computed avatar fallback.
#[derive(Debug, Clone, Serialize)]
pub struct TestimonialView {
    pub quote: String,
    pub author: String,
    pub role: String,
    pub company: String,
    pub photo: String,
    pub initials: String,
}

impl From<Testimonial> for TestimonialView {
    fn from(t: Testimonial) -> Self {
        let initials = t
            .author
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();
        Self { quote: t.quote, author: t.author, role: t.role, company: t.company, photo: t.photo, initials }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub slug: String,
    pub parent_id: Option<ObjectId>,
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub role: String,
    pub bio: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faq {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub page: String,
    pub question: String,
    pub answer: String,
    #[serde(default)]
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub category: String,
    pub published_at: DateTime<Utc>,
    #[serde(default)]
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteSettings {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub twitter: String,
    pub linkedin: String,
    pub github: String,
    pub years_active: i32,
    pub projects_delivered: i32,
    pub countries_served: i32,
    pub response_time: String,
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            id: None,
            phone: "+91 90000 00000".into(),
            email: "hello@italydevelopers.com".into(),
            address: "Remote-first · India & Italy".into(),
            twitter: "https://twitter.com/italydevelopers".into(),
            linkedin: "https://linkedin.com/company/italydevelopers".into(),
            github: "https://github.com/italydevelopers".into(),
            years_active: 5,
            projects_delivered: 40,
            countries_served: 9,
            response_time: "2 working days".into(),
        }
    }
}
