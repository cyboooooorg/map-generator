//! The shortwave: the sunlight a column absorbs, reflects and lets
//! through (ported from `legacy-final`'s `planet/shortwave.rs`).

use std::sync::LazyLock;

use super::ktable::Table;
use super::starlight::{BANDS, band_edge_m};

/// The shortwave table of CO2's and water's lines
/// (`crates/science/data/shortwave.bin`, see [`super::ktable`]), with
/// MT_CKD 4.3's water and CO2 continua binned on its bins, checked
/// against the star's bands.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the shortwave paths of PR 2b-2 Task 4 read it")
)]
pub(crate) static SHORTWAVE: LazyLock<Table> = LazyLock::new(|| {
    let table = Table::parse(include_bytes!("../../data/shortwave.bin"))
        .expect("crates/science/data/shortwave.bin is a shortwave table of 8 g-points");
    assert!(
        !table.edges_cm.is_empty(),
        "shortwave.bin is a longwave table"
    );
    assert_eq!(table.water_continuum.len(), table.bins);
    let first = BANDS - table.bins;
    for (j, &edge) in table.edges_cm.iter().enumerate() {
        let lambda_m = band_edge_m(BANDS - j);
        assert!(
            (1e-2 / lambda_m / edge - 1.0).abs() < 1e-9,
            "the shortwave table's bins are not the star's bands"
        );
    }
    assert!(first > 0);
    table
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_on_the_albedo_bands() {
        let table = &*SHORTWAVE;
        assert_eq!(table.bins, 147);
        assert!((table.edges_cm[0] - 2000.0).abs() < 1e-6);
        assert!(table.edges_cm[table.bins] > 14_300.0);
        assert!((table.weights.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }
}
