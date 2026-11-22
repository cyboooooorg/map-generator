use super::*;

#[test]
fn the_highland_ramp_is_smooth_between_its_edges() {
    assert_eq!(highland_weight(HIGHLANDS.edge.0 - 0.1), 0.0);
    assert_eq!(highland_weight(HIGHLANDS.edge.1 + 0.1), 1.0);
    let middle = (HIGHLANDS.edge.0 + HIGHLANDS.edge.1) / 2.0;
    assert!((highland_weight(middle) - 0.5).abs() < 1e-12);
    assert!(highland_weight(0.26) < highland_weight(0.30));
}
