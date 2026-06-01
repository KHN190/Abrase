use crate::ast::Span;
use std::collections::{HashMap, HashSet};

pub type Var = u32;
pub type Region = u32;
pub type Closure = u32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind { Move, Copy, Shared, Ref, RefMut }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CapMode { Move, Copy }

#[derive(Clone, Debug)]
pub struct SafetyError {
    pub code: &'static str,
    pub msg: String,
    pub span: Span,
}

#[derive(Default)]
pub struct Facts {
    pub binding: HashMap<Var, Kind>,
    pub is_handle: HashSet<Var>,
    pub in_region: HashMap<Var, Region>,
    pub flows_outside: HashMap<Var, Region>,
    pub capture: Vec<(Closure, Var, CapMode)>,
    pub closure_var: HashMap<Closure, Var>,
    pub consumed: Vec<(Var, Span)>,
    pub span: HashMap<Var, Span>,
}

impl Facts {
    pub fn new() -> Self { Self::default() }

    fn span_of(&self, v: Var) -> Span {
        self.span.get(&v).copied().unwrap_or(Span::new(0, 0))
    }

    pub fn analyze(&self) -> (Vec<SafetyError>, HashSet<(Var, Region)>) {
        let mut errors = Vec::new();

        let mut escapes: HashSet<(Var, Region)> = HashSet::new();
        for (&v, &r) in &self.flows_outside {
            escapes.insert((v, r));
        }

        loop {
            let before = escapes.len();
            let snapshot: Vec<(Var, Region)> = escapes.iter().copied().collect();
            for (cv, r) in snapshot {
                if let Some(&closure) = self.closure_var.iter()
                    .find_map(|(c, &v)| if v == cv { Some(c) } else { None })
                {
                    for (cc, captured, mode) in &self.capture {
                        if *cc == closure && *mode == CapMode::Move {
                            escapes.insert((*captured, r));
                        }
                    }
                }
            }
            if escapes.len() == before { break; }
        }

        for (cl, v, _mode) in &self.capture {
            if matches!(self.binding.get(v), Some(Kind::Ref) | Some(Kind::RefMut)) {
                let _ = cl;
                errors.push(SafetyError {
                    code: "capture-ref",
                    msg: format!("closure cannot capture reference binding"),
                    span: self.span_of(*v),
                });
            }
        }

        let mut must_forget: HashSet<(Var, Region)> = HashSet::new();
        for &(v, r) in &escapes {
            match self.binding.get(&v) {
                Some(Kind::Ref) | Some(Kind::RefMut) => errors.push(SafetyError {
                    code: "escape-ref",
                    msg: format!("reference cannot escape its region"),
                    span: self.span_of(v),
                }),
                Some(Kind::Shared) => errors.push(SafetyError {
                    code: "escape-shared",
                    msg: format!("Shared value cannot escape its region"),
                    span: self.span_of(v),
                }),
                _ if self.is_handle.contains(&v) => { must_forget.insert((v, r)); }
                _ => {}
            }
        }

        let mut consume_count: HashMap<Var, (usize, Span)> = HashMap::new();
        for &(v, sp) in &self.consumed {
            let e = consume_count.entry(v).or_insert((0, sp));
            e.0 += 1;
            e.1 = sp;
        }
        for (&v, &(n, sp)) in &consume_count {
            if n >= 2 && self.is_handle.contains(&v)
                && matches!(self.binding.get(&v), Some(Kind::Move))
            {
                errors.push(SafetyError {
                    code: "double-move",
                    msg: format!("move handle consumed more than once"),
                    span: sp,
                });
            }
        }

        (errors, must_forget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Facts { Facts::new() }

    #[test]
    fn shared_escaping_region_is_error() {
        let mut f = facts();
        f.binding.insert(0, Kind::Shared);
        f.in_region.insert(0, 1);
        f.flows_outside.insert(0, 1);
        let (errs, _) = f.analyze();
        assert!(errs.iter().any(|e| e.code == "escape-shared"));
    }

    #[test]
    fn ref_escaping_region_is_error() {
        let mut f = facts();
        f.binding.insert(0, Kind::Ref);
        f.flows_outside.insert(0, 1);
        let (errs, _) = f.analyze();
        assert!(errs.iter().any(|e| e.code == "escape-ref"));
    }

    #[test]
    fn closure_capturing_ref_is_error() {
        let mut f = facts();
        f.binding.insert(0, Kind::Ref);
        f.binding.insert(1, Kind::Move);
        f.closure_var.insert(0, 1);
        f.capture.push((0, 0, CapMode::Move));
        let (errs, _) = f.analyze();
        assert!(errs.iter().any(|e| e.code == "capture-ref"));
    }

    #[test]
    fn escaping_closure_carries_moved_capture() {
        let mut f = facts();
        f.binding.insert(0, Kind::Move);
        f.is_handle.insert(0);
        f.in_region.insert(0, 5);
        f.binding.insert(1, Kind::Move);
        f.is_handle.insert(1);
        f.closure_var.insert(0, 1);
        f.capture.push((0, 0, CapMode::Move));
        f.flows_outside.insert(1, 5);
        let (errs, must_forget) = f.analyze();
        assert!(errs.is_empty());
        assert!(must_forget.contains(&(0, 5)));
        assert!(must_forget.contains(&(1, 5)));
    }

    #[test]
    fn move_handle_consumed_twice_is_error() {
        let mut f = facts();
        f.binding.insert(0, Kind::Move);
        f.is_handle.insert(0);
        f.consumed.push((0, Span::new(1, 1)));
        f.consumed.push((0, Span::new(2, 1)));
        let (errs, _) = f.analyze();
        assert!(errs.iter().any(|e| e.code == "double-move"));
    }

    #[test]
    fn copy_value_escaping_is_fine() {
        let mut f = facts();
        f.binding.insert(0, Kind::Copy);
        f.flows_outside.insert(0, 1);
        let (errs, must_forget) = f.analyze();
        assert!(errs.is_empty());
        assert!(must_forget.is_empty());
    }
}
