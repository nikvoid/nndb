use crate::{model, CONFIG, config::StaticFolder, import};
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

impl From<model::read::Tag> for api::Tag {
    fn from(value: model::read::Tag) -> Self {
        Self {
            id: value.id,
            name: value.name,
            alt_name: value.alt_name,
            tag_type: value.tag_type,
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

impl From<model::read::ElementMetadata> for api::ElementMetadata {
    fn from(value: model::read::ElementMetadata) -> Self {
        Self {
            src_links: (&value.src_links).into_vec(),
            src_times: (&value.src_times).into_vec(),
            add_time: value.add_time,
            ai_meta: value.ai_meta,
            tags: value.tags.into_vec(),
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

impl<T> IntoVec<(String, T)> for &Vec<(import::Fetcher, T)> where T: Clone {
    fn into_vec(self) -> Vec<(String, T)> {
        self.iter().map(|x| (x.0.name().into(), x.1.clone())).collect()
    }
}