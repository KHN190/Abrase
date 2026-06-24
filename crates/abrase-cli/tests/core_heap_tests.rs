use abrase::compiler::Compiler;
use abrase::lexer::Lexer;
use abrase::parser::Parser;
use myriad::{Heap, Value, VirtualMachine};

fn xorshift(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

const CORE: &str = include_str!("fixtures/core_heap.abe");

fn build() -> abrase::bytecode::Module {
    let mut p = Parser::new(Lexer::new(CORE)).with_source(CORE.into());
    let ast = p.parse_program();
    assert!(p.errors.is_empty(), "parse errors: {:?}", p.errors);
    let mut c = Compiler::new().with_source(CORE.into()).with_lib(true);
    c.compile_module(&ast).unwrap_or_else(|e| {
        panic!("compile errors: {:?}", e.iter().map(|x| &x.message).collect::<Vec<_>>())
    })
}

fn vm_init(size: i64) -> (abrase::bytecode::Module, VirtualMachine) {
    let m = build();
    let mut vm = VirtualMachine::new().with_core_arena(size as usize);
    vm.call_export(&m, "core_init", &[Value::from_int(size)]).expect("core_init");
    (m, vm)
}

fn call(vm: &mut VirtualMachine, m: &abrase::bytecode::Module, name: &str, a: &[i64]) -> i64 {
    let args: Vec<Value> = a.iter().map(|x| Value::from_int(*x)).collect();
    vm.call_export(m, name, &args).expect(name).as_int()
}

#[test]
fn core_init_writes_global_header() {
    let (_m, vm) = vm_init(1024);
    let a = vm.core_arena_ref();
    assert_eq!(a.peek(0, 8).unwrap(), 32);
    assert_eq!(a.peek(8, 8).unwrap(), 1024);
    assert_eq!(a.peek(16, 8).unwrap(), 0);
}

#[test]
fn alloc_bumps_frontier_and_inits_block_header() {
    let (m, mut vm) = vm_init(1024);
    let h = call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(h >> 24, 32, "first block at offset 32");
    assert_eq!(h & 16777215, 0, "fresh gen 0");
    let a = vm.core_arena_ref();
    assert_eq!(a.peek(32, 4).unwrap(), 1, "rc = 1");
    assert_eq!(a.peek(40, 4).unwrap(), 2, "size = 2 words");
    assert_eq!(a.peek(0, 8).unwrap(), 32 + 48, "frontier bumped (blk_bytes(2)=24+8+16=48)");
}

#[test]
fn alloc_zero_size_block() {
    let (m, mut vm) = vm_init(1024);
    let h = call(&mut vm, &m, "alloc", &[0]);
    assert_ne!(h, 0, "zero-size alloc returns a valid handle");
    assert_eq!(vm.core_arena_ref().peek((h >> 24) as u64, 4).unwrap(), 1, "rc=1");
    // free must not write its freelist link outside the 24-byte header-only block
    assert_eq!(call(&mut vm, &m, "rc_dec", &[h]), 1, "freed cleanly");
    let h2 = call(&mut vm, &m, "alloc", &[0]);
    assert_eq!(h2 >> 24, h >> 24, "zero-size block reused");
}

#[test]
fn reused_block_data_is_cleared_no_residue() {
    let (m, mut vm) = vm_init(1024);
    let h1 = call(&mut vm, &m, "alloc", &[2]);
    call(&mut vm, &m, "cell_set", &[h1, 0, 0x7777]);
    call(&mut vm, &m, "cell_set", &[h1, 1, 0x8888]);
    call(&mut vm, &m, "rc_dec", &[h1]);
    let h2 = call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(h2 >> 24, h1 >> 24, "same block reused");
    assert_eq!(call(&mut vm, &m, "cell_get", &[h2, 0]), 0, "stale data word 0 cleared");
    assert_eq!(call(&mut vm, &m, "cell_get", &[h2, 1]), 0, "stale data word 1 cleared");
}

#[test]
fn reused_block_mask_is_cleared_no_stale_handle_bits() {
    let (m, mut vm) = vm_init(1024);
    let a = call(&mut vm, &m, "alloc", &[2]);
    let child = call(&mut vm, &m, "alloc", &[1]);
    // set a handle bit in a's mask, then free a (recursively frees child)
    call(&mut vm, &m, "cell_set_child", &[a, 0, child]);
    let aoff = (a >> 24) as u64;
    assert_ne!(vm.core_arena_ref().peek(aoff + 24, 8).unwrap(), 0, "mask bit set before free");
    call(&mut vm, &m, "rc_dec", &[a]);
    // reuse the same block; prep_block must clear the stale mask region
    let b = call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(b >> 24, a >> 24, "same block reused");
    assert_eq!(vm.core_arena_ref().peek((b >> 24) as u64 + 24, 8).unwrap(), 0,
        "stale mask bit cleared on reuse");
    // a scalar store + free must NOT spuriously recurse on a leftover handle bit
    call(&mut vm, &m, "cell_set", &[b, 0, 0x1234]);
    assert_eq!(call(&mut vm, &m, "rc_dec", &[b]), 1, "scalar block frees cleanly, no phantom child recurse");
}

#[test]
fn alloc_returns_null_on_oom() {
    let (m, mut vm) = vm_init(64);
    assert_eq!(call(&mut vm, &m, "alloc", &[100]), 0);
}

#[test]
fn null_sentinel_handle_is_rejected_not_treated_as_block() {
    let (m, mut vm) = vm_init(1024);
    let frontier_before = vm.core_arena_ref().peek(0, 8).unwrap();
    // handle 0 decodes to offset 0 = the global header. chk must reject it
    // (offset < GLOBAL_HDR), never read/write the header as if it were a block.
    assert_eq!(call(&mut vm, &m, "rc_inc", &[0]), -1, "rc_inc(null) rejected");
    assert_eq!(call(&mut vm, &m, "rc_dec", &[0]), -1, "rc_dec(null) rejected");
    assert_eq!(call(&mut vm, &m, "cell_get", &[0, 0]), -1, "cell_get(null) rejected");
    assert_eq!(call(&mut vm, &m, "cell_set", &[0, 0, 7]), -1, "cell_set(null) rejected");
    assert_eq!(call(&mut vm, &m, "cell_set_child", &[0, 0, 0]), -1, "cell_set_child(null) rejected");
    assert_eq!(vm.core_arena_ref().peek(0, 8).unwrap(), frontier_before, "global header untouched");
}

#[test]
fn allocation_exhaustion_returns_null_cleanly() {
    let (m, mut vm) = vm_init(512);
    let arena = 512u64;
    let mut last = 1i64;
    let mut count = 0;
    while last != 0 && count < 100 {
        last = call(&mut vm, &m, "alloc", &[2]);
        count += 1;
        // frontier must never exceed the arena, even at the exhaustion boundary
        assert!(vm.core_arena_ref().peek(0, 8).unwrap() <= arena, "frontier within arena");
    }
    assert_eq!(last, 0, "allocator returns null once the arena is full");
}

#[test]
fn rc_inc_dec_balance_then_free() {
    let (m, mut vm) = vm_init(1024);
    let h = call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(call(&mut vm, &m, "rc_inc", &[h]), 0, "rc_inc ok");
    assert_eq!(vm.core_arena_ref().peek(32, 4).unwrap(), 2, "rc=2 after inc");
    assert_eq!(call(&mut vm, &m, "rc_dec", &[h]), 0, "rc 2->1, not freed");
    assert_eq!(call(&mut vm, &m, "rc_dec", &[h]), 1, "rc 1->0, freed");
    assert_eq!(vm.core_arena_ref().peek(16, 8).unwrap(), 32, "freed block on freelist");
}

#[test]
fn free_bumps_gen_and_realloc_reuses_with_new_gen() {
    let (m, mut vm) = vm_init(1024);
    let h1 = call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(call(&mut vm, &m, "rc_dec", &[h1]), 1, "freed");
    let h2 = call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(h2 >> 24, 32, "reused same offset");
    assert_eq!(h2 & 16777215, 1, "gen bumped on reuse");
    assert_ne!(h1, h2, "stale handle h1 differs from h2");
}

#[test]
fn stale_handle_access_reports_error() {
    let (m, mut vm) = vm_init(1024);
    let h1 = call(&mut vm, &m, "alloc", &[2]);
    call(&mut vm, &m, "rc_dec", &[h1]);
    call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(call(&mut vm, &m, "rc_inc", &[h1]), -1, "stale rc_inc reports -1");
    assert_eq!(call(&mut vm, &m, "rc_dec", &[h1]), -1, "stale rc_dec reports -1");
}

#[test]
fn cell_access_out_of_bounds_rejected() {
    let (m, mut vm) = vm_init(1024);
    let h = call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(call(&mut vm, &m, "cell_get", &[h, 2]), -1, "index == size rejected");
    assert_eq!(call(&mut vm, &m, "cell_get", &[h, 99]), -1, "index > size rejected");
    assert_eq!(call(&mut vm, &m, "cell_get", &[h, -1]), -1, "negative index rejected");
    assert_eq!(call(&mut vm, &m, "cell_set", &[h, 2, 7]), -1, "set index == size rejected");
    assert_eq!(call(&mut vm, &m, "cell_set_child", &[h, 5, 0]), -1, "set_child oob rejected");
}

#[test]
fn cell_access_on_stale_handle_rejected() {
    let (m, mut vm) = vm_init(1024);
    let h1 = call(&mut vm, &m, "alloc", &[2]);
    call(&mut vm, &m, "rc_dec", &[h1]);
    call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(call(&mut vm, &m, "cell_get", &[h1, 0]), -1, "stale cell_get rejected");
    assert_eq!(call(&mut vm, &m, "cell_set", &[h1, 0, 7]), -1, "stale cell_set rejected");
    assert_eq!(call(&mut vm, &m, "cell_set_child", &[h1, 0, 0]), -1, "stale cell_set_child rejected");
}

#[test]
fn alloc_reuses_only_exact_size_class() {
    let (m, mut vm) = vm_init(1 << 16);
    // free a size-1 and a size-8 block. A size-8 request must skip the size-1
    // node (size mismatch) and reuse the size-8 node exactly.
    let small = call(&mut vm, &m, "alloc", &[1]);
    let large = call(&mut vm, &m, "alloc", &[8]);
    call(&mut vm, &m, "rc_dec", &[large]);
    call(&mut vm, &m, "rc_dec", &[small]);
    let reused = call(&mut vm, &m, "alloc", &[8]);
    assert_eq!(reused >> 24, large >> 24, "size-8 request reused the size-8 block, not the size-1");
}

#[test]
fn mismatched_size_does_not_reuse_smaller_block() {
    let (m, mut vm) = vm_init(1 << 16);
    // free a size-1 block, then request size-4: exact-size policy must NOT reuse
    // the size-1 block; it bumps a fresh block instead (restrictive, no shrink).
    let small = call(&mut vm, &m, "alloc", &[1]);
    let soff = small >> 24;
    call(&mut vm, &m, "rc_dec", &[small]);
    let bigger = call(&mut vm, &m, "alloc", &[4]);
    assert_ne!(bigger >> 24, soff, "size-4 must not reuse the freed size-1 block");
}

#[test]
fn large_cell_cross_word_mask_recursive_free() {
    let (m, mut vm) = vm_init(1 << 16);
    // parent with 70 words forces a 2-word mask; put a child handle past word 64
    let parent = call(&mut vm, &m, "alloc", &[70]);
    let child = call(&mut vm, &m, "alloc", &[1]);
    let coff = (child >> 24) as u64;
    assert_eq!(call(&mut vm, &m, "cell_set_child", &[parent, 65, child]), 0, "set child in 2nd mask word");
    assert_eq!(vm.core_arena_ref().peek(coff, 4).unwrap(), 1, "child rc=1 before");
    assert_eq!(call(&mut vm, &m, "rc_dec", &[parent]), 1, "parent freed");
    assert_eq!(vm.core_arena_ref().peek(coff, 4).unwrap(), 0,
        "cross-word masked child recursively freed");
}

#[test]
fn cell_get_set_roundtrip() {
    let (m, mut vm) = vm_init(1024);
    let h = call(&mut vm, &m, "alloc", &[3]);
    assert_eq!(call(&mut vm, &m, "cell_set", &[h, 0, 111]), 0);
    assert_eq!(call(&mut vm, &m, "cell_set", &[h, 2, 222]), 0);
    assert_eq!(call(&mut vm, &m, "cell_get", &[h, 0]), 111);
    assert_eq!(call(&mut vm, &m, "cell_get", &[h, 2]), 222);
}

#[test]
fn rc_dec_recursively_frees_masked_children() {
    let (m, mut vm) = vm_init(1024);
    let parent = call(&mut vm, &m, "alloc", &[1]);
    let child = call(&mut vm, &m, "alloc", &[1]);
    let coff = (child >> 24) as u64;
    assert_eq!(call(&mut vm, &m, "cell_set_child", &[parent, 0, child]), 0);
    assert_eq!(vm.core_arena_ref().peek(coff, 4).unwrap(), 1, "child rc=1 before");
    assert_eq!(call(&mut vm, &m, "rc_dec", &[parent]), 1, "parent freed");
    assert_eq!(vm.core_arena_ref().peek(coff, 4).unwrap(), 0, "child rc dropped to 0 by recursive free");
    // both parent and child blocks now on freelist
    let poff = (parent >> 24) as u64;
    let head = vm.core_arena_ref().peek(16, 8).unwrap();
    assert!(head == coff || head == poff, "freelist holds freed blocks");
}

#[test]
fn alloc_negative_size_rejected() {
    let (m, mut vm) = vm_init(1024);
    assert_eq!(call(&mut vm, &m, "alloc", &[-1]), 0, "negative size returns null");
    assert_eq!(call(&mut vm, &m, "alloc", &[-1000]), 0, "large negative size returns null");
    // arena unchanged — no bogus block carved
    assert_eq!(vm.core_arena_ref().peek(0, 8).unwrap(), 32, "frontier untouched by rejected alloc");
}

#[test]
fn cell_models_struct_fields_by_word_offset() {
    let (m, mut vm) = vm_init(1024);
    // struct { x: Int, y: Int, child } -> 3-word cell; members are word offsets 0,1,2
    let s = call(&mut vm, &m, "alloc", &[3]);
    let child = call(&mut vm, &m, "alloc", &[1]);
    call(&mut vm, &m, "cell_set", &[s, 0, 42]);
    call(&mut vm, &m, "cell_set", &[s, 1, 99]);
    call(&mut vm, &m, "cell_set_child", &[s, 2, child]);
    assert_eq!(call(&mut vm, &m, "cell_get", &[s, 0]), 42, "field x at word 0");
    assert_eq!(call(&mut vm, &m, "cell_get", &[s, 1]), 99, "field y at word 1");
    let coff = (child >> 24) as u64;
    assert_eq!(call(&mut vm, &m, "rc_dec", &[s]), 1, "struct freed");
    assert_eq!(vm.core_arena_ref().peek(coff, 4).unwrap(), 0, "handle field recursively freed");
}

#[test]
fn rc_cannot_collect_cycles_documents_leak() {
    let (m, mut vm) = vm_init(1024);
    let a = call(&mut vm, &m, "alloc", &[1]);
    let b = call(&mut vm, &m, "alloc", &[1]);
    let aoff = (a >> 24) as u64;
    let boff = (b >> 24) as u64;
    // cycle a.0 -> b, b.0 -> a; each store takes an owning (rc_inc'd) reference
    call(&mut vm, &m, "rc_inc", &[b]);
    call(&mut vm, &m, "cell_set_child", &[a, 0, b]);
    call(&mut vm, &m, "rc_inc", &[a]);
    call(&mut vm, &m, "cell_set_child", &[b, 0, a]);
    // drop both external references
    assert_eq!(call(&mut vm, &m, "rc_dec", &[a]), 0, "a still held by b's edge");
    assert_eq!(call(&mut vm, &m, "rc_dec", &[b]), 0, "b still held by a's edge");
    // RC cannot reclaim the cycle — both leak with rc==1 (identical to the Rust RC heap)
    assert_eq!(vm.core_arena_ref().peek(aoff, 4).unwrap(), 1, "a leaked (rc=1)");
    assert_eq!(vm.core_arena_ref().peek(boff, 4).unwrap(), 1, "b leaked (rc=1)");
}

#[test]
fn alloc_extreme_size_rejected_no_overflow() {
    let (m, mut vm) = vm_init(1 << 16);
    // size near i64::MAX would overflow size*8 in blk_bytes; must be rejected
    assert_eq!(call(&mut vm, &m, "alloc", &[i64::MAX]), 0, "i64::MAX size rejected");
    assert_eq!(call(&mut vm, &m, "alloc", &[1 << 40]), 0, "huge size rejected");
    // arena still intact, normal alloc still works
    assert_ne!(call(&mut vm, &m, "alloc", &[2]), 0, "normal alloc unaffected");
    assert_eq!(vm.core_arena_ref().peek(0, 8).unwrap(), 32 + 48, "frontier only moved by the valid alloc");
}

#[test]
fn double_free_is_rejected_via_generation() {
    let (m, mut vm) = vm_init(1024);
    let h = call(&mut vm, &m, "alloc", &[2]);
    assert_eq!(call(&mut vm, &m, "rc_dec", &[h]), 1, "first free");
    // second rc_dec on the same (now freed, gen-bumped) handle must be caught
    assert_eq!(call(&mut vm, &m, "rc_dec", &[h]), -1, "double free rejected as stale");
}

#[test]
fn generation_survives_past_byte_boundary() {
    // regression: chk once compared full u32 header gen to an 8-bit handle gen,
    // breaking after gen 255. With a 24-bit gen the same offset must stay usable
    // across hundreds of alloc/free cycles, each new handle valid, old one stale.
    let (m, mut vm) = vm_init(1024);
    let mut prev = 0i64;
    for n in 0..300 {
        let h = call(&mut vm, &m, "alloc", &[1]);
        assert_eq!(h >> 24, 32, "iteration {n}: same offset reused");
        assert_eq!(call(&mut vm, &m, "rc_inc", &[h]), 0, "iteration {n}: fresh handle valid (gen {})", h & 16777215);
        assert_eq!(call(&mut vm, &m, "rc_dec", &[h]), 0, "iteration {n}: rc back to 1");
        if n > 0 {
            assert_eq!(call(&mut vm, &m, "rc_inc", &[prev]), -1, "iteration {n}: prior handle is stale");
        }
        call(&mut vm, &m, "rc_dec", &[h]);
        prev = h;
    }
}

struct Mh {
    core: i64,
    slot: u32,
    g: u32,
    rc: i64,
    alive: bool,
}

#[test]
fn differential_rc_against_rust_heap() {
    let (m, mut vm) = vm_init(1 << 20);
    let mut heap = Heap::new();
    let mut hs: Vec<Mh> = Vec::new();
    let mut rng: u64 = 0x1234_5678_9abc_def1;

    let core_rc = |vm: &VirtualMachine, h: i64| -> i64 {
        vm.core_arena_ref().peek((h >> 24) as u64, 4).unwrap() as i64
    };

    for _ in 0..3000 {
        let live: Vec<usize> = (0..hs.len()).filter(|&i| hs[i].alive).collect();
        let pick = if live.is_empty() { 0 } else { xorshift(&mut rng) % 3 };
        if pick == 0 {
            let size = (xorshift(&mut rng) % 4 + 1) as i64;
            let core = call(&mut vm, &m, "alloc", &[size]);
            assert_ne!(core, 0, "arena OOM — raise arena size");
            let (slot, g) = heap.alloc(size as usize);
            assert_eq!(core_rc(&vm, core), 1);
            assert_eq!(heap.rc(slot, g), Some(1));
            hs.push(Mh { core, slot, g, rc: 1, alive: true });
        } else {
            let idx = live[(xorshift(&mut rng) as usize) % live.len()];
            if pick == 1 {
                assert_eq!(call(&mut vm, &m, "rc_inc", &[hs[idx].core]), 0);
                heap.rc_inc(hs[idx].slot, hs[idx].g).unwrap();
                hs[idx].rc += 1;
            } else {
                let cfreed = call(&mut vm, &m, "rc_dec", &[hs[idx].core]);
                let rfreed = heap.rc_dec(hs[idx].slot, hs[idx].g).unwrap();
                hs[idx].rc -= 1;
                let freed = hs[idx].rc == 0;
                assert_eq!(cfreed == 1, freed, "core freed-flag mismatch");
                assert_eq!(rfreed, freed, "rust freed-flag mismatch");
                if freed {
                    hs[idx].alive = false;
                }
            }
            if hs[idx].alive {
                assert_eq!(core_rc(&vm, hs[idx].core), hs[idx].rc, "core rc diverged");
                assert_eq!(heap.rc(hs[idx].slot, hs[idx].g).map(|x| x as i64), Some(hs[idx].rc),
                    "rust rc diverged");
            }
        }
    }

    for h in hs.iter_mut().filter(|h| h.alive) {
        while h.rc > 0 {
            call(&mut vm, &m, "rc_dec", &[h.core]);
            heap.rc_dec(h.slot, h.g).unwrap();
            h.rc -= 1;
        }
        h.alive = false;
    }
    assert_eq!(heap.live_count(), 0, "rust heap fully reclaimed after balanced drain");
}
