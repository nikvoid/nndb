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
    Count,
}

/// Tags search
/// https://danbooru.donmai.us/wiki_pages/api%3Atags
#[derive(Serialize)]
pub struct TagSearch {
    pub order: Order,
}

/// Paginated wiki pages query
#[derive(Serialize)]
pub struct TagQuery {
    pub search: TagSearch,
    #[serde(flatten)]
    pub pagination: PaginatedRequest,
}

/// Wiki entry (record subset).
#[derive(Deserialize, Serialize)]
pub struct WikiEntry {
    // pub id: u32,
    // pub title: String,
    // body: String,
    pub other_names: Vec<String>,
    // is_deleted: bool,
    // locked: bool,
    // created_at: DateTime<Local>,
    // pub updated_at: DateTime<Local>,
}

/// Tag entry (record subset)
/// `only=wiki_page` is mandatory
#[derive(Deserialize, Serialize)]
pub struct TagEntry {
    pub name: String,
    pub category: u32,
    pub wiki_page: Option<WikiEntry>
}

impl TryFrom<TagEntry> for super::write::Wiki {
    type Error = ();

    /// Fail if `tag` is `None`
    fn try_from(value: TagEntry) -> Result<Self, Self::Error> {
        let aliases = match value.wiki_page {
            Some(w) => w.other_names,
            None => vec![]
        };
        
        Ok(Self {
            title: value.name,
            aliases,
            category: match value.category {
                1 => TagType::Artist,
                3 => TagType::Title,
                4 => TagType::Character,
                5 => TagType::Metadata,
                _ => TagType::Tag
            }
        })
    }
}
