//! Select generated conveniences without changing the dimensional type system
//! or the unit parser's vocabulary. Compound dimensions require all their axes.

use whippyunits_core::dimension_exponents::DynDimensionExponents;
use whippyunits_core::scale_exponents::ScaleExponents;
use whippyunits_core::{Dimension, SiPrefix, System, Unit};

pub fn broad_dimension_enabled(exponents: &DynDimensionExponents) -> bool {
    let [mass, length, time, current, temperature, amount, luminosity, angle] = exponents.0;
    (cfg!(feature = "units-dynamics") || [mass, length, time, angle] == [0; 4])
        && (cfg!(feature = "units-electrical") || current == 0)
        && (cfg!(feature = "units-thermal") || temperature == 0)
        && (cfg!(feature = "units-chemical") || amount == 0)
        && (cfg!(feature = "units-photometric") || luminosity == 0)
}

/// Select the coherent mechanical subset without changing existing broad catalogs.
fn coherent_dimension_enabled(exponents: &DynDimensionExponents) -> bool {
    cfg!(feature = "units-si-dynamics") && exponents.0[3..7] == [0; 4]
}

pub fn dimension_enabled(exponents: &DynDimensionExponents) -> bool {
    broad_dimension_enabled(exponents) || coherent_dimension_enabled(exponents)
}

/// Whether a resolved storage scale can have a generated convenience alias.
/// The broad catalogs retain their existing scales; the coherent subset only
/// includes unity scale, even when the spelling has a prefix (kilogram).
pub fn scale_enabled(exponents: &DynDimensionExponents, scale: ScaleExponents) -> bool {
    broad_dimension_enabled(exponents)
        || (coherent_dimension_enabled(exponents) && scale == ScaleExponents::IDENTITY)
}

/// Selection shared by default/local/branded declarators and literal suffixes.
/// Check the resolved scale: neither `Metric` nor `conversion_factor == 1`
/// alone distinguishes coherent SI units from minutes, degrees, liters, etc.
pub fn entry_enabled(unit: &Unit, prefix: Option<&SiPrefix>) -> bool {
    if broad_dimension_enabled(&unit.exponents) {
        return true;
    }
    let scale = match prefix {
        Some(prefix) => unit.scale.mul(ScaleExponents::_10(prefix.factor_log10())),
        None => unit.scale,
    };
    coherent_dimension_enabled(&unit.exponents)
        && unit.system == System::Metric
        && prefix.is_none_or(|prefix| unit.name == "gram" && *prefix == SiPrefix::KILO)
        && unit.conversion_factor == 1.0
        && unit.affine_offset == 0.0
        && scale == ScaleExponents::IDENTITY
}

/// Resolve the actual spelling before deciding whether a hover alias exists.
/// `get_unit_info("mm")` returns the underlying meter, so it cannot make this
/// decision without also accounting for the prefix.
pub fn identifier_enabled(identifier: &str) -> bool {
    if let Some((unit, _)) = Dimension::find_unit(identifier) {
        return entry_enabled(unit, None);
    }
    for prefix in SiPrefix::ALL {
        if let Some(unit) = prefix
            .strip_prefix_symbol(identifier)
            .and_then(Dimension::find_unit_by_symbol)
            .or_else(|| {
                prefix
                    .strip_prefix_name(identifier)
                    .and_then(Dimension::find_unit_by_name)
            })
        {
            return entry_enabled(unit.0, Some(prefix));
        }
    }
    false
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
            cfg!(any(
                feature = "units-dynamics",
                feature = "units-si-dynamics"
            ))
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
                cfg!(any(
                    feature = "units-dynamics",
                    feature = "units-si-dynamics"
                )),
                &["m", "kg", "rad", "J"][..],
            ),
            (
                cfg!(feature = "units-dynamics"),
                &["mm", "kN", "ft", "min", "deg", "L"][..],
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

    #[test]
    fn coherent_catalog_uses_resolved_scale_and_keeps_kilogram() {
        let gram = &Unit::GRAM.erase();
        let meter = &Unit::METER.erase();
        let liter = &Unit::LITER.erase();
        let broad = cfg!(feature = "units-dynamics");
        let coherent = broad || cfg!(feature = "units-si-dynamics");
        assert_eq!(entry_enabled(gram, Some(&SiPrefix::KILO)), coherent);
        assert_eq!(entry_enabled(gram, None), broad);
        assert_eq!(entry_enabled(meter, None), coherent);
        assert_eq!(entry_enabled(meter, Some(&SiPrefix::MILLI)), broad);
        // A kiloliter has unity scale but is not a coherent SI spelling.
        assert_eq!(entry_enabled(liter, Some(&SiPrefix::KILO)), broad);
        assert_eq!(identifier_enabled("kg"), coherent);
        assert_eq!(identifier_enabled("mm"), broad);
        assert_eq!(identifier_enabled("yard"), broad);
    }

    #[cfg(all(feature = "units-si-dynamics", not(feature = "units-dynamics")))]
    #[test]
    fn coherent_literal_catalog_has_only_the_ten_selected_spellings() {
        let mut expected = ["kg", "m", "s", "rad", "Hz", "N", "J", "Nm", "W", "Pa"]
            .map(String::from)
            .to_vec();
        expected.sort();
        assert_eq!(get_all_unit_symbols_for_literals(), expected);
    }
}
