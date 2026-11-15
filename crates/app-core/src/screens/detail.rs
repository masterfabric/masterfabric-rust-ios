//! Detail screen view model.

use super::home::ItemRow;
use crate::store::items::{Item, ItemStore};

#[derive(Clone, Debug, PartialEq)]
pub struct DetailViewModel {
    pub key: String,
    pub item: Item,
    pub favorite_label: String,
    pub related: Vec<ItemRow>,
}

impl DetailViewModel {
    pub fn build(store: &ItemStore, item_id: u32) -> Option<Self> {
        let item = store.get(item_id)?.clone();
        Some(Self {
            key: crate::Route::Detail { item_id }.key(),
            favorite_label: if item.favorite {
                "Remove from favorites"
            } else {
                "Add to favorites"
            }
            .to_string(),
            related: store
                .related(item_id, 3)
                .into_iter()
                .map(ItemRow::from)
                .collect(),
            item,
        })
    }
}
