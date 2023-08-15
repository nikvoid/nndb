use crate::{model, CONFIG, config::StaticFolder};
use nndb_common::model as api;

impl StaticFolder {
    /// Get absolute or relative url
    pub fn url(&self, tail: &str) -> String {
        if self.serve {
            format!("http://{}:{}{}{}", CONFIG.bind_address, CONFIG.port, self.url, tail)
        } else {
            self.url.clone() + tail
        }
    }
}

impl From<model::TagType> for api::TagType {
    fn from(value: model::TagType) -> Self {
        match value {
            model::TagType::Service => Self::Service,
            model::TagType::Artist => Self::Artist,
            model::TagType::Character => Self::Character,
            model::TagType::Title => Self::Title,
            model::TagType::Metadata => Self::Metadata,
            model::TagType::Tag => Self::Tag,
        }
    }
}

impl From<model::read::Tag> for api::Tag {
    fn from(value: model::read::Tag) -> Self {
        Self {
            id: value.id,
            name: value.name,
            alt_name: value.alt_name,
            tag_type: value.tag_type.into(),
            count: value.count,
            hidden: value.hidden,
        }
    }
}

impl From<model::read::Element> for api::Element {
    fn from(value: model::read::Element) -> Self {
        Self {
            id: value.id,
            url: CONFIG.element_pool.url(&value.filename),
            broken: value.broken,
            thumb_url: value.has_thumb.then(|| {
                let name = value
                    .filename
                    .split('.')
                    .next()
                    .unwrap()
                    .to_string() + ".jpeg";
                CONFIG.thumbnails_folder.url(&name)
            }),
            animated: value.animated,
        }
    }
}

/// Helper for converting `Vec<T> -> Vec<U> where U: From<T>`
pub trait IntoVec<T> {
    fn into_vec(self) -> Vec<T>;
}

impl<T, U> IntoVec<T> for Vec<U> where T: From<U> {
    fn into_vec(self) -> Vec<T> {
        self.into_iter().map(|x| x.into()).collect()
    }
}