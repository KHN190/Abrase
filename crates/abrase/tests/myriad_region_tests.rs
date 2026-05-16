// Integration tests for the RegionTable runtime.

use myriad::{BoxPool, BoxedValue, Heap, RegionTable, Value};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_balances() {
        let mut rt = RegionTable::new();
        assert_eq!(rt.depth(), 0);
        rt.push();
        rt.push();
        assert_eq!(rt.depth(), 2);
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        rt.pop_and_release(&mut heap, &mut pool).unwrap();
        assert_eq!(rt.depth(), 1);
        rt.pop_and_release(&mut heap, &mut pool).unwrap();
        assert_eq!(rt.depth(), 0);
    }

    #[test]
    fn pop_without_push_errs() {
        let mut rt = RegionTable::new();
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        assert!(rt.pop_and_release(&mut heap, &mut pool).is_err());
    }

    #[test]
    fn record_outside_region_is_silent() {
        let mut rt = RegionTable::new();
        rt.record_alloc(7, 0);
        assert_eq!(rt.depth(), 0);
    }

    #[test]
    fn region_force_frees_recorded_slots() {
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        let (s1, g1) = heap.alloc(1);
        let (s2, g2) = heap.alloc(1);
        assert_eq!(heap.live_count(), 2);

        let mut rt = RegionTable::new();
        rt.push();
        rt.record_alloc(s1, g1);
        rt.record_alloc(s2, g2);
        rt.pop_and_release(&mut heap, &mut pool).unwrap();

        assert_eq!(heap.live_count(), 0, "region exit must force-free both slots");
    }

    // Gap (1): cell stores a Box-typed Value; force-free must dec the pool entry.
    #[test]
    fn region_force_free_cascades_to_box_pool() {
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        let box_idx = pool.intern(BoxedValue::String("hi".into()));
        assert_eq!(pool.live_count(), 1);

        let (slot, gen_) = heap.alloc(1);
        heap.st(slot, gen_, 0, Value::from_box(box_idx)).unwrap();

        let mut rt = RegionTable::new();
        rt.push();
        rt.record_alloc(slot, gen_);
        rt.pop_and_release(&mut heap, &mut pool).unwrap();

        assert_eq!(heap.live_count(), 0, "cell freed");
        assert_eq!(pool.live_count(), 0, "boxed String inside cell must also be reclaimed");
    }

    // Gap (1) corollary: rc_dec cascade also decs Box children.
    #[test]
    fn rc_dec_cascades_to_box_pool() {
        let mut heap = Heap::new();
        let mut pool = BoxPool::new();
        let box_idx = pool.intern(BoxedValue::String("hi".into()));

        let (slot, gen_) = heap.alloc(1);
        heap.st(slot, gen_, 0, Value::from_box(box_idx)).unwrap();
        assert_eq!(pool.live_count(), 1);

        // ordinary rc=0 drop
        let _ = heap.rc_dec(slot, gen_, &mut pool).unwrap();
        assert_eq!(heap.live_count(), 0);
        assert_eq!(pool.live_count(), 0, "rc=0 cascade must reclaim Box child");
    }
}
