//! Request-scoped I/O timing. Never retains SQL, object keys, or user content.
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use worker::Date;

#[derive(Clone, Default)]
pub struct Timings(Rc<RefCell<BTreeMap<&'static str, (u64, u64)>>>);
impl Timings {
    pub(crate) fn start(&self, name: &'static str) -> Span {
        Span {
            timings: self.clone(),
            name,
            started: Date::now().as_millis(),
        }
    }
    pub fn server_timing(&self) -> String {
        self.0
            .borrow()
            .iter()
            .map(|(name, (ms, count))| format!("{name};dur={ms};desc=\"{count} calls\""))
            .collect::<Vec<_>>()
            .join(", ")
    }
}
pub(crate) struct Span {
    timings: Timings,
    name: &'static str,
    started: u64,
}
impl Drop for Span {
    fn drop(&mut self) {
        let mut timings = self.timings.0.borrow_mut();
        let (ms, count) = timings.entry(self.name).or_default();
        *ms += Date::now().as_millis().saturating_sub(self.started);
        *count += 1;
    }
}
