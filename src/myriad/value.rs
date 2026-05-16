use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub u64);

const QNAN_MASK: u64    = 0x7FF8_0000_0000_0000;
const TAG_SHIFT: u64    = 48;
const TAG_MASK: u64     = 0x7 << TAG_SHIFT;
const PAYLOAD_MASK: u64 = (1u64 << 48) - 1;

const TAG_INT: u64       = 0;
const TAG_BOOL: u64      = 1;
const TAG_CHAR: u64      = 2;
const TAG_UNIT: u64      = 3;
const TAG_HANDLE: u64    = 4;
const TAG_NONE: u64      = 5;
const TAG_BOX: u64       = 6;
const TAG_STR_CONST: u64 = 7;

const I48_MIN: i64 = -(1i64 << 47);
const I48_MAX: i64 = (1i64 << 47) - 1;

impl Value {
    pub const NONE:  Value = Value(QNAN_MASK | (TAG_NONE  << TAG_SHIFT));
    pub const UNIT:  Value = Value(QNAN_MASK | (TAG_UNIT  << TAG_SHIFT));
    pub const TRUE:  Value = Value(QNAN_MASK | (TAG_BOOL  << TAG_SHIFT) | 1);
    pub const FALSE: Value = Value(QNAN_MASK | (TAG_BOOL  << TAG_SHIFT));

    #[inline(always)]
    pub fn is_none(self) -> bool { self.0 == Self::NONE.0 }

    #[inline(always)]
    pub fn is_float(self) -> bool { (self.0 & QNAN_MASK) != QNAN_MASK }

    #[inline(always)]
    fn tag(self) -> u64 { (self.0 & TAG_MASK) >> TAG_SHIFT }

    #[inline(always)]
    pub fn fits_i48(n: i64) -> bool { (I48_MIN..=I48_MAX).contains(&n) }

    // Inline-only Int constructor. Caller must pre-check range; outside i48 the
    // bits are masked (sandbox-safe but value-lossy). Use `from_int_or_box` for
    // correct overflow handling.
    #[inline(always)]
    pub fn from_int(n: i64) -> Value {
        debug_assert!(Self::fits_i48(n), "from_int: {} outside i48; use from_int_or_box", n);
        let payload = (n as u64) & PAYLOAD_MASK;
        Value(QNAN_MASK | (TAG_INT << TAG_SHIFT) | payload)
    }

    // Inline if it fits i48, otherwise intern as `BoxedValue::Int` and return a TAG_BOX.
    #[inline]
    pub fn from_int_or_box(pool: &mut BoxPool, n: i64) -> Value {
        if Self::fits_i48(n) {
            Self::from_int(n)
        } else {
            let idx = pool.intern(BoxedValue::Int(n));
            Self::from_box(idx)
        }
    }

    #[inline(always)]
    pub fn from_float(f: f64) -> Value {
        if f.is_nan() {
            Value(0x7FF0_0000_0000_0001)
        } else {
            Value(f.to_bits())
        }
    }

    #[inline(always)]
    pub fn from_bool(b: bool) -> Value { if b { Self::TRUE } else { Self::FALSE } }

    #[inline(always)]
    pub fn from_char(c: char) -> Value {
        Value(QNAN_MASK | (TAG_CHAR << TAG_SHIFT) | (c as u64))
    }

    #[inline(always)]
    pub fn from_handle(slot: u32, generation: u32) -> Value {
        let s = (slot as u64) & 0x00FF_FFFF;
        let g = (generation as u64) & 0x00FF_FFFF;
        Value(QNAN_MASK | (TAG_HANDLE << TAG_SHIFT) | (s << 24) | g)
    }

    #[inline(always)]
    pub fn from_box(idx: u32) -> Value {
        Value(QNAN_MASK | (TAG_BOX << TAG_SHIFT) | (idx as u64))
    }

    #[inline(always)]
    pub fn from_str_const(idx: u32) -> Value {
        Value(QNAN_MASK | (TAG_STR_CONST << TAG_SHIFT) | (idx as u64))
    }

    #[inline(always)]
    pub fn as_int(self) -> Option<i64> {
        if self.is_float() || self.tag() != TAG_INT { return None; }
        let payload = self.0 & PAYLOAD_MASK;
        Some(((payload as i64) << 16) >> 16)
    }

    // Read i64 from inline TAG_INT or from a boxed BoxedValue::Int (i48 overflow path).
    #[inline]
    pub fn as_int_full(self, pool: &BoxPool) -> Option<i64> {
        if let Some(n) = self.as_int() { return Some(n); }
        if let Some(idx) = self.as_box() {
            if let Some(BoxedValue::Int(n)) = pool.get(idx) { return Some(*n); }
        }
        None
    }

    #[inline(always)]
    pub fn as_float(self) -> Option<f64> {
        if self.is_float() { Some(f64::from_bits(self.0)) } else { None }
    }

    #[inline(always)]
    pub fn as_bool(self) -> Option<bool> {
        if !self.is_float() && self.tag() == TAG_BOOL { Some((self.0 & 1) != 0) } else { None }
    }

    #[inline(always)]
    pub fn as_char(self) -> Option<char> {
        if !self.is_float() && self.tag() == TAG_CHAR {
            char::from_u32((self.0 & 0x1F_FFFF) as u32)
        } else { None }
    }

    #[inline(always)]
    pub fn as_handle(self) -> Option<(u32, u32)> {
        if !self.is_float() && self.tag() == TAG_HANDLE {
            let p = self.0 & PAYLOAD_MASK;
            Some((((p >> 24) & 0x00FF_FFFF) as u32, (p & 0x00FF_FFFF) as u32))
        } else { None }
    }

    #[inline(always)]
    pub fn as_box(self) -> Option<u32> {
        if !self.is_float() && self.tag() == TAG_BOX {
            Some((self.0 & PAYLOAD_MASK) as u32)
        } else { None }
    }

    #[inline(always)]
    pub fn as_str_const(self) -> Option<u32> {
        if !self.is_float() && self.tag() == TAG_STR_CONST {
            Some((self.0 & PAYLOAD_MASK) as u32)
        } else { None }
    }

    #[inline(always)] pub fn is_unit(self)      -> bool { !self.is_float() && self.tag() == TAG_UNIT }
    #[inline(always)] pub fn is_handle(self)    -> bool { !self.is_float() && self.tag() == TAG_HANDLE }
    #[inline(always)] pub fn is_box(self)       -> bool { !self.is_float() && self.tag() == TAG_BOX }
    #[inline(always)] pub fn is_str_const(self) -> bool { !self.is_float() && self.tag() == TAG_STR_CONST }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_none() { return write!(f, "None"); }
        if let Some(n) = self.as_int()   { return write!(f, "Int({})", n); }
        if let Some(x) = self.as_float() { return write!(f, "Float({})", x); }
        if let Some(b) = self.as_bool()  { return write!(f, "Bool({})", b); }
        if let Some(c) = self.as_char()  { return write!(f, "Char({:?})", c); }
        if self.is_unit() { return write!(f, "Unit"); }
        if let Some((s, g)) = self.as_handle() { return write!(f, "Handle({},{})", s, g); }
        if let Some(i) = self.as_box() { return write!(f, "Box({})", i); }
        write!(f, "Value({:#x})", self.0)
    }
}

#[derive(Clone, Debug)]
pub enum BoxedValue {
    String(String),
    Closure { func_id: usize, env_slot: u32, env_gen: u32 },
    Reference(Value),
    // i64 values outside the inline i48 range. Result of arithmetic that
    // overflows the NaN-box int payload.
    Int(i64),
}

// Design: per-VM pool (option A). B/C (module-owned / process-shared) still on the table — revisit when cartridge serialize lands.
pub struct BoxPool {
    slots: Vec<Option<BoxedValue>>,
    rc: Vec<u32>,
    free_list: Vec<u32>,
}

impl BoxPool {
    pub fn new() -> Self { Self { slots: Vec::new(), rc: Vec::new(), free_list: Vec::new() } }

    pub fn intern(&mut self, b: BoxedValue) -> u32 {
        if let Some(idx) = self.free_list.pop() {
            self.slots[idx as usize] = Some(b);
            self.rc[idx as usize] = 1;
            idx
        } else {
            let idx = self.slots.len() as u32;
            self.slots.push(Some(b));
            self.rc.push(1);
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
            self.free_list.push(idx);
        }
    }

    pub fn live_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn value_is_8_bytes() { assert_eq!(std::mem::size_of::<Value>(), 8); }

    #[test]
    fn int_round_trip() {
        for n in [0i64, 1, -1, 42, -42, I48_MAX, I48_MIN] {
            let v = Value::from_int(n);
            assert_eq!(v.as_int(), Some(n));
            assert_eq!(v.as_float(), None);
        }
    }

    #[test]
    fn float_round_trip() {
        for f in [0.0, 1.5, -3.14, f64::INFINITY, -f64::INFINITY] {
            assert_eq!(Value::from_float(f).as_float(), Some(f));
        }
    }

    #[test]
    fn nan_is_float() {
        let v = Value::from_float(f64::NAN);
        assert!(v.as_float().unwrap().is_nan());
        assert_eq!(v.as_int(), None);
    }

    #[test]
    fn bool_round_trip() {
        assert_eq!(Value::from_bool(true).as_bool(), Some(true));
        assert_eq!(Value::from_bool(false).as_bool(), Some(false));
    }

    #[test]
    fn char_round_trip() {
        for c in ['a', '中', '🦀'] {
            assert_eq!(Value::from_char(c).as_char(), Some(c));
        }
    }

    #[test]
    fn handle_round_trip() {
        let v = Value::from_handle(0xABCDEF, 0x123456);
        assert_eq!(v.as_handle(), Some((0xABCDEF, 0x123456)));
    }

    #[test]
    fn none_and_unit_distinct() {
        assert!(Value::UNIT.is_unit() && !Value::UNIT.is_none());
        assert!(Value::NONE.is_none() && !Value::NONE.is_unit());
    }

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
        let idx = pool.intern(BoxedValue::String("a".into()));   // rc=1
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
        // Force rc to u32::MAX then try to inc — must Err, not silently saturate.
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
    fn from_int_or_box_inline_path_for_small_values() {
        let mut pool = BoxPool::new();
        let v = Value::from_int_or_box(&mut pool, 42);
        assert_eq!(v.as_int(), Some(42));
        assert_eq!(v.as_int_full(&pool), Some(42));
        assert_eq!(pool.live_count(), 0);
    }

    #[test]
    fn from_int_or_box_boxes_i64_min() {
        let mut pool = BoxPool::new();
        let v = Value::from_int_or_box(&mut pool, i64::MIN);
        assert_eq!(v.as_int(), None);              // not inline
        assert_eq!(v.as_int_full(&pool), Some(i64::MIN));
        assert_eq!(pool.live_count(), 1);
    }

    #[test]
    fn from_int_or_box_boxes_just_outside_i48() {
        let mut pool = BoxPool::new();
        let n = I48_MAX + 1;
        let v = Value::from_int_or_box(&mut pool, n);
        assert_eq!(v.as_int(), None);
        assert_eq!(v.as_int_full(&pool), Some(n));
    }
}
