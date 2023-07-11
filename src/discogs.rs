use serde;
use serde::{Serialize, Deserialize};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use nonzero_ext::nonzero;
use itertools::Itertools;

// TODO: codegen these files?
// https://github.com/OpenAPITools/openapi-generator/issues/15549

#[derive(Debug, Serialize, Deserialize)]
struct Pagination {
    items: u32,
    page: u32,
    pages: u32,
    per_page: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Release {
    barcode: Vec<String>,
    catno: String,
    country: String,
    title: String,
    year: String,
}

#[derive(Debug, Serialize)]
struct SearchQuery<'a> {
    barcode: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum SearchItem {
    Release(Release),
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResponse {
    pagination: Pagination,
    results: Vec<SearchItem>,
}

#[derive(Debug)]
pub struct OutputItem {
    pub artist: String,
    pub title: String,
    pub catalogue_number: String,
    pub year: String,
}

pub struct Client {
    http_client: reqwest::blocking::Client,
    contact: String,
    api_key: String,
    api_key_secret: String,
    rate_limiter: DefaultDirectRateLimiter,
}

fn search_item_to_output_item(search_item: &SearchItem) -> OutputItem {
    match search_item {
        SearchItem::Release(x) => {
            let mut parts = x.title.split(" - ");
            let raw_artist = parts.next().unwrap();
            let raw_title = parts.next().unwrap();
            let artist = if raw_artist.ends_with(")") { raw_artist.rsplit_once(" (").unwrap().0 } else { raw_artist };
            OutputItem {
                artist: artist.to_string(),
                title: raw_title.to_string(),
                catalogue_number: x.catno.clone(),
                year: x.year.clone(),
            }
        }
    }
}

impl Client {
    pub fn search(&self, barcode: &str) -> Result<Option<OutputItem>, reqwest::Error> {
        self.rate_limiter.check().unwrap();

        let query = SearchQuery { barcode };
        let resp: SearchResponse = self.http_client.get("https://api.discogs.com/database/search")
            .query(&query)
            .header("User-Agent", format!("cdscanner/0.0.1 +{}", self.contact))
            .header("Authorization", format!("Discogs key={}, secret={}", self.api_key, self.api_key_secret))
            .header("Accept", "application/vnd.discogs.v2.discogs+json")
            .send()
            ?.json()?;

        let mut output_items = resp.results.iter().map(search_item_to_output_item).unique_by(|x| x.catalogue_number.to_string());
        let first_item: Option<OutputItem> = output_items.next();
        Ok(first_item)
    }
}

pub fn make_client(contact: String, api_key: String, api_key_secret: String) -> Client {
    // "Requests are throttled by the server by source IP to 60 per minute for authenticated requests, and 25 per minute for unauthenticated requests, with some exceptions."
    // TODO: obey rate limit headers
    let rate_limiter = RateLimiter::direct(Quota::per_minute(nonzero!(60u32)));
    let http_client = reqwest::blocking::Client::new();
    Client { http_client, contact, api_key, api_key_secret, rate_limiter }
}
