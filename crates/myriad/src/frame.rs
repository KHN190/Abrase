pub struct Frame {
    pub func_id: usize,
    pub ip: usize,
    pub base_reg: usize,
    pub dest_reg: usize,
    // True when this frame was pushed by a non-tail `resume` (arm body keeps
    // computing after the continuation runs to completion). When popped, the
    // active handler's `pending_return_arm` decides whether to route the
    // popped value through the return arm before delivering.
    pub is_arm_continuation: bool,
    // Snapshot of the arm's register window, taken at Resume time so the arm
    // can be re-entered cleanly even if subsequent yields stomp on the same
    // physical slots. (slot, gen) — `slot == 0` means no snapshot.
    pub arm_snapshot_slot: u32,
    pub arm_snapshot_gen: u32,
    pub arm_snapshot_count: usize,
}

impl Frame {
    #[inline]
    pub fn normal(func_id: usize, ip: usize, base_reg: usize, dest_reg: usize) -> Self {
        Self {
            func_id,
            ip,
            base_reg,
            dest_reg,
            is_arm_continuation: false,
            arm_snapshot_slot: 0,
            arm_snapshot_gen: 0,
            arm_snapshot_count: 0,
        }
    }
}
