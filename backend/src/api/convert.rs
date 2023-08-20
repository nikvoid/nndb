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

impl From<model::read::Associated> for api::Associated {
    fn from(value: model::read::Associated) -> Self {
        Self {
            source: value.source,
            value: value.id,
            elements: value.elements.into_vec(),
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
