#[derive(Clone, Copy)]
pub enum FieldKind { Text, TextArea, Number, Markdown }

#[derive(Clone, Copy)]
pub struct FieldSpec {
    pub name: &'static str,
    pub label: &'static str,
    pub kind: FieldKind,
    pub required: bool,
}

pub const fn f(name: &'static str, label: &'static str, kind: FieldKind, required: bool) -> FieldSpec {
    FieldSpec { name, label, kind, required }
}

#[derive(Clone, Copy)]
pub struct ContentType {
    pub key: &'static str,
    pub label: &'static str,
    pub collection: &'static str,
    pub fields: &'static [FieldSpec],
    pub title_field: &'static str,
    pub subtitle_field: &'static str,
}

pub const SERVICES: ContentType = ContentType {
    key: "services", label: "Services", collection: "services",
    fields: &[
        f("title", "Title", FieldKind::Text, true),
        f("summary", "Summary", FieldKind::TextArea, true),
        f("stack", "Stack / caption", FieldKind::Text, true),
        f("order", "Order", FieldKind::Number, true),
    ],
    title_field: "title", subtitle_field: "stack",
};

pub const WORK: ContentType = ContentType {
    key: "work", label: "Work / Case studies", collection: "work_items",
    fields: &[
        f("title", "Title", FieldKind::Text, true),
        f("summary", "Summary", FieldKind::TextArea, true),
        f("stack", "Stack / caption", FieldKind::Text, true),
        f("image", "Image path", FieldKind::Text, false),
        f("order", "Order", FieldKind::Number, true),
    ],
    title_field: "title", subtitle_field: "stack",
};

pub const TESTIMONIALS: ContentType = ContentType {
    key: "testimonials", label: "Testimonials", collection: "testimonials",
    fields: &[
        f("quote", "Quote", FieldKind::TextArea, true),
        f("author", "Author", FieldKind::Text, true),
        f("role", "Role", FieldKind::Text, true),
        f("company", "Company", FieldKind::Text, true),
        f("photo", "Photo URL (optional — falls back to initials)", FieldKind::Text, false),
        f("order", "Order", FieldKind::Number, true),
    ],
    title_field: "author", subtitle_field: "company",
};

pub const TEAM: ContentType = ContentType {
    key: "team", label: "Team", collection: "team_members",
    fields: &[
        f("name", "Name", FieldKind::Text, true),
        f("role", "Role", FieldKind::Text, true),
        f("bio", "Bio", FieldKind::TextArea, true),
        f("image", "Image path", FieldKind::Text, false),
        f("order", "Order", FieldKind::Number, true),
    ],
    title_field: "name", subtitle_field: "role",
};

pub const FAQS: ContentType = ContentType {
    key: "faqs", label: "FAQs", collection: "faqs",
    fields: &[
        f("page", "Page (home/services/work/about/contact)", FieldKind::Text, true),
        f("question", "Question", FieldKind::Text, true),
        f("answer", "Answer", FieldKind::TextArea, true),
        f("order", "Order", FieldKind::Number, true),
    ],
    title_field: "question", subtitle_field: "page",
};

pub const INSIGHTS: ContentType = ContentType {
    key: "insights", label: "Insights", collection: "insights",
    fields: &[
        f("slug", "Slug", FieldKind::Text, true),
        f("title", "Title", FieldKind::Text, true),
        f("summary", "Summary", FieldKind::TextArea, true),
        f("category", "Category", FieldKind::Text, true),
        f("body", "Body (Markdown)", FieldKind::Markdown, true),
        f("order", "Order", FieldKind::Number, true),
    ],
    title_field: "title", subtitle_field: "category",
};

pub const ALL: &[ContentType] = &[SERVICES, WORK, TESTIMONIALS, TEAM, FAQS, INSIGHTS];

pub fn find(key: &str) -> Option<ContentType> {
    ALL.iter().copied().find(|c| c.key == key)
}
