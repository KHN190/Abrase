use myriad::frame::Frame;

#[test]
fn normal_frame_has_no_continuation() {
    let f = Frame::normal(3, 17, 64, 5);
    assert_eq!(f.func_id, 3);
    assert_eq!(f.ip, 17);
    assert_eq!(f.base_reg, 64);
    assert_eq!(f.dest_reg, 5);
    assert!(!f.dest_is_handle);
    assert!(!f.is_arm_continuation());
    assert!(f.cont.is_none());
}

#[test]
fn arm_continuation_frame_carries_snapshot() {
    let f = Frame::arm_continuation(9, 0, 128, 2, 42, 7, 16);
    assert_eq!(f.func_id, 9);
    assert_eq!(f.base_reg, 128);
    assert!(f.is_arm_continuation());
    let cont = f.cont.as_ref().unwrap();
    assert_eq!(cont.snapshot_slot, 42);
    assert_eq!(cont.snapshot_gen, 7);
    assert_eq!(cont.snapshot_count, 16);
}

#[test]
fn dest_is_handle_defaults_false_for_both_constructors() {
    assert!(!Frame::normal(0, 0, 0, 0).dest_is_handle);
    assert!(!Frame::arm_continuation(0, 0, 0, 0, 0, 0, 0).dest_is_handle);
}
