use anyhow::bail;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use pixivcrab::{AppApi, AppApiConfig, AuthMethod, models::illust::Illust};
use regex::Regex;
use reqwest::{ClientBuilder, Client, StatusCode};
use serde::Deserialize;

use crate::{model::{read::PendingImport, write::{ElementMetadata, Tag}, TagType}, CONFIG, dao::STORAGE, config::PixivCreds};

use super::MetadataFetcher;

// Pixiv metadata fetcher
// TODO: We can extensively use cache here, as several images can be in one work
pub struct Pixiv {
    api: Option<AppApi>,
    client: Client,
}

/// Images saved from pixiv web version
///
///     104550403_p0_master1200.jpg
///     work_id   page     ???
static WEB_REX: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d+)_p\d+_master\d+").unwrap());

/// Images saved from pixiv mobile app
/// 
///     illust_103201575_20221210_034038.png
///            work_id   date     time
static APP_REX: Lazy<Regex> = Lazy::new(|| Regex::new(r"illust_(\d+)_\d+_\d+").unwrap()); 

/// Pixiv fetcher singleton
pub static PIXIV: Lazy<Pixiv> = Lazy::new(|| { 
    Pixiv::new(CONFIG.pixiv_credentials.clone())
});

/// Base pixiv api url
const PIXIV_API_URL: &str = "https://app-api.pixiv.net";

impl Pixiv {
    fn new(creds: Option<PixivCreds>) -> Self {
        Self {
            api: creds.map(|cred| {
                let mut cfg = AppApiConfig::default();
                cfg.set_language("en-us").unwrap();

                cfg.client_secret = cred.client_secret;
                cfg.client_id = cred.client_id;
                AppApi::new_with_config(
                    AuthMethod::RefreshToken(cred.refresh_token), 
                    ClientBuilder::new(),
                    cfg
                ).unwrap()
            }),
            client: Client::new()
        }
    }
}

/// Wrapper for parsing
#[derive(Deserialize)]
struct IllustResponse {
    illust: Illust
}

#[async_trait]
impl MetadataFetcher for Pixiv {
    
    /// Check if importer can get metadata for element
    /// Try to match typical web or app filename 
    fn supported(&self, import: &PendingImport) -> bool {
        APP_REX.is_match(&import.orig_filename) 
        || WEB_REX.is_match(&import.orig_filename)
    }
    
    /// Check if importer can fetch metadata now
    fn available(&self) -> bool { self.api.is_some() }
    
    /// Fetch metadata for pending import (network access implied)
    async fn fetch_metadata(
        &self,
        import: &PendingImport
    ) -> anyhow::Result<Option<ElementMetadata>> {
        let Some(api) = &self.api else { bail!("client is not configured") };
        
        // Extract illust_id
        let illust_id: u64 = if let Some(capts) = APP_REX.captures(&import.orig_filename) {
            capts.get(1).unwrap().as_str().parse()?
        } else if let Some(capts) = WEB_REX.captures(&import.orig_filename) {
            capts.get(1).unwrap().as_str().parse()?
        } else {
            bail!("Failed to get illust id");
        };
    
        let request = self.client
            .get(format!("{PIXIV_API_URL}/v1/illust/detail?illust_id={illust_id}"));

        let resp = api.send_authorized(request).await?;    

        match resp.status() {
            StatusCode::OK => {
                let IllustResponse { illust } = resp.json().await?;

                let tags = tokio::task::spawn_blocking(move || {
                    illust.tags
                        .into_iter()
                        .flat_map(|tag| {
                            let name = if let Some(alias) = STORAGE.lookup_alias(&tag.name) {
                                // Try to look for alias 
                                alias
                            } else if let Some(translation) = tag.translated_name {
                                // Use translated name, if it exists
                                translation
                            } else if tag.name.is_ascii() {
                                // Use name, if it is ascii
                                tag.name.clone()
                            } else {
                                // TODO: Romaji
                                String::new()
                            };

                            Tag::new(&name, Some(tag.name), TagType::Tag)
                        })
                        .chain(Tag::new("pixiv_source", None, TagType::Metadata).into_iter())
                        .collect()
                }).await?;
                
                let meta = ElementMetadata {
                    src_link: Some(
                        format!("https://www.pixiv.net/artworks/{}", illust.id)
                    ),
                    src_time: Some(illust.create_date),
                    ai_meta: None,
                    group: Some(illust_id as i64),
                    tags
                };
                
                Ok(Some(meta))
            }
            StatusCode::NOT_FOUND => Ok(None),
            _ => bail!(resp.error_for_status().unwrap_err())
        }
    }
}
