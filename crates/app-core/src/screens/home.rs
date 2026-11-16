//! Home view model. SwiftUI renders it as is.

use crate::store::items::{Item, ItemStore};

#[derive(Clone, Debug, PartialEq)]
pub struct ItemRow {
    pub id: u32,
    pub title: String,
    pub subtitle: String,
    pub category: String,
    pub symbol: String,
    pub favorite: bool,
}

impl From<&Item> for ItemRow {
    fn from(i: &Item) -> Self {
        Self {
            id: i.id,
            title: i.title.clone(),
            subtitle: i.subtitle.clone(),
            category: i.category.clone(),
            symbol: i.symbol.clone(),
            favorite: i.favorite,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HomeViewModel {
    pub title: String,
    pub query: String,
    pub rows: Vec<ItemRow>,
    pub total_count: u32,
    pub favorite_count: u32,
    /// Text to show when the list is empty (e.g. no search results).
    pub empty_message: Option<String>,
}

impl HomeViewModel {
    pub fn build(store: &ItemStore, query: &str) -> Self {
        let rows: Vec<ItemRow> = store
            .visible(query)
            .into_iter()
            .map(ItemRow::from)
            .collect();
        let empty_message = rows
            .is_empty()
            .then(|| format!("No results for \"{}\"", query.trim()));
        Self {
            title: "MasterFabric".to_string(),
            query: query.to_string(),
            total_count: store.all().len() as u32,
            favorite_count: store.all().iter().filter(|i| i.favorite).count() as u32,
            rows,
            empty_message,
        }
    }
}
