//! Select generated conveniences without changing the dimensional type system
//! or the unit parser's vocabulary. Compound dimensions require all their axes.

use whippyunits_core::dimension_exponents::DynDimensionExponents;

pub fn dimension_enabled(exponents: &DynDimensionExponents) -> bool {
    let [mass, length, time, current, temperature, amount, luminosity, angle] = exponents.0;
    (cfg!(feature = "units-dynamics") || [mass, length, time, angle] == [0; 4])
        && (cfg!(feature = "units-electrical") || current == 0)
        && (cfg!(feature = "units-thermal") || temperature == 0)
        && (cfg!(feature = "units-chemical") || amount == 0)
        && (cfg!(feature = "units-photometric") || luminosity == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::literal_macros::get_all_unit_symbols_for_literals;

    #[test]
    fn compound_dimensions_require_every_nonzero_axis() {
        assert!(dimension_enabled(&DynDimensionExponents([0; 8])));
        assert_eq!(
            dimension_enabled(&DynDimensionExponents([1, 1, -2, 0, 0, 0, 0, 0])),
            cfg!(feature = "units-dynamics")
        );
        assert_eq!(
            dimension_enabled(&DynDimensionExponents([1, 2, -3, -1, -1, 0, 0, 0])),
            cfg!(all(feature = "units-electrical", feature = "units-thermal"))
        );
    }

    #[test]
    fn symbols_follow_catalog_selection_including_prefixes_and_derived_units() {
        let symbols = get_all_unit_symbols_for_literals();
        for (feature, names) in [
            (
                cfg!(feature = "units-dynamics"),
                &["m", "kg", "rad", "kN", "J"][..],
            ),
            (
                cfg!(feature = "units-electrical"),
                &["A", "mA", "C", "V"][..],
            ),
            (cfg!(feature = "units-thermal"), &["K", "mK", "celsius"][..]),
            (cfg!(feature = "units-chemical"), &["mol", "mmol"][..]),
            (
                cfg!(feature = "units-photometric"),
                &["cd", "mcd", "lx"][..],
            ),
        ] {
            for name in names {
                assert_eq!(
                    symbols.iter().any(|symbol| symbol == name),
                    feature,
                    "{name}"
                );
            }
        }
    }
}
