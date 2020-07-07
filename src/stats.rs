use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
  /// how many restarts did this solver perform
  pub restarts: u32,
  /// how many clauses did this solver learn
  pub clauses_learned: usize,
  /// how many propogations were there
  pub propogations: u32,

  /// For all the learned clauses, how many literals were there
  pub learnt_literals: u32,

  /// The start time of this solver
  pub start_time: Instant,
}

impl Default for Stats {
  fn default() -> Self { Self::new() }
}

impl Stats {
  pub fn new() -> Self {
    Self {
      restarts: 0,
      clauses_learned: 0,
      propogations: 0,
      learnt_literals: 0,
      start_time: Instant::now(),
    }
  }
  pub fn record_restart(&mut self) { self.restarts += 1; }
  pub fn record_learned_clause(&mut self) { self.clauses_learned += 1; }
  pub fn record_propogation(&mut self) { self.propogations += 1; }
  pub fn record_learnt_literals(&mut self, n: u32) { self.learnt_literals += n; }
  /// Prints the rate for this solver given some unit time
  pub fn rate(&self, unit_time: Duration) {
    let total_time = self.start_time.elapsed();
    let elapsed_units = total_time.div_duration_f64(unit_time);
    println!("=======================[Problem Statistics]=====================");
    println!("Restarts {}", self.restarts);
    let clause_rate = (self.clauses_learned as f64) / elapsed_units;
    println!(
      "Conflicts {} ({}/{:?})",
      self.clauses_learned, clause_rate as u32, unit_time
    );
    let prop_rate = (self.propogations as f64) / elapsed_units;
    println!(
      "Propogations: {} ({}/{:?})",
      self.propogations, prop_rate as u32, unit_time
    );
    println!("Total time: {:?}", total_time);
  }
  pub fn csv<S: AsRef<str>>(&self, name: S, sat: bool) {
    println!(
      "{}, {}, {}, {}, {}, {}, {}",
      name.as_ref(),
      self.restarts,
      self.clauses_learned,
      self.propogations,
      self.learnt_literals,
      self.start_time.elapsed().as_nanos(),
      if sat { "SAT" } else { "UNSAT" }
    )
  }
}
