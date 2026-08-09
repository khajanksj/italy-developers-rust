use std::{future::{ready, Ready}, pin::Pin, task::{Context, Poll}};
use actix_session::Session;
use actix_web::{dev::{Service, ServiceRequest, ServiceResponse, Transform}, Error, HttpMessage};
use crate::error::AppError;

pub const CSP:&str = "default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; manifest-src 'self'; upgrade-insecure-requests";

pub struct RequestId;
impl<S,B> Transform<S,ServiceRequest> for RequestId where S:Service<ServiceRequest,Response=ServiceResponse<B>,Error=Error>+'static,S::Future:'static,B:'static {
    type Response=ServiceResponse<B>; type Error=Error; type InitError=(); type Transform=RequestIdMiddleware<S>; type Future=Ready<Result<Self::Transform,Self::InitError>>;
    fn new_transform(&self,service:S)->Self::Future{ready(Ok(RequestIdMiddleware{service}))}
}
pub struct RequestIdMiddleware<S>{service:S}
impl<S,B> Service<ServiceRequest> for RequestIdMiddleware<S> where S:Service<ServiceRequest,Response=ServiceResponse<B>,Error=Error>+'static,S::Future:'static,B:'static {
    type Response=ServiceResponse<B>; type Error=Error; type Future=Pin<Box<dyn std::future::Future<Output=Result<Self::Response,Self::Error>>>>;
    fn poll_ready(&self,cx:&mut Context<'_>)->Poll<Result<(),Self::Error>>{self.service.poll_ready(cx)}
    fn call(&self,req:ServiceRequest)->Self::Future { let id=format!("{:x}-{:x}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()); req.extensions_mut().insert(id.clone()); let fut=self.service.call(req); Box::pin(async move{let mut res=fut.await?; if let Ok(value)=actix_web::http::header::HeaderValue::from_str(&id){res.headers_mut().insert(actix_web::http::header::HeaderName::from_static("x-request-id"),value);} Ok(res)}) }
}

pub fn csrf_valid(expected:&str,received:&str)->bool { expected.len()==received.len() && constant_time_eq::constant_time_eq(expected.as_bytes(),received.as_bytes()) }

fn random_token()->String{format!("{:x}{:x}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos())}
pub fn get_or_create_csrf(session:&Session)->Result<String,AppError>{let token=session.get::<String>("csrf").map_err(|_|AppError::BadRequest)?.unwrap_or_else(random_token);session.insert("csrf",&token).map_err(|_|AppError::BadRequest)?;Ok(token)}
