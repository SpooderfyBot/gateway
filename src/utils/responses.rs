#![allow(unused)]

use std::io::Cursor;
use std::error::Error;

use rocket::Response;
use rocket::http::Status;
use rocket::http::ContentType;
use rocket::response::Redirect;

use serde::{Serialize, Deserialize};
use serde_json::Value;


/// Returns a `rocket::Response` with headers set to use `ContentType::JSON`
/// and serializes the data with `serde_json::to_string(&body)?;`
pub fn json_response<T>(status: Status, body: &T) -> Result<Response<'static>, Box<dyn Error>>
where T: Serialize {
    let body = serde_json::to_string(&body)?;

    let mut resp = Response::new();
    resp.set_header(ContentType::JSON);
    resp.set_status(status);
    resp.set_sized_body(body.len(), Cursor::new(body));

    Ok(resp)
}

/// Returns a `rocket::Response` with the headers set to use `ContentType::HTML`
pub fn html_response(status: Status, body: &str) -> Result<Response, Box<dyn Error>> {
    let mut resp = Response::new();
    resp.set_header(ContentType::HTML);
    resp.set_status(status);
    resp.set_sized_body(body.len(), Cursor::new(body));

    Ok(resp)
}

/// Returns a `rocket::Response` with the headers set to use `ContentType::Plain`
pub fn plaintext_response(status: Status, body: &str) -> Result<Response, Box<dyn Error>> {
    let mut resp = Response::new();
    resp.set_header(ContentType::Plain);
    resp.set_status(status);
    resp.set_sized_body(body.len(), Cursor::new(body));

    Ok(resp)
}

/// Returns a `rocket::Response` with the headers set to use `ContentType::Plain`
pub fn redirect_to(url: String) -> Result<Response<'static>, Box<dyn Error>> {
    let mut resp = Response::new();
    resp.set_raw_header("Location", url);
    resp.set_status(Status::SeeOther);

    Ok(resp)
}