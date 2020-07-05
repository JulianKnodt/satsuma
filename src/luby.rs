// replicated from Minisat
// Find the finite subsequence that contains index 'x', and the
// size of that subsequence:
pub const fn luby(mut x: u64, y: u64) -> u64 {
  let mut size = 1;
  let mut seq = 0;
  while size < x + 1 {
    seq += 1;
    size = 2 * size + 1;
  }
  while size - 1 != x {
    size = (size - 1) >> 1;
    seq -= 1;
    x %= size;
  }
  y.pow(seq)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartState {
  pub base_restart_interval: u64,
  pub restart_inc_interval: u64,

  /// Number of previous restarts
  num_restarts: u64,

  /// Number of conflicts remaining before restart
  remaining: u64,
}

impl RestartState {
  pub const fn new(base: u64, inc: u64) -> Self {
    Self {
      base_restart_interval: base,
      restart_inc_interval: inc,
      num_restarts: 0,
      remaining: base * luby(inc, 0),
    }
  }
  pub fn mark_conflict(&mut self) { self.remaining = self.remaining.saturating_sub(1); }
  pub const fn restart_suggested(&self) -> bool { self.remaining == 0 }
  pub fn restart(&mut self) {
    self.num_restarts += 1;
    self.remaining =
      luby(self.restart_inc_interval, self.num_restarts) * self.base_restart_interval;
  }
}
