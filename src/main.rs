use env_logger;
use rouille;
use serde::Deserialize;
use std::fs::read_to_string;
use std::sync::Mutex;
use chrono::Datelike;

mod discogs;
mod musicbrainz;
mod whatapi;

#[derive(Deserialize)]
struct Secrets {
    what_api_key: String,
    discogs_contact: String,
    discogs_api_key: String,
    discogs_api_key_secret: String,
}

fn get_info(what: &whatapi::Client, discogs: &discogs::Client, barcode: String) -> Result<Option<whatapi::Group>, reqwest::Error> {
    let discogs_result = discogs.search(&barcode)?;

    if let Some(ref info) = discogs_result {
        if let Some(what_from_catalog) = what.search(&whatapi::SearchQuery::from_catalog(None, &info.catalogue_number))? {
            return Ok(Some(what_from_catalog));
        }
        if let Some(what_from_names) = what.search(&whatapi::SearchQuery::from_names(&info.artist, &info.title, info.year.parse::<u16>().unwrap()))? {
            return Ok(Some(what_from_names));
        }
    }

    let musicbrainz_result = musicbrainz::search(&barcode)?;

    if let Some(ref info) = musicbrainz_result {
        if let Some(catalog_number) = info.label_info.as_ref().and_then(|x| x.get(0)).and_then(|x| x.catalog_number.clone()) {
            if let Some(what_from_catalog) = what.search(&whatapi::SearchQuery::from_catalog(None, &catalog_number))? {
                return Ok(Some(what_from_catalog));
            }
        }
        if let (Some(date), Some(first_artist)) = (info.date, info.artist_credit.as_ref().and_then(|x| x.get(0))) {
            if let Some(what_from_names) = what.search(&whatapi::SearchQuery::from_names(&first_artist.name, &info.title, date.year().try_into().unwrap()))? {
                return Ok(Some(what_from_names));
            }
        }
    }

    Ok(None)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let secrets: Secrets = toml::from_str(&read_to_string("secrets.toml")?)?;

    let what = Mutex::new(whatapi::make_client(secrets.what_api_key));
    let discogs = Mutex::new(discogs::make_client(secrets.discogs_contact, secrets.discogs_api_key, secrets.discogs_api_key_secret));

    rouille::start_server("localhost:51337", move |request| -> rouille::Response {
        rouille::router!(
            request,
            (GET) (/info/{barcode: String}) => {
                let release_group = rouille::try_or_400!(get_info(&what.lock().unwrap(), &discogs.lock().unwrap(), barcode));
                match release_group {
                    Some(group) => rouille::Response::json(&group),
                    None => rouille::Response::empty_404(),
                }
            },
            (GET) (/search) => {
                let search_str = request.get_param("q");
                match search_str {
                    Some(search_str) => {
                        let release_group = rouille::try_or_400!(what.lock().unwrap().search(&whatapi::SearchQuery::from_search_str(&search_str)));
                        match release_group {
                            Some(group) => rouille::Response::json(&group),
                            None => rouille::Response::empty_404(),
                        }
                    }
                    None => rouille::Response::empty_404(),
                }
            },
            _ => rouille::Response::empty_404()
        )
    });
}
