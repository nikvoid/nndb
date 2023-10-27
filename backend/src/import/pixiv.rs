use anyhow::bail;
use moka::future::Cache;
use once_cell::sync::Lazy;
use pixivcrab::{AppApi, AppApiConfig, AuthMethod, models::illust::Illust};
use regex::Regex;
use reqwest::{ClientBuilder, Client, StatusCode};
use serde::Deserialize;

use crate::{model::{read::PendingImport, write::{ElementMetadata, Tag}, TagType}, CONFIG, dao::STORAGE, config::PixivCreds};

// Pixiv metadata fetcher
pub struct Pixiv {
    api: Option<AppApi>,
    client: Client,
    illust_cache: Cache<u64, Illust>
}


/// Base pixiv api url
const PIXIV_API_URL: &str = "https://app-api.pixiv.net";

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
            client: Client::new(),
            illust_cache: Cache::new(2048),
        }
    }

    /// Convert pixiv illust metadata to our metadata
    async fn extract_data(illust: Illust) -> ElementMetadata {

        // This should not fail because it was valid json
        let raw_meta = Some(serde_json::to_string(&illust).unwrap());

        async fn lookup_alias(alias: &str) -> Option<String> {
            let s = STORAGE.get()?;
            s.lookup_alias_async(alias).await
        }
        
        // Aliases can also contain artists
        let artist_name = if let Some(alias) = lookup_alias(&illust.user.name).await {
            alias
        } else {
            illust.user.account
        };
        
        let artist = Tag::new(&artist_name, Some(illust.user.name), TagType::Artist);
        
        let mut tags = vec![
            Tag::new("pixiv_source", None, TagType::Metadata).unwrap(),
            artist.unwrap_or_else(|| 
                Tag::new("stub_artist", None, TagType::Artist).unwrap()
            )
        ];
                
        for il_tag in illust.tags {
            let name = if let Some(alias) = lookup_alias(&il_tag.name).await {
                // Try to look for alias 
                alias
            } else if let Some(translation) = il_tag.translated_name {
                // Use translated name, if it exists
                translation
            } else if il_tag.name.is_ascii() {
                // Use name, if it is ascii
                il_tag.name.clone()
            } else {
                // Convert to romaji using dictionary
                kakasi::convert(&il_tag.name).romaji
            };
            
            if let Some(tag) = Tag::new(&name, Some(il_tag.name), TagType::Tag) {
                tags.push(tag);
            }
        }
        
        ElementMetadata {
            src_link: Some(
                format!("https://www.pixiv.net/artworks/{}", illust.id)
            ),
            src_time: Some(illust.create_date),
            raw_meta,
            group: Some(illust.id),
            tags
        }
    }
    
    /// Check if importer can get metadata for element
    /// Try to match typical web or app filename 
    pub fn supported(&self, import: &PendingImport) -> bool {
        APP_REX.is_match(&import.orig_filename) 
        || WEB_REX.is_match(&import.orig_filename)
    }
    
    /// Check if importer can fetch metadata now
    pub fn available(&self) -> bool { self.api.is_some() }
    
    /// Fetch metadata for pending import (network access implied)
    pub async fn fetch_metadata(
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

        // Look in cache first
        if let Some(illust) = self.illust_cache.get(&illust_id) {
            return Ok(Some(Self::extract_data(illust).await));
        }
    
        let request = self.client
            .get(format!("{PIXIV_API_URL}/v1/illust/detail?illust_id={illust_id}"));

        let resp = api.send_authorized(request).await?;    

        match resp.status() {
            StatusCode::OK => {
                let IllustResponse { illust } = resp.json().await?;

                self.illust_cache.insert(illust.id as u64, illust.clone()).await;
                
                let meta = Self::extract_data(illust).await;
                
                Ok(Some(meta))
            }
            StatusCode::NOT_FOUND => Ok(None),
            _ => bail!(resp.error_for_status().unwrap_err())
        }
    }
}

/// Wrapper for parsing
#[derive(Deserialize)]
struct IllustResponse {
    illust: Illust
}
