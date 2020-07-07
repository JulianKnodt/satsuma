extern crate priority_queue;

use crate::{CRef, Database};
use priority_queue::PriorityQueue;
use rustc_hash::FxHasher;
use std::hash::BuildHasherDefault;

#[derive(PartialOrd, Debug, PartialEq, Clone, Copy)]
struct Priority(f32);

impl Eq for Priority {}
impl Ord for Priority {
  fn cmp(&self, o: &Self) -> std::cmp::Ordering { self.partial_cmp(o).unwrap() }
}

#[derive(Debug, PartialEq, Clone)]
pub struct VariableState {
  // TODO need to make these two fields into one which is a lazily deleting Priority queue for
  // integers.
  /// Variable -> activity (- implies that it's currently deactivated);
  priorities: PriorityQueue<u32, Priority, BuildHasherDefault<FxHasher>>,

  /// constant rate of decay for this state
  pub decay_rate: f32,

  /// How much to increment the activity each time a variable is seen
  pub inc_amt: f32,
}

pub const DEFAULT_DECAY_RATE: f32 = 1.2;
pub const DEFAULT_INC_AMT: f32 = 1.0;

impl VariableState {
  pub fn new(vars: u32) -> Self {
    let mut priorities = PriorityQueue::with_capacity_and_default_hasher(vars as usize);
    for i in 0..vars {
      priorities.push(i, Priority(0.0));
    }
    Self {
      priorities,
      decay_rate: DEFAULT_DECAY_RATE,
      inc_amt: DEFAULT_INC_AMT,
    }
  }
  /// decays the current occurrence account
  pub fn decay(&mut self) {
    let decay_rate = self.decay_rate;
    for (_, v) in self.priorities.iter_mut() {
      v.0 /= decay_rate;
    }
  }
  /// Increases the activity for this variable
  pub fn increase_var_activity(&mut self, var: u32) {
    let inc_amt = self.inc_amt;
    self.priorities.change_priority_by(&var, |Priority(curr)| {
      Priority((curr.abs() + inc_amt).copysign(curr))
    });
  }
  /// Adds a clause to this variable state cache
  pub fn update_clause(&mut self, c: &CRef, db: &Database) {
    for lit in c.iter(db) {
      self.increase_var_activity(lit.var() as u32);
    }
  }
  pub fn enable(&mut self, var: u32) {
    self
      .priorities
      .change_priority_by(&var, |Priority(curr)| Priority(curr.abs()));
  }
  /// returns the variable with highest priority
  /// Modifies the internal state so that the variable cannot be picked again
  /// Until it is re-enabled
  pub fn take_highest_prio(&mut self) -> u32 {
    let (&var, &Priority(p)) = self.priorities.peek().unwrap();
    debug_assert!(p.is_sign_positive(), "{:?}", self.priorities);
    self
      .priorities
      .change_priority(&var, Priority(-(p + f32::EPSILON)));
    // assert!(self.evicted[next.0 as usize].replace(next.1).is_none());
    var
  }
}
