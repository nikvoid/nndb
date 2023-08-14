use gloo::net::http::Request;
use serde::{de::DeserializeOwned, Serialize};

/// Backend host url
pub static BACKEND_URL: &str = env!("BACKEND_URL");

pub type GlooResult<T> = Result<T, gloo::net::Error>;

/// Backend GET json
#[macro_export]
macro_rules! backend_get {
    ($fmt_tail:literal $($tail:tt)*) => {{
        use $crate::api::*;
        get_json(
            &format!(concat!("{}", $fmt_tail), BACKEND_URL $($tail)*)
        )
    }};
}

/// Backend POST json
#[macro_export]
macro_rules! backend_post {
    ($req:expr, $fmt_tail:literal $($tail:tt)*) => {{
        use $crate::api::*;
        post_json(
            &format!(concat!("{}", $fmt_tail), BACKEND_URL $($tail)*),
            $req
        )
    }};
}

pub async fn get_json<O>(url: &str) -> GlooResult<O> 
where 
    O: DeserializeOwned 
{
    let out: O = Request::get(url)
        .send()
        .await?
        .json()
        .await?;
    Ok(out)
}
pub async fn post_json<I, O>(url: &str, payload: &I) -> GlooResult<O> 
where 
    O: DeserializeOwned,
    I: Serialize
{
    let out: O = Request::post(url)
        .json(payload)?
        .send()
        .await?
        .json()
        .await?;

    Ok(out)
}