use whippyunits::scale_conversion::{pow2, pow3, pow5};

// Keep the previous implementation as an independent oracle in this test target.
mod legacy {
    whippyunits_proc_macros::pow_lookup!(pow2, 2, 100, rational);
    whippyunits_proc_macros::pow_lookup!(pow3, 3, 100, rational);
    whippyunits_proc_macros::pow_lookup!(pow5, 5, 100, rational);
}

type Power = fn(i32) -> (i128, i128);
const PAIRS: [(Power, Power); 3] = [
    (pow2, legacy::pow2),
    (pow3, legacy::pow3),
    (pow5, legacy::pow5),
];

#[test]
fn rational_tables_match_values_and_overflow_behavior() {
    // Compare every supported exponent, out-of-range fallbacks, and extrema.
    // Compare panic/value outcomes rather than assuming debug_assertions tells
    // us whether overflow checking was enabled for this compilation.
    for (actual, expected) in PAIRS {
        for exp in (-105..=105).chain([i32::MIN, i32::MAX]) {
            let exp = std::hint::black_box(exp);
            let actual = std::panic::catch_unwind(|| actual(exp)).map_err(|_| ());
            let expected = std::panic::catch_unwind(|| expected(exp)).map_err(|_| ());
            assert_eq!(actual, expected, "exponent {exp}");
        }
    }
}

const CONST_ACTUAL: [(i128, i128); 7] = [
    pow2(-100),
    pow2(100),
    pow3(-80),
    pow3(80),
    pow5(-54),
    pow5(54),
    pow5(i32::MIN),
];
const CONST_EXPECTED: [(i128, i128); 7] = [
    legacy::pow2(-100),
    legacy::pow2(100),
    legacy::pow3(-80),
    legacy::pow3(80),
    legacy::pow5(-54),
    legacy::pow5(54),
    legacy::pow5(i32::MIN),
];

#[test]
fn rational_tables_preserve_const_boundary_values() {
    assert_eq!(CONST_ACTUAL, CONST_EXPECTED);
}
