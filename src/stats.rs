use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
  /// how many restarts did this solver perform
  pub restarts: u32,
  /// how many clauses did this solver learn
  pub clauses_learned: usize,
  /// how many propogations were there
  pub propogations: u32,
  /// how many clauses did this solver write to the database
  pub written_clauses: u32,
  /// how many clauses did this solver have transferred to it
  pub transferred_clauses: usize,

  /// For all the learned clauses, how many literals were there
  pub learnt_literals: usize,

  /// The start time of this solver
  pub start_time: Instant,
}

#[derive(Debug, Clone, Copy)]
pub enum Record {
  Restart,
  LearnedClause,
  Propogation,
  Written(u32),
  Transferred(usize),
  LearntLiterals(usize),
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
      written_clauses: 0,
      transferred_clauses: 0,
      learnt_literals: 0,
      start_time: Instant::now(),
    }
  }
  pub fn record(&mut self, rec: Record) {
    match rec {
      Record::Restart => self.restarts += 1,
      Record::LearnedClause => self.clauses_learned += 1,
      Record::Propogation => self.propogations += 1,
      Record::Written(n) => self.written_clauses += n,
      Record::Transferred(n) => self.transferred_clauses += n,
      Record::LearntLiterals(n) => self.learnt_literals += n,
    };
  }
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
  pub fn csv<S: AsRef<str>>(&self, name: S, num_cores: usize, sat: bool) {
    println!(
      "{}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
      name.as_ref(),
      self.restarts,
      self.clauses_learned,
      self.propogations,
      self.written_clauses,
      self.transferred_clauses,
      self.learnt_literals,
      self.start_time.elapsed().as_nanos(),
      num_cores,
      if sat { "SAT" } else { "UNSAT" }
    )
  }
}
