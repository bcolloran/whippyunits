// Known compiler crash on nightly-2026-09-13 with rustc --test.
// This is a bug reproducer, not an expected-rejection safety test.
#![allow(incomplete_features)]
#![feature(min_generic_const_args, generic_const_args)]

#[test]
fn trivial() {}
