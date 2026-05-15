use super::Value;

pub struct Heap {
    cells: Vec<Option<Vec<Value>>>,
    rc: Vec<u32>,
    free_list: Vec<usize>,
}

impl Heap {
    pub fn new() -> Self {
        Self { cells: Vec::new(), rc: Vec::new(), free_list: Vec::new() }
    }

    pub fn alloc(&mut self, size: usize) -> usize {
        let slot = vec![Value::Unit; size];
        if let Some(h) = self.free_list.pop() {
            self.cells[h] = Some(slot);
            self.rc[h] = 1;
            h
        } else {
            self.cells.push(Some(slot));
            self.rc.push(1);
            self.cells.len() - 1
        }
    }

    pub fn free(&mut self, handle: usize) -> Result<(), String> {
        if handle >= self.cells.len() || self.cells[handle].is_none() {
            return Err(format!("free: invalid handle {}", handle));
        }
        self.cells[handle] = None;
        self.rc[handle] = 0;
        self.free_list.push(handle);
        Ok(())
    }

    pub fn ld(&self, handle: usize, offset: usize) -> Result<Value, String> {
        let cell = self.cells.get(handle).and_then(|c| c.as_ref())
            .ok_or_else(|| format!("ld: invalid handle {}", handle))?;
        cell.get(offset).cloned()
            .ok_or_else(|| format!("ld: offset {} out of bounds (size {})", offset, cell.len()))
    }

    pub fn st(&mut self, handle: usize, offset: usize, val: Value) -> Result<(), String> {
        let cell = self.cells.get_mut(handle).and_then(|c| c.as_mut())
            .ok_or_else(|| format!("st: invalid handle {}", handle))?;
        if offset >= cell.len() {
            return Err(format!("st: offset {} out of bounds (size {})", offset, cell.len()));
        }
        cell[offset] = val;
        Ok(())
    }

    pub fn size(&self, handle: usize) -> Result<usize, String> {
        self.cells.get(handle).and_then(|c| c.as_ref().map(|v| v.len()))
            .ok_or_else(|| format!("size: invalid handle {}", handle))
    }

    pub fn rc_inc(&mut self, handle: usize) -> Result<(), String> {
        if handle >= self.rc.len() || self.cells[handle].is_none() {
            return Err(format!("rc_inc: invalid handle {}", handle));
        }
        self.rc[handle] = self.rc[handle].saturating_add(1);
        Ok(())
    }

    pub fn rc_dec(&mut self, handle: usize) -> Result<bool, String> {
        if handle >= self.rc.len() || self.cells[handle].is_none() {
            return Err(format!("rc_dec: invalid handle {}", handle));
        }
        self.rc[handle] = self.rc[handle].saturating_sub(1);
        if self.rc[handle] == 0 {
            self.cells[handle] = None;
            self.free_list.push(handle);
            return Ok(true);
        }
        Ok(false)
    }
}
