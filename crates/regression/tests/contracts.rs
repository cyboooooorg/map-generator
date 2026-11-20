//! Contracts between the engines, asserted on the shared reference systems.
//! Placeholder until the reference systems land with slice 1, PR 2.

#[test]
fn workspace_builds() {
    // Links every engine, so tier 2 has a test to run and a broken
    // dependency graph fails here before PR 2 adds the real contracts.
    let _ = (regression::REFERENCE_SEEDS,);
}
