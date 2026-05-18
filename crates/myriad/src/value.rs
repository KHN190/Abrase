use polka::Value;

#[derive(Clone, Debug)]
pub enum BoxedValue {
    String(String),
    // i64 values outside the inline i48 range. Result of arithmetic that
    // overflows the NaN-box int payload.
    Int(i64),
}

// TODO: per-VM pool; module-owned; process-shared?
pub struct BoxPool {
    slots: Vec<Option<BoxedValue>>,
    rc: Vec<u32>,
    bytes: Vec<usize>,
    free_list: Vec<u32>,
    bytes_used: usize,
}

pub fn boxed_value_bytes(b: &BoxedValue) -> usize {
    let base = std::mem::size_of::<BoxedValue>();
    match b {
        BoxedValue::String(s) => base + s.capacity(),
        _ => base,
    }
}

impl BoxPool {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            rc: Vec::new(),
            bytes: Vec::new(),
            free_list: Vec::new(),
            bytes_used: 0,
        }
    }

    pub fn bytes_used(&self) -> usize { self.bytes_used }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.rc.clear();
        self.bytes.clear();
        self.free_list.clear();
        self.bytes_used = 0;
    }

    // Bytes the next intern of `b` would charge
    pub fn pending_bytes(b: &BoxedValue) -> usize { boxed_value_bytes(b) }

    pub fn intern(&mut self, b: BoxedValue) -> u32 {
        let cost = boxed_value_bytes(&b);
        self.bytes_used = self.bytes_used.saturating_add(cost);
        if let Some(idx) = self.free_list.pop() {
            self.slots[idx as usize] = Some(b);
            self.rc[idx as usize] = 1;
            self.bytes[idx as usize] = cost;
            idx
        } else {
            let idx = self.slots.len() as u32;
            self.slots.push(Some(b));
            self.rc.push(1);
            self.bytes.push(cost);
            idx
        }
    }

    pub fn get(&self, idx: u32) -> Option<&BoxedValue> {
        self.slots.get(idx as usize).and_then(|s| s.as_ref())
    }

    pub fn get_mut(&mut self, idx: u32) -> Option<&mut BoxedValue> {
        self.slots.get_mut(idx as usize).and_then(|s| s.as_mut())
    }

    #[inline]
    pub fn inc(&mut self, idx: u32) -> Result<(), String> {
        let i = idx as usize;
        if i >= self.rc.len() {
            return Err(format!("BoxPool::inc: slot {} out of range", idx));
        }
        self.rc[i] = self.rc[i].checked_add(1)
            .ok_or_else(|| format!("BoxPool::inc: refcount overflow at slot {}", idx))?;
        Ok(())
    }

    #[inline]
    pub fn dec(&mut self, idx: u32) -> bool {
        let i = idx as usize;
        if i >= self.rc.len() || self.rc[i] == 0 { return false; }
        self.rc[i] -= 1;
        if self.rc[i] == 0 {
            self.slots[i] = None;
            self.bytes_used = self.bytes_used.saturating_sub(self.bytes[i]);
            self.bytes[i] = 0;
            self.free_list.push(idx);
            true
        } else {
            false
        }
    }

    pub fn free(&mut self, idx: u32) {
        if (idx as usize) < self.slots.len() && self.slots[idx as usize].is_some() {
            self.slots[idx as usize] = None;
            if (idx as usize) < self.rc.len() { self.rc[idx as usize] = 0; }
            let i = idx as usize;
            if i < self.bytes.len() {
                self.bytes_used = self.bytes_used.saturating_sub(self.bytes[i]);
                self.bytes[i] = 0;
            }
            self.free_list.push(idx);
        }
    }

    pub fn live_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    // Inline if it fits i48, otherwise intern as `BoxedValue::Int`
    #[inline]
    pub fn intern_int(&mut self, n: i64) -> Value {
        if Value::fits_i48(n) {
            Value::from_int(n)
        } else {
            let idx = self.intern(BoxedValue::Int(n));
            Value::from_box(idx)
        }
    }

    // Read i64 from inline TAG_INT or from BoxedValue::Int (i48 overflow path).
    #[inline]
    pub fn read_int(&self, v: Value) -> Option<i64> {
        if let Some(n) = v.as_int() { return Some(n); }
        if let Some(idx) = v.as_box() {
            if let Some(BoxedValue::Int(n)) = self.get(idx) { return Some(*n); }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_pool_round_trip() {
        let mut pool = BoxPool::new();
        let idx = pool.intern(BoxedValue::String("hi".into()));
        assert!(matches!(pool.get(idx), Some(BoxedValue::String(s)) if s == "hi"));
        pool.free(idx);
        assert!(pool.get(idx).is_none());
        let idx2 = pool.intern(BoxedValue::String("re".into()));
        assert_eq!(idx2, idx);
    }

    #[test]
    fn box_pool_rc_reclaims_at_zero() {
        let mut pool = BoxPool::new();
        let idx = pool.intern(BoxedValue::String("a".into()));    // rc=1
        pool.inc(idx).unwrap();                                   // rc=2
        assert_eq!(pool.live_count(), 1);
        assert!(!pool.dec(idx));                                  // rc=1, not freed
        assert!(pool.get(idx).is_some());
        assert!(pool.dec(idx));                                   // rc=0, freed
        assert!(pool.get(idx).is_none());
        assert_eq!(pool.live_count(), 0);
    }

    #[test]
    fn box_pool_inc_rejects_out_of_range() {
        let mut pool = BoxPool::new();
        let err = pool.inc(99).expect_err("out-of-range slot must err");
        assert!(err.contains("out of range"), "msg: {}", err);
    }

    #[test]
    fn box_pool_inc_rejects_overflow() {
        let mut pool = BoxPool::new();
        let idx = pool.intern(BoxedValue::String("x".into()));
        pool.rc[idx as usize] = u32::MAX;
        let err = pool.inc(idx).expect_err("rc overflow must err");
        assert!(err.contains("overflow"), "msg: {}", err);
    }

    #[test]
    fn box_pool_reuses_slot_after_dec() {
        let mut pool = BoxPool::new();
        let a = pool.intern(BoxedValue::String("a".into()));
        pool.dec(a);
        let b = pool.intern(BoxedValue::String("b".into()));
        assert_eq!(a, b);
        assert_eq!(pool.live_count(), 1);
    }

    #[test]
    fn intern_int_inline_path_for_small_values() {
        let mut pool = BoxPool::new();
        let v = pool.intern_int(42);
        assert_eq!(v.as_int(), Some(42));
        assert_eq!(pool.read_int(v), Some(42));
        assert_eq!(pool.live_count(), 0);
    }

    #[test]
    fn intern_int_boxes_i64_min() {
        let mut pool = BoxPool::new();
        let v = pool.intern_int(i64::MIN);
        assert_eq!(v.as_int(), None);              // not inline
        assert_eq!(pool.read_int(v), Some(i64::MIN));
        assert_eq!(pool.live_count(), 1);
    }

    #[test]
    fn intern_int_boxes_just_outside_i48() {
        let mut pool = BoxPool::new();
        let n: i64 = 1i64 << 47;                   // I48_MAX + 1
        let v = pool.intern_int(n);
        assert_eq!(v.as_int(), None);
        assert_eq!(pool.read_int(v), Some(n));
    }
}
