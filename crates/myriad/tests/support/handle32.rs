//! Bounded 32-bit handle bindings, independent of the existing 64-bit heap.
//!
//! Handles belong to one table lifetime. The caller owns physical allocations,
//! must provide an unaliased live offset to `allocate`, and must discard all
//! handles before destroying the table. Releasing a binding does not free memory.

use core::mem::size_of;
use std::vec::Vec;

const GEN_MASK: u32 = u16::MAX as u32;
const ACTIVE: u32 = 1 << 16;
const RETIRED: u32 = 1 << 17;
const MAX_SLOTS: usize = u16::MAX as usize - 1;

#[derive(Clone, Copy)]
struct Entry {
    // Active: physical offset. Free: next free slot. Retired: unused.
    offset_or_next: u32,
    state: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle32Error {
    InvalidCapacity,
    BudgetExceeded,
    AllocationFailed,
    Exhausted,
    InvalidHandle,
}

pub struct Handle32Table {
    entries: Vec<Entry>,
    free_head: u32,
}

impl Handle32Table {
    /// Reserve `max_slots` usable bindings plus reserved slot zero.
    ///
    /// `table_budget_bytes` bounds table-entry storage, including slot zero;
    /// it excludes this struct, allocator bookkeeping, and the physical heap.
    /// The table never grows and never resets generations. Slot 65535 is reserved
    /// so the all-ones word cannot be issued as a handle.
    pub fn new(max_slots: usize, table_budget_bytes: usize) -> Result<Self, Handle32Error> {
        if !(1..=MAX_SLOTS).contains(&max_slots) {
            return Err(Handle32Error::InvalidCapacity);
        }
        let count = max_slots
            .checked_add(1)
            .ok_or(Handle32Error::InvalidCapacity)?;
        let bytes = count
            .checked_mul(size_of::<Entry>())
            .ok_or(Handle32Error::InvalidCapacity)?;
        if bytes > table_budget_bytes {
            return Err(Handle32Error::BudgetExceeded);
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(count)
            .map_err(|_| Handle32Error::AllocationFailed)?;
        let reserved_bytes = entries
            .capacity()
            .checked_mul(size_of::<Entry>())
            .ok_or(Handle32Error::BudgetExceeded)?;
        if reserved_bytes > table_budget_bytes {
            return Err(Handle32Error::BudgetExceeded);
        }
        entries.push(Entry {
            offset_or_next: 0,
            state: RETIRED,
        });
        for slot in 1..=max_slots {
            entries.push(Entry {
                offset_or_next: if slot == max_slots {
                    0
                } else {
                    (slot + 1) as u32
                },
                state: 0,
            });
        }
        Ok(Self {
            entries,
            free_head: 1,
        })
    }

    pub fn storage_bytes(&self) -> usize {
        self.entries.capacity() * size_of::<Entry>()
    }

    pub fn allocate(&mut self, offset: u32) -> Result<u32, Handle32Error> {
        if self.free_head == 0 {
            return Err(Handle32Error::Exhausted);
        }
        let slot = self.free_head;
        let entry = &mut self.entries[slot as usize];
        self.free_head = entry.offset_or_next;
        entry.offset_or_next = offset;
        entry.state |= ACTIVE;
        Ok((slot << 16) | (entry.state & GEN_MASK))
    }

    pub fn resolve(&self, handle: u32) -> Result<u32, Handle32Error> {
        let slot = (handle >> 16) as usize;
        if slot == 0 || slot > MAX_SLOTS {
            return Err(Handle32Error::InvalidHandle);
        }
        let entry = self.entries.get(slot).ok_or(Handle32Error::InvalidHandle)?;
        if entry.state & ACTIVE == 0 || entry.state & GEN_MASK != handle & GEN_MASK {
            return Err(Handle32Error::InvalidHandle);
        }
        Ok(entry.offset_or_next)
    }

    /// A slot released at generation 65535 is permanently retired. Its physical
    /// offset can still be reused through another slot; stale handles never revive.
    pub fn release(&mut self, handle: u32) -> Result<u32, Handle32Error> {
        let offset = self.resolve(handle)?;
        let slot = handle >> 16;
        let entry = &mut self.entries[slot as usize];
        let generation = entry.state & GEN_MASK;
        if generation == GEN_MASK {
            entry.state = RETIRED | generation;
            entry.offset_or_next = 0;
        } else {
            entry.state = generation + 1;
            entry.offset_or_next = self.free_head;
            self.free_head = slot;
        }
        Ok(offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_is_exactly_two_u32_words() {
        assert_eq!(size_of::<Entry>(), 2 * size_of::<u32>());
    }
}
