use serde;
use serde::{Serialize, Deserialize};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use nonzero_ext::nonzero;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Torrent {
    format: String,
    encoding: String,
    pub has_log: bool,
    pub log_score: i8,
    pub time: String,
    pub remastered: bool,
    pub remaster_catalogue_number: String,
    pub remaster_title: String,
    pub remaster_year: u16,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    #[serde(rename = "artist")]
    pub artist_name: String,
    pub group_name: String,
    pub group_year: u16,
    pub torrents: Vec<Torrent>
}

#[derive(Debug, Serialize)]
pub struct SearchQuery<'a> {
    // mode 1
    #[serde(rename = "artistname")]
    artist_name: Option<&'a str>,
    #[serde(rename = "groupname")]
    group_name: Option<&'a str>,
    year: Option<u16>,
    // mode 2
    #[serde(rename = "recordlabel")]
    record_label: Option<&'a str>,
    #[serde(rename = "cataloguenumber")]
    catalogue_number: Option<&'a str>,
    // mode 3
    #[serde(rename = "searchstr")]
    search_str: Option<&'a str>,
}

impl<'a> SearchQuery<'a> {
    pub fn from_names(artist_name: &'a str, group_name: &'a str, year: u16) -> Self {
        Self { artist_name: Some(artist_name), group_name: Some(group_name), year: Some(year), record_label: None, catalogue_number: None, search_str: None }
    }

    pub fn from_catalog(record_label: Option<&'a str>, catalogue_number: &'a str) -> Self {
        Self { artist_name: None, group_name: None, year: None, record_label, catalogue_number: Some(catalogue_number), search_str: None }
    }

    pub fn from_search_str(search_str: &'a str) -> Self {
        Self { artist_name: None, group_name: None, year: None, record_label: None, catalogue_number: None, search_str: Some(search_str) }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    current_page: Option<u32>,
    pages: Option<u32>,
    results: Vec<Group>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct APIResponse<T> {
    status: String,
    response: T,
}

pub struct Client {
    http_client: reqwest::blocking::Client,
    api_key: String,
    rate_limiter: DefaultDirectRateLimiter,
}

impl Client {
    pub fn search(&self, query: &SearchQuery) -> Result<Option<Group>, reqwest::Error> {
        self.rate_limiter.check().unwrap();

        let resp: APIResponse<SearchResponse> = self.http_client.get("https://redacted.ch/ajax.php?action=browse&page=1&media=CD&format=FLAC")
            .query(&query)
            .header("User-Agent", "cdscanner/0.0.1")
            .header("Authorization", &self.api_key)
            .header("Accept", "application/json")
            .send()
            ?.json()?;

        Ok(resp.response.results.into_iter().next())
    }
}

pub fn make_client(api_key: String) -> Client {
    // "Refrain from making more than ten (10) requests every ten (10) seconds."
    let rate_limiter = RateLimiter::direct(
        Quota::per_second(nonzero!(1u32)).allow_burst(nonzero!(10u32))
    );
    let http_client = reqwest::blocking::Client::builder().connection_verbose(true).build().unwrap();
    Client { http_client, api_key, rate_limiter }
}
