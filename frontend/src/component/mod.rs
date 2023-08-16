pub mod prelude {
    pub use yew::prelude::*;
    pub use yew_router::prelude::*;
    pub use nndb_common::*;
    pub use std::rc::Rc;
    pub use crate::route::*;
    pub use serde::{Serialize, Deserialize};
    pub use futures::FutureExt;
    pub use crate::{backend_post, backend_get};
}

pub mod element;
pub mod paginator;
pub mod input;
pub mod metadata;
pub mod link;