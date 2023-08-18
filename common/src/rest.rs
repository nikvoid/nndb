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

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct AutocompleteRequest {
    pub input: String
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct AutocompleteResponse {
    pub completions: Vec<Tag>
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct MetadataResponse {
    pub element: Element,
    pub metadata: ElementMetadata,
    pub associated: Vec<Associated>,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct TagResponse {
    pub tag: Tag,
    pub aliases: Vec<Tag>
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct TagEditRequest {
    pub tag_name: String,
    pub new_name: String,
    pub alt_name: Option<String>,
    pub tag_type: TagType,
    pub hidden: bool,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct TagAliasRequest {
    pub tag_name: String,
    pub query: String,   
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct LogRequest {
    pub read_size: u32
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct LogResponse {
    pub data: String
}

#[derive(Serialize, Deserialize, PartialEq, Default)]
pub struct StatusResponse {
    pub scan_files: TaskStatus,
    pub update_metadata: TaskStatus,
    pub group_elements: TaskStatus,
    pub make_thumbnails: TaskStatus,
    pub wiki_fetch: TaskStatus,
}

