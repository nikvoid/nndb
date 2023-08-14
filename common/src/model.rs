use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

pub type UtcDateTime = DateTime<Utc>;

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Tag {
    /// Tag id
    pub id: u32,
    /// Primary name
    pub name: String,
    /// Alternative name
    pub alt_name: Option<String>,
    /// Tag type
    pub tag_type: TagType,
    /// Count of elements with this tag
    pub count: u32,
    /// Is tag hidden
    pub hidden: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Element {
    /// Element id
    pub id: u32,
    /// Url to file
    pub url: String,
    /// Field to mark broken file
    pub broken: bool,
    /// Url to ile thumbnail
    pub thumb_url: Option<String>,
    /// Whether element is animation
    pub animated: bool,
}   

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct ElementMetadata {
    /// Link to source (if was imported from other sources)
    pub src_links: Vec<(String, String)>,
    /// Time when element was added to other source (if present)
    pub src_times: Vec<(String, UtcDateTime)>,
    /// Time when element was added to db
    pub add_time: UtcDateTime,
    /// Stable Diffusion/etc metadata
    pub ai_meta: Option<AIMetadata>,
    /// Tags of the element
    pub tags: Vec<Tag>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Associated {
    /// Associated by this key
    pub key: String,
    /// Key value
    pub value: i64,
    /// Associated elements
    pub elements: Vec<Element>
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct AIMetadata {
    /// Positive prompt
    pub positive_prompt: String,
    /// Negative prompt
    pub negative_prompt: Option<String>,
    /// Step count
    pub steps: u32,
    /// CFG scale
    pub scale: f32,
    /// Used sampler
    pub sampler: String,
    /// Generation seed
    pub seed: i64,
    /// Denoising strength
    pub strength: f32,
    /// Noise
    pub noise: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TagType {
    Service,
    Artist,
    Character,
    Title,
    Metadata,
    Tag,
}
