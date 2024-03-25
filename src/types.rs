use anyhow::Error;
use serde_derive::{Deserialize, Serialize};
use warp::reject::Reject;

#[derive(Debug, Clone, Copy)]
pub enum SourceName {
    Safari,
    Firefox,
    Chrome,
}

#[derive(Debug, Serialize)]
pub struct VisitDetail {
    pub url: String,
    pub title: String,
    // unix_epoch_ms
    pub visit_time: i64,
    pub visit_type: i64,
}

#[derive(Debug, Deserialize)]
pub struct DetailsQueryParams {
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IndexQueryParams {
    pub start: Option<String>, // Y-m-d
    pub end: Option<String>,   // Y-m-d
    pub keyword: Option<String>,
}

#[derive(Debug)]
pub struct ServerError {
    pub e: String,
}

impl From<Error> for ServerError {
    fn from(err: Error) -> Self {
        Self {
            e: format!("{err:#}"),
        }
    }
}

impl Reject for ServerError {}

#[derive(Debug)]
pub struct ClientError {
    pub e: String,
}

impl From<Error> for ClientError {
    fn from(err: Error) -> Self {
        Self {
            e: format!("{err:#}"),
        }
    }
}

impl Reject for ClientError {}

#[derive(Serialize)]
pub struct ErrorMessage {
    pub code: u16,
    pub message: String,
}
