//! Lazily read the source catalog embedded by the browser-session build.
//!
//! Each worker shares one reader across all of its games. Only its compressed
//! index and the reader's bounded set of requested card chunks are inflated.

use std::cell::RefCell;

use ironsmith_card_catalog::Catalog;
use wasm_bindgen::prelude::*;

use crate::WasmGame;

include!(concat!(env!("OUT_DIR"), "/embedded_card_catalog_bytes.rs"));

struct EmbeddedCatalog<'a> {
    catalog: Option<Catalog<'a>>,
}

impl<'a> EmbeddedCatalog<'a> {
    fn from_bytes(bytes: Option<&'a [u8]>) -> Result<Self, String> {
        let catalog = bytes
            .map(Catalog::from_bytes)
            .transpose()
            .map_err(|error| format!("invalid embedded card catalog: {error}"))?;
        Ok(Self { catalog })
    }

    fn index_json(&mut self) -> Result<Option<String>, String> {
        self.catalog
            .as_mut()
            .map(|catalog| catalog.index_json().map(str::to_owned))
            .transpose()
            .map_err(|error| format!("failed to read embedded card catalog index: {error}"))
    }

    fn source_json(&mut self, route: &str) -> Result<Option<String>, String> {
        let Some(catalog) = self.catalog.as_mut() else {
            return Ok(None);
        };
        catalog
            .route_json(route)
            .map(|source| source.map(str::to_owned))
            .map_err(|error| format!("failed to read embedded card source {route:?}: {error}"))
    }
}

thread_local! {
    // Thread-local initialization is lazy. Native callers are isolated safely;
    // browser calls share the bounded reader inside their engine worker.
    static CATALOG: RefCell<Result<EmbeddedCatalog<'static>, String>> =
        RefCell::new(EmbeddedCatalog::from_bytes(EMBEDDED_CARD_CATALOG_BYTES));
}

fn read_catalog<T>(
    read: impl FnOnce(&mut EmbeddedCatalog<'static>) -> Result<T, String>,
) -> Result<T, String> {
    CATALOG.with(|state| {
        let mut state = state.borrow_mut();
        match state.as_mut() {
            Ok(catalog) => read(catalog),
            Err(error) => Err(error.clone()),
        }
    })
}

#[wasm_bindgen]
impl WasmGame {
    /// Read the embedded browsing index, or return None for an unbundled build.
    #[wasm_bindgen(js_name = getEmbeddedCardCatalogIndexJson)]
    pub fn get_embedded_card_catalog_index_json(&self) -> Result<Option<String>, JsValue> {
        read_catalog(EmbeddedCatalog::index_json).map_err(|error| JsValue::from_str(&error))
    }

    /// Read one catalog route without inflating the complete source catalog.
    #[wasm_bindgen(js_name = getEmbeddedCardSourceJson)]
    pub fn get_embedded_card_source_json(&self, route: String) -> Result<Option<String>, JsValue> {
        read_catalog(|catalog| catalog.source_json(&route))
            .map_err(|error| JsValue::from_str(&error))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith_card_catalog::builder::BundleBuilder;

    #[test]
    fn embedded_card_catalog_preserves_complete_route_documents() {
        let index = r#"{"names":["Fixture Front","Fixture Back"],"linked_face_groups":[["Fixture Front","Fixture Back"]]}"#;
        let front = r#"{"name":"Fixture Front","oracle_text":"First line.\nSecond line.","compiled":{"abilities":[{"kind":"test"}]},"metadata":{"set":"TST","collector_number":"1a","image_uris":{"normal":"front.png"}}}"#;
        let back = r#"{"name":"Fixture Back","oracle_text":"Back text.","metadata":{"collector_number":"1b"}}"#;
        let mut builder = BundleBuilder::new(index.as_bytes()).unwrap();
        builder
            .add_route("fixture-front", front.as_bytes())
            .unwrap();
        builder.add_route("fixture-back", back.as_bytes()).unwrap();
        let bytes = builder.finish().unwrap();
        let mut catalog = EmbeddedCatalog::from_bytes(Some(&bytes)).unwrap();

        assert_eq!(catalog.index_json().unwrap().as_deref(), Some(index));
        let retained_front = catalog.source_json("fixture-front.json").unwrap();
        assert_eq!(retained_front.as_deref(), Some(front));
        assert_eq!(
            catalog.source_json("fixture-back").unwrap().as_deref(),
            Some(back)
        );
        assert_eq!(catalog.source_json("missing.json").unwrap(), None);
        assert_eq!(catalog.source_json("../fixture-front.json").unwrap(), None);
        assert_eq!(retained_front.as_deref(), Some(front));
    }

    #[test]
    fn embedded_card_catalog_absence_is_supported() {
        let mut catalog = EmbeddedCatalog::from_bytes(None).unwrap();
        assert_eq!(catalog.index_json().unwrap(), None);
        assert_eq!(catalog.source_json("missing.json").unwrap(), None);
    }

    #[test]
    fn embedded_card_catalog_invalid_bytes_fail_explicitly() {
        let error = EmbeddedCatalog::from_bytes(Some(b"not a catalog"))
            .err()
            .expect("corrupt embedded catalog must not look like a missing catalog");
        assert!(error.starts_with("invalid embedded card catalog:"));
    }
}
