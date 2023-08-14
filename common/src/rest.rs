use serde::{Serialize, Deserialize};
use crate::model::*;

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct SearchRequest {
    pub query: String,
    pub offset: u32,
    pub limit: u32,
    pub tag_limit: u32,
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct SearchResponse {
    pub elements: Vec<Element>,
    pub tags: Vec<Tag>,
    pub count: u32
}