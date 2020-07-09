use crate::{CRef, Database, Literal, RestartState, Stats, VariableState, WatchList};
use hashbrown::{hash_map::Entry, HashMap};
use rustc_hash::FxHasher;
use std::{hash::BuildHasherDefault, io, mem::replace, path::Path};

pub const RESTART_BASE: u64 = 100;
pub const RESTART_INC: u64 = 2;
pub const LEARNTSIZE_FACTOR: f32 = 1.0 / 3.0;
pub const LEARNTSIZE_INC: f32 = 1.3;

/// Reserved value that specifies an invalid value
pub const INVALID_LEVEL: u32 = std::u32::MAX;

#[derive(Debug)]
pub struct Solver {
  /// which vars are assigned to what at the current stage
  // TODO could compress this into some sort of bit array with a mask
  assignments: Vec<Option<bool>>,

  /// stack of assignments, needed for backtracking
  assignment_trail: Vec<Literal>,

  /// Which index in the assignment trail was a variable assigned at
  // despite this being u32 this is an index into an array.
  level_indeces: Vec<u32>,

  /// keeps track of which clause caused a variable to be assigned and what level it was
  /// assigned it. None in the case of unassigned or assumption
  // despite this being a u32, there are INVALID entries, as represented by INVALID_LEVEL.
  levels: Vec<u32>,
  causes: Vec<Option<CRef>>,

  database: Database,

  /// Watch list for this solver, and where list of clauses is kept
  watch_list: WatchList,

  /// last assigned per each variable
  /// initialized to false
  // TODO could compress this into a bit array
  polarities: Vec<bool>,

  /// Var state independent decaying sum
  var_state: VariableState,

  /// which level is this solver currently at
  level: u32,

  /// Restart State using Luby
  restart_state: RestartState,

  // a reusable tracker for what was seen and what was not
  // should be clear before and after each call to analyze
  analyze_seen: HashMap<u32, SeenState, BuildHasherDefault<FxHasher>>,

  /// Statistics for this solver
  pub stats: Stats,

  learnt_buf: Vec<Literal>,
  unit_buf: Vec<(CRef, Literal)>,
  cref_buf: Vec<CRef>,
  seen_stack: Vec<(u32, Literal)>,
}

impl Solver {
  /// Creates a new instance of this solver
  pub fn new() -> Self {
    Self {
      assignments: vec![],
      assignment_trail: vec![],
      level_indeces: vec![],
      levels: vec![],
      causes: vec![],
      watch_list: WatchList::new(),
      database: Database::new(),
      polarities: vec![],
      var_state: VariableState::new(),
      level: 0,
      restart_state: RestartState::new(RESTART_BASE, RESTART_INC),

      analyze_seen: HashMap::with_hasher(Default::default()),
      stats: Stats::new(),
      learnt_buf: vec![],
      unit_buf: vec![],
      cref_buf: vec![],
      seen_stack: vec![],
    }
  }
  /// Attempt to find a satisfying assignment for the current solver.
  /// Returning true if there is a solution found.
  pub fn solve(&mut self) -> bool {
    assert_eq!(self.level, 0);
    let mut max_learnts = (self.database.num_clauses as f32) * LEARNTSIZE_FACTOR;

    while self.has_unassigned_vars() {
      self.next_level();
      let lit = self.choose_lit();
      let mut conflict = self.with(lit, None);

      // loop as long as there is a conflict
      while let Some(clause) = conflict {
        self.restart_state.mark_conflict();

        // Conflict when we can't backtrack any more.
        if self.level == 0 {
          return false;
        }
        self.stats.record_learned_clause();
        let (learnt_clause, backtrack_lvl) = self.analyze(&clause, self.level);
        debug_assert!(backtrack_lvl < self.level);
        self.backtrack_to(backtrack_lvl);
        if learnt_clause.is_empty() {
          return false;
        }

        self
          .stats
          .record_learnt_literals(learnt_clause.len() as u32);

        let lit = self
          .watch_list
          .add_learnt(&self.assignments, learnt_clause, &self.database);

        self.var_state.decay();
        // self.watch_list.clause_decay();

        // assign resulting literal with the learnt clause as the cause
        conflict = self.with(lit, Some(learnt_clause));
        debug_assert_eq!(self.assignments[lit.var() as usize], Some(lit.val()));
      }

      if self.restart_state.restart_suggested() {
        self.stats.record_restart();
        self.restart_state.restart();
        self.backtrack_to(0);
      }

      if self.level == 0 {
        self.watch_list.remove_satisfied(&self.assignments);
      }

      if self.level == 0 && self.stats.clauses_learned > (max_learnts as usize) {
        let crefs = self.watch_list.drain().filter_map(|(l_0, l_1, cref)| {
          debug_assert_ne!(l_0, l_1);
          if l_0 < l_1 {
            Some(cref)
          } else {
            None
          }
        });
        self.cref_buf.extend(crefs);
        let new_crefs = self
          .database
          .compact(&self.assignments, self.cref_buf.drain(..));
        self.unit_buf.clear();
        for (cref, l_0, l_1) in new_crefs {
          if !l_1.is_valid() {
            self.unit_buf.push((cref, l_0));
            continue;
          }
          let unit = self.watch_list.watch_with_lits(cref, l_0, l_1);
          assert!(
            unit.is_none(),
            "INTERNAL ERROR there shouldn't be any unit clauses when compacting"
          );
        }
        if self.with_units_from_buf().is_some() {
          return false;
        }
        max_learnts *= LEARNTSIZE_INC;
      }
    }
    true
  }

  /// gets the final assignments for this solver
  /// panics if any variable is still null.
  pub fn final_assignments(&self) -> &[Option<bool>] {
    assert!(
      !self.has_unassigned_vars(),
      "There is no final assignment while there are unassigned variables"
    );
    &self.assignments
  }
  /// Are still unassigned variables for this solver?
  pub fn has_unassigned_vars(&self) -> bool { self.assignment_trail.len() < self.assignments.len() }
  /// returns the reason for a var's assignment if it exists
  pub fn reason(&self, var: u32) -> Option<&CRef> { self.causes[var as usize].as_ref() }

  /// Analyzes a conflict for a given variable
  fn analyze(&mut self, src_clause: &CRef, decision_level: u32) -> (CRef, u32) {
    let mut learnt = replace(&mut self.learnt_buf, vec![]);
    debug_assert!(learnt.is_empty());

    let mut seen = replace(
      &mut self.analyze_seen,
      HashMap::with_hasher(Default::default()),
    );
    debug_assert!(seen.is_empty());
    let curr_len = self.assignment_trail.len() - 1;
    let var_state = &mut self.var_state;
    let trail = &self.assignment_trail;
    let reasons = &self.causes;
    let levels = &self.levels;
    let db = &self.database;
    let mut learn_until_uip =
      |cref: &CRef, remaining: usize, trail_idx: usize, previous_lit: Option<Literal>| {
        // cref.boost();
        let count: usize = cref
          .iter(db)
          // only find new literals
          .filter(|&&lit| previous_lit.map_or(true, |prev| prev != lit))
          .filter(|&&lit| match levels[lit.var() as usize] {
            INVALID_LEVEL | 0 => false,
            lvl => match seen.entry(lit.var()) {
              Entry::Occupied(_) => false,
              Entry::Vacant(ent) => {
                ent.insert(SeenState::Source);
                var_state.increase_var_activity(lit.var());
                let trail = lvl >= decision_level;
                if !trail {
                  learnt.push(lit)
                }
                trail
              },
            },
          })
          .count();
        let mut idx = trail_idx;
        while !seen.contains_key(&trail[idx].var()) && idx > 0 {
          idx -= 1;
        }
        let lit_on_path = trail[idx];
        // should have previously seen this assignment
        let prev_seen = seen.remove(&lit_on_path.var());
        debug_assert!(prev_seen.is_some());
        let conflict = reasons[lit_on_path.var() as usize];
        let next_remaining: usize = (remaining + count).saturating_sub(1);
        (conflict, next_remaining, idx.saturating_sub(1), lit_on_path)
      };
    let mut causes = learn_until_uip(src_clause, 0, curr_len, None);
    while causes.1 > 0 {
      let conflict = causes.0.expect("No cause found in analyze?");
      causes = learn_until_uip(&conflict, causes.1, causes.2, Some(causes.3));
    }
    let mut seen_stack = replace(&mut self.seen_stack, vec![]);
    // minimization before adding asserting literal
    learnt.retain(|lit| {
      self.reason(lit.var()).is_none() || !self.lit_redundant(*lit, &mut seen, &mut seen_stack)
    });
    self.seen_stack = seen_stack;

    // add asserting literal
    learnt.push(!causes.3);
    seen.clear();
    self.analyze_seen = seen;

    if learnt.len() == 1 {
      // backtrack to 0
      self.learnt_buf = learnt;
      let cref = self.database.add_clause_from_slice(&self.learnt_buf);
      self.learnt_buf.clear();
      return (cref, 0);
    }
    // get the first two items explicitly
    let mut levels = learnt.iter().map(|lit| levels[lit.var() as usize]);
    let curr_max = levels.next().unwrap();
    let mut others = levels.filter(|&lvl| lvl != curr_max);
    let (max, second) = match others.next() {
      None => {
        self.learnt_buf = learnt;
        let cref = self.database.add_clause_from_slice(&self.learnt_buf);
        self.learnt_buf.clear();
        return (cref, curr_max);
      },
      Some(lvl) if lvl > curr_max => (lvl, curr_max),
      Some(lvl) => (curr_max, lvl),
    };
    debug_assert_ne!(max, second);
    // and then get the rest of the items
    use std::cmp::Ordering;
    let (_, second) = others.fold((max, second), |(max, second), next| match next.cmp(&max) {
      Ordering::Greater => (next, max),
      Ordering::Equal => (max, second),
      Ordering::Less => (max, second.max(next)),
    });
    self.learnt_buf = learnt;
    let cref = self.database.add_clause_from_slice(&self.learnt_buf);
    self.learnt_buf.clear();
    (cref, second)
  }

  /// Increases the level on this solver and returns the level assigned.
  pub fn next_level(&mut self) -> u32 {
    self.level_indeces.push(self.assignment_trail.len() as u32);
    self.level += 1;
    self.level
  }
  /// revert to given level, retaining all state at that level.
  fn backtrack_to(&mut self, lvl: u32) {
    debug_assert!(lvl >= self.level);
    /*
    if lvl >= self.level {
      // TODO I'm not sure if this ever actually occurs
      return;
    }
    */
    self.level = lvl;
    let index = self.level_indeces[lvl as usize] as usize;
    self.level_indeces.truncate(lvl as usize);
    for lit in self.assignment_trail.drain(index..) {
      let var = lit.var();
      let prev_assn = self.assignments[var as usize].take();
      debug_assert_ne!(prev_assn, None);

      debug_assert_ne!(self.levels[var as usize], INVALID_LEVEL);
      self.levels[var as usize] = INVALID_LEVEL;

      self.polarities[var as usize] = lit.val();
      self.causes[var as usize] = None;
      self.var_state.enable(var);
    }
    debug_assert_eq!(self.level_indeces.len(), lvl as usize);
  }
  /// Records a literal written at the current level, with a possible cause
  fn with(&mut self, lit: Literal, cause: Option<CRef>) -> Option<CRef> {
    let units = &mut self.unit_buf;
    units.clear();
    match cause {
      // In the case there was no previous cause, we need to do one iteration
      None => {
        debug_assert!(lit.assn(&self.assignments).is_none());
        self.assignment_trail.push(lit);
        debug_assert_eq!(self.levels[lit.var() as usize], INVALID_LEVEL);
        self.levels[lit.var() as usize] = self.level;

        debug_assert_eq!(self.assignments[lit.var() as usize], None);
        self.assignments[lit.var() as usize] = Some(lit.val());
        self
          .watch_list
          .set(lit, &self.assignments, &self.database, |c, l| {
            units.push((c, l))
          });
      },
      Some(cause) => units.push((cause, lit)),
    };
    self.with_units_from_buf()
  }

  /// reads from unit_buf and attempts to propogate from there
  fn with_units_from_buf(&mut self) -> Option<CRef> {
    let units = &mut self.unit_buf;
    while let Some((cause, lit)) = units.pop() {
      match lit.assn(&self.assignments) {
        Some(true) => continue,
        None => (),
        Some(false) => return Some(cause),
      }
      self.assignment_trail.push(lit);
      self.stats.record_propogation();
      let var = lit.var() as usize;
      let prev_cause = self.causes[var].replace(cause);
      debug_assert_eq!(prev_cause, None);
      debug_assert_eq!(self.levels[var], INVALID_LEVEL);
      self.levels[var] = self.level;
      let prev_assn = self.assignments[var].replace(lit.val());
      debug_assert_eq!(prev_assn, None);
      self
        .watch_list
        .set(lit, &self.assignments, &self.database, |c, l| {
          units.push((c, l))
        });
    }
    None
  }
  /// Chooese the next decision literal.
  fn choose_lit(&mut self) -> Literal {
    debug_assert!(self.has_unassigned_vars());
    loop {
      let var = self.var_state.take_highest_prio();
      if self.assignments[var as usize].is_none() {
        return Literal::new(var as u32, !self.polarities[var as usize]);
      }
    }
  }

  fn lit_redundant(
    &self,
    lit: Literal,
    seen: &mut HashMap<u32, SeenState, BuildHasherDefault<FxHasher>>,
    seen_stack: &mut Vec<(u32, Literal)>,
  ) -> bool {
    debug_assert!(seen
      .get(&lit.var())
      .map_or(true, |&ss| ss == SeenState::Source));
    debug_assert!(seen_stack.is_empty());
    seen_stack.push((0, lit));

    'recur: while let Some((i, lit)) = seen_stack.pop() {
      let cause = self.reason(lit.var()).unwrap();
      let curr = cause.as_slice(&self.database);
      // TODO need to convert this into a slice so it looks cleaner or an iterator
      for i in i..curr.len() as u32 {
        let to_check = unsafe { *curr.get_unchecked(i as usize) };
        let level_0 = self.levels[to_check.var() as usize] == 0;
        let previously_redundant = seen.get(&to_check.var()).map_or(false, |&ss| {
          ss == SeenState::Source || ss == SeenState::Redundant
        });
        if to_check == lit || level_0 || previously_redundant {
          continue;
        }
        let no_previous_reason = self.reason(to_check.var()).is_none();
        let required = seen
          .get(&to_check.var())
          .map_or(false, |&ss| ss == SeenState::Required);
        if no_previous_reason || required {
          seen.entry(lit.var()).or_insert(SeenState::Required);
          for (_, lit) in seen_stack.drain(..) {
            seen.entry(lit.var()).or_insert(SeenState::Required);
          }
          return false;
        }
        seen_stack.push((i, lit));
        seen_stack.push((0, to_check));
        continue 'recur;
      }
      seen.entry(lit.var()).or_insert(SeenState::Redundant);
    }
    debug_assert!(seen_stack.is_empty());
    true
  }

  /// Loads a DIMACs file into this solver
  pub fn load_dimacs<S: AsRef<Path>>(&mut self, s: S) -> io::Result<bool> {
    crate::parser::from_dimacs(s, &mut self.database, &mut self.cref_buf)?;
    self.resize(self.database.max_var);
    let mut cref_buf = replace(&mut self.cref_buf, vec![]);
    for cref in cref_buf.drain(..) {
      let lit = self.watch_list.watch(cref, &self.database);
      if let Some(lit) = lit {
        if let Some(_conflict) = self.with(lit, Some(cref)) {
          // This show there is a conflict
          return Ok(false);
        }
      }
    }
    self.cref_buf = cref_buf;
    Ok(true)
  }

  /// Resizes this solver to be ready to handle max_vars.
  pub fn resize(&mut self, max_vars: u32) {
    self.levels.resize(max_vars as usize, INVALID_LEVEL);

    self.assignments.resize(max_vars as usize, None);
    self.causes.resize(max_vars as usize, None);
    self.polarities.resize(max_vars as usize, false);

    self.watch_list.resize(max_vars);

    self.var_state.resize(max_vars);
  }

  /// Clears all assignments from this solver
  pub fn clear(&mut self) {
    self.assignments.clear();
    self.assignment_trail.clear();
    self.level_indeces.clear();
    self.levels.clear();
    self.causes.clear();
    self.watch_list.clear();
    self.database.clear();
    self.polarities.clear();
    self.var_state.clear();
    self.level = 0;
    self.restart_state = RestartState::new(RESTART_BASE, RESTART_INC);

    self.analyze_seen.clear();
    self.stats = Stats::new();
    self.learnt_buf.clear();
    self.unit_buf.clear();
    self.cref_buf.clear();
    self.seen_stack.clear();
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SeenState {
  Source,
  Redundant,
  Required,
}

impl Default for Solver {
  fn default() -> Self { Self::new() }
}
