//! Helpers shared by the tests of the frontends.

use neon_engine::text::{Catalog, Lang};

pub(crate) fn catalog_en() -> Catalog {
    Catalog::embedded(Lang::En).unwrap()
}

pub(crate) fn catalog_fr() -> Catalog {
    Catalog::embedded(Lang::Fr).unwrap()
}
