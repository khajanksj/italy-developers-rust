use actix_web::{error::JsonPayloadError, http::StatusCode, HttpRequest, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug,Error)] pub enum AppError { #[error("not found")] NotFound, #[error("invalid request")] BadRequest, #[error("forbidden")] Forbidden, #[error("service unavailable")] Database(#[from] mongodb::error::Error), #[error("template error")] Template(#[from] askama::Error), #[error("storage error")] Io(#[from] std::io::Error) }
impl ResponseError for AppError { fn status_code(&self)->StatusCode{match self{Self::NotFound=>StatusCode::NOT_FOUND,Self::BadRequest=>StatusCode::BAD_REQUEST,Self::Forbidden=>StatusCode::FORBIDDEN,_=>StatusCode::INTERNAL_SERVER_ERROR}} fn error_response(&self)->HttpResponse{if self.status_code().is_server_error(){tracing::error!(error=%self,"request failed");} HttpResponse::build(self.status_code()).content_type("text/plain; charset=utf-8").body(self.to_string())}}
pub fn json_error(_:JsonPayloadError,_:&HttpRequest)->actix_web::Error{AppError::BadRequest.into()}
pub fn form_error(_:actix_web::error::UrlencodedError,_:&HttpRequest)->actix_web::Error{AppError::BadRequest.into()}
