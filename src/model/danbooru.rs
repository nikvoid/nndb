use chrono::{DateTime, Local};
use serde::{Serialize, Deserialize};

use super::TagType;

/// Generic pagination parameters
#[derive(Serialize)]
pub struct PaginatedRequest {
    pub page: u32,
    pub limit: u32,
    /// Retain/Join with other attributes, comma-separated
    pub only: String,
}

/// Generic? Ordering
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// Order by post count DESC
    PostCount,
}

/// Wiki pages search
/// https://danbooru.donmai.us/wiki_pages/api%3Awiki_pages
#[derive(Serialize)]
pub struct WikiSearch {
    pub order: Order,
    /// Include only pages that have >0 other names
    pub other_names_present: bool,
}

/// Paginated wiki pages query
#[derive(Serialize)]
pub struct WikiQuery {
    pub search: WikiSearch,
    #[serde(flatten)]
    pub pagination: PaginatedRequest,
}

/// Wiki entry (record subset).
/// `only=tag` is mandatory
#[derive(Deserialize, Serialize)]
pub struct WikiEntry {
    pub id: u32,
    pub title: String,
    // body: String,
    pub other_names: Vec<String>,
    // is_deleted: bool,
    // locked: bool,
    // created_at: DateTime<Local>,
    // pub updated_at: DateTime<Local>,
    pub tag: Option<TagEntry>,
}

/// Tag entry (record subset)
#[derive(Deserialize, Serialize)]
pub struct TagEntry {
    pub category: u32,
}

impl TryFrom<WikiEntry> for super::write::Wiki {
    type Error = ();

    /// Fail if `tag` is `None`
    fn try_from(value: WikiEntry) -> Result<Self, Self::Error> {
        let Some(tag) = value.tag else {
            return Err(())
        };
        
        Ok(Self {
            id: value.id,
            title: value.title,
            aliases: value.other_names,
            category: match tag.category {
                1 => TagType::Artist,
                3 => TagType::Title,
                4 => TagType::Character,
                5 => TagType::Metadata,
                _ => TagType::Tag
            }
        })
    }
}
