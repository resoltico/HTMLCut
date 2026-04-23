mod catalog;
mod schema;
mod shared;

#[cfg(test)]
use crate::model::CatalogAvailability;
use crate::model::{CatalogCommandReport, SchemaCommandReport};

pub(crate) fn render_catalog_text(report: &CatalogCommandReport) -> String {
    catalog::render_catalog_text(report)
}

pub(crate) fn render_schema_text(report: &SchemaCommandReport) -> String {
    schema::render_schema_text(report)
}

#[cfg(test)]
pub(crate) fn render_catalog_surface(
    command: Option<&str>,
    availability: &CatalogAvailability,
) -> String {
    catalog::render_catalog_surface(command, availability)
}
