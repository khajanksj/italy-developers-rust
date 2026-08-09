use std::{collections::HashMap, fmt::Write};

use mongodb::bson::oid::ObjectId;

use crate::models::Comment;

fn escape_html(input: &str) -> String {
    input.chars().fold(String::with_capacity(input.len()), |mut acc, c| {
        match c {
            '&' => acc.push_str("&amp;"),
            '<' => acc.push_str("&lt;"),
            '>' => acc.push_str("&gt;"),
            '"' => acc.push_str("&quot;"),
            '\'' => acc.push_str("&#39;"),
            _ => acc.push(c),
        }
        acc
    })
}

/// Renders the full `#comments-block` fragment: heading, nested comment tree, and the top-level form.
/// This is what every comment mutation (new top-level comment or reply) swaps back in via htmx.
pub fn render_block(slug: &str, comments: &[Comment], csrf: &str) -> String {
    let mut children: HashMap<Option<ObjectId>, Vec<&Comment>> = HashMap::new();
    for c in comments {
        children.entry(c.parent_id).or_default().push(c);
    }
    let mut list = String::new();
    if let Some(roots) = children.get(&None) {
        for c in roots {
            render_node(c, &children, slug, &mut list);
        }
    }
    let count = comments.len();
    let noun = if count == 1 { "comment" } else { "comments" };
    format!(
        r##"<section id="comments-block" class="comments"><p class="kicker">Discussion</p><h2>{count} {noun}</h2><div class="comment-list">{list}</div>{form}</section>"##,
        count = count,
        noun = noun,
        list = list,
        form = render_form(slug, None, csrf),
    )
}

fn render_node(c: &Comment, children: &HashMap<Option<ObjectId>, Vec<&Comment>>, slug: &str, out: &mut String) {
    let Some(id) = c.id else { return };
    let id = id.to_hex();
    let _ = write!(
        out,
        r##"<div class="comment" id="comment-{id}"><div class="comment-meta"><strong>{author}</strong><span>{date}</span></div><p class="comment-body">{body}</p><div class="comment-actions" id="actions-{id}"><a href="#" hx-get="/insights/{slug}/comments/{id}/reply-form" hx-target="#reply-slot-{id}" hx-swap="innerHTML">Reply</a></div><div id="reply-slot-{id}"></div><div class="comment-children">"##,
        id = id,
        author = escape_html(&c.author),
        date = c.created_at.format("%-d %b %Y"),
        body = escape_html(&c.body),
        slug = slug,
    );
    if let Some(kids) = c.id.and_then(|cid| children.get(&Some(cid))) {
        for k in kids {
            render_node(k, children, slug, out);
        }
    }
    out.push_str("</div></div>");
}

/// Renders a comment form. `parent_id` of `None` renders the top-level "add a comment" form;
/// `Some(id)` renders an inline reply form (with a Cancel link) targeting that comment's reply slot.
pub fn render_form(slug: &str, parent_id: Option<&str>, csrf: &str) -> String {
    let parent_field = parent_id.map(|p| format!(r##"<input type="hidden" name="parent_id" value="{p}">"##, p = p)).unwrap_or_default();
    let cancel = parent_id
        .map(|p| {
            format!(
                r##" <button type="button" class="text-link" hx-get="/insights/{slug}/comments/{p}/reply-cancel" hx-target="#reply-slot-{p}" hx-swap="innerHTML">Cancel</button>"##,
                slug = slug,
                p = p
            )
        })
        .unwrap_or_default();
    let heading = if parent_id.is_some() { "" } else { r##"<h3>Add a comment</h3>"## };
    format!(
        r##"<form class="comment-form" hx-post="/insights/{slug}/comments" hx-target="#comments-block" hx-swap="outerHTML">{heading}<input type="hidden" name="csrf" value="{csrf}">{parent_field}<div class="trap" aria-hidden="true"><label>Website<input name="website" tabindex="-1" autocomplete="off"></label></div><label>Name<input name="author" required minlength="2" maxlength="80"></label><label class="wide">Comment<textarea name="body" required minlength="2" maxlength="2000" rows="3"></textarea></label><button class="button" type="submit">Post comment</button>{cancel}</form>"##,
        slug = slug,
        heading = heading,
        csrf = csrf,
        parent_field = parent_field,
        cancel = cancel,
    )
}
