use musicbrainz_rs::entity::release::{Release, ReleaseSearchQuery};
use musicbrainz_rs::prelude::*;

pub fn search(barcode: &str) -> Result<Option<Release>, reqwest::Error> {
    let query = ReleaseSearchQuery::query_builder()
        .barcode(&barcode)
        .build();
    let query_result = Release::search(query).execute()?;
    println!("{:#?}", query_result);

    let mut output_items = query_result.entities.iter();
    let first_item: Option<Release> = output_items.next().cloned();
    Ok(first_item)
}
