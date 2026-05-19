pub struct Frame {
    pub func_id: usize,
    pub ip: usize,
    pub base_reg: usize,
    pub dest_reg: usize,
    pub dest_is_handle: bool,
    pub cont: Option<Box<ContinuationInfo>>,
}

pub struct ContinuationInfo {
    pub snapshot_slot: u32,
    pub snapshot_gen: u32,
    pub snapshot_count: usize,
}

impl Frame {
    #[inline]
    pub fn normal(func_id: usize, ip: usize, base_reg: usize, dest_reg: usize) -> Self {
        Self {
            func_id,
            ip,
            base_reg,
            dest_reg,
            dest_is_handle: false,
            cont: None,
        }
    }

    #[inline]
    pub fn arm_continuation(
        func_id: usize,
        ip: usize,
        base_reg: usize,
        dest_reg: usize,
        snapshot_slot: u32,
        snapshot_gen: u32,
        snapshot_count: usize,
    ) -> Self {
        Self {
            func_id,
            ip,
            base_reg,
            dest_reg,
            dest_is_handle: false,
            cont: Some(Box::new(ContinuationInfo {
                snapshot_slot,
                snapshot_gen,
                snapshot_count,
            })),
        }
    }

    #[inline]
    pub fn is_arm_continuation(&self) -> bool {
        self.cont.is_some()
    }
}
