mod admin;
mod public;

use actix_web::{http::header, web, HttpResponse};
use askama::Template;

use crate::error::AppError;

pub(crate) fn html<T: Template>(t: T) -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").insert_header((header::CACHE_CONTROL, "public, max-age=0, must-revalidate")).body(t.render()?))
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/", web::get().to(public::home))
        .route("/services", web::get().to(public::services))
        .route("/work", web::get().to(public::work))
        .route("/about", web::get().to(public::about))
        .route("/insights", web::get().to(public::insights))
        .route("/insights/{slug}", web::get().to(public::insight_detail))
        .route("/insights/{slug}/comments", web::post().to(public::create_comment))
        .route("/insights/{slug}/comments/{id}/reply-form", web::get().to(public::reply_form))
        .route("/insights/{slug}/comments/{id}/reply-cancel", web::get().to(public::reply_cancel))
        .route("/contact", web::get().to(public::contact))
        .route("/contact", web::post().to(public::submit_contact))
        .route("/robots.txt", web::get().to(public::robots))
        .route("/sitemap.xml", web::get().to(public::sitemap))
        .route("/health/live", web::get().to(public::live))
        .route("/health/ready", web::get().to(public::ready))
        .service(
            web::scope("/admin")
                .route("/login", web::get().to(admin::login_page))
                .route("/login", web::post().to(admin::login_submit))
                .route("/logout", web::post().to(admin::logout))
                .route("", web::get().to(admin::dashboard))
                .route("/leads", web::get().to(admin::leads_list))
                .route("/leads/{id}/status", web::post().to(admin::lead_update_status))
                .route("/leads/{id}/delete", web::post().to(admin::lead_delete))
                .route("/content/{type}", web::get().to(admin::content_list))
                .route("/content/{type}/new", web::get().to(admin::content_new))
                .route("/content/{type}/new", web::post().to(admin::content_create))
                .route("/content/{type}/{id}/edit", web::get().to(admin::content_edit))
                .route("/content/{type}/{id}/edit", web::post().to(admin::content_update))
                .route("/content/{type}/{id}/delete", web::post().to(admin::content_delete))
                .route("/settings", web::get().to(admin::settings_page))
                .route("/settings", web::post().to(admin::settings_update))
                .route("/admins", web::get().to(admin::admins_list))
                .route("/admins/new", web::post().to(admin::admins_create))
                .route("/admins/{id}/delete", web::post().to(admin::admins_delete)),
        )
        .default_service(web::route().to(public::not_found));
}
