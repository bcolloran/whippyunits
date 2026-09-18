use whippyunits::scale_conversion::{pow_pi_float, pow2_float, pow3_float, pow5_float};

// Keep the former generated expressions as an independent compatibility oracle.
// They are generated only in this test target, not in downstream library builds.
mod legacy {
    whippyunits_proc_macros::pow_lookup!(pow2_float, 2.0, 100, float);
    whippyunits_proc_macros::pow_lookup!(pow3_float, 3.0, 100, float);
    whippyunits_proc_macros::pow_lookup!(pow5_float, 5.0, 100, float);
    whippyunits_proc_macros::pow_pi_lookup!(pow_pi_float, 8, float);
}

// Cover every supported exponent, values beyond both table boundaries, and
// signed extrema (especially i32::MIN, which cannot be negated).
const EXPONENTS: [i32; 243] = {
    let mut exponents = [0; 243];
    let mut index = 0;
    while index < 241 {
        exponents[index] = index as i32 - 120;
        index += 1;
    }
    exponents[241] = i32::MIN;
    exponents[242] = i32::MAX;
    exponents
};

const ACTUAL_BITS: [[u64; 4]; 243] = {
    let mut bits = [[0; 4]; 243];
    let mut index = 0;
    while index < EXPONENTS.len() {
        let exponent = EXPONENTS[index];
        bits[index] = [
            pow2_float(exponent).to_bits(),
            pow3_float(exponent).to_bits(),
            pow5_float(exponent).to_bits(),
            pow_pi_float(exponent).to_bits(),
        ];
        index += 1;
    }
    bits
};

const EXPECTED_BITS: [[u64; 4]; 243] = {
    let mut bits = [[0; 4]; 243];
    let mut index = 0;
    while index < EXPONENTS.len() {
        let exponent = EXPONENTS[index];
        bits[index] = [
            legacy::pow2_float(exponent).to_bits(),
            legacy::pow3_float(exponent).to_bits(),
            legacy::pow5_float(exponent).to_bits(),
            legacy::pow_pi_float(exponent).to_bits(),
        ];
        index += 1;
    }
    bits
};

#[test]
fn float_power_tables_preserve_const_results_bit_for_bit() {
    for (index, exponent) in EXPONENTS.into_iter().enumerate() {
        assert_eq!(ACTUAL_BITS[index], EXPECTED_BITS[index], "{exponent}");
    }
}

#[test]
fn float_power_tables_preserve_runtime_results_bit_for_bit() {
    for (index, exponent) in EXPONENTS.into_iter().enumerate() {
        let exponent = std::hint::black_box(exponent);
        let actual = [
            pow2_float(exponent).to_bits(),
            pow3_float(exponent).to_bits(),
            pow5_float(exponent).to_bits(),
            pow_pi_float(exponent).to_bits(),
        ];
        assert_eq!(actual, EXPECTED_BITS[index], "{exponent}");
    }
}
