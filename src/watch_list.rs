use crate::{
  database::{ClauseDatabase, ClauseRef},
  literal::Literal,
};
use hashbrown::{hash_map::Entry, HashMap};

/// An implementation of occurrence lists based on MiniSat's OccList
#[derive(Debug, Clone)]
pub struct WatchList {
  occurrences: Vec<HashMap<ClauseRef, Literal>>
}

/// leaves enough space for both true and false variables up to max_var.
#[inline]
fn space_for_all_lits(size: usize) -> usize { (size << 1) }

impl WatchList {
  /// returns a new watchlist, as well as any unit clauses
  /// from the initial constraints
  pub fn new(db: &ClauseDatabase) -> (Self, Vec<(ClauseRef, Literal)>) {
    let mut wl = Self {
      occurrences: vec![HashMap::new(); space_for_all_lits(db.max_var)].into_boxed_slice(),
    };
    let units = db
      .iter()
      .filter_map(|cref| wl.watch(&cref).map(|lit| (cref, lit)))
      .collect();
    (wl, units)
  }
  /// Adds some clause from the given database to this list.
  /// It must not have previously been added to the list.
  fn watch(&mut self, cref: &ClauseRef) -> Option<Literal> {
    let mut lits = cref.literals.iter().take(2);
    match lits.next() {
      None => panic!("Empty clause passed to watch"),
      Some(&lit) => match lits.next() {
        None => Some(lit),
        Some(&o_lit) => {
          assert!(self.add_clause_with_lits(cref.clone(), lit, o_lit));
          None
        },
      },
    }
  }
  /// adds a learnt clause, which is assumed to have at least two literals as well as cause
  /// and implication.
  pub(crate) fn add_learnt(&mut self, assns: &[Option<bool>], cref: &ClauseRef) -> Literal {
    if cref.literals.len() == 1 {
      return cref.literals[0];
    }
    self.activities.push(Arc::downgrade(&cref.activity));
    debug_assert!(cref
      .literals
      .iter()
      .all(|lit| lit.assn(assns) != Some(true)));
    debug_assert_eq!(
      1,
      cref
        .literals
        .iter()
        .filter(|lit| lit.assn(assns).is_none())
        .count()
    );
    let false_lit = *cref
      .literals
      .iter()
      .find(|lit| lit.assn(&assns) == Some(false))
      .unwrap();
    let unassn = *cref
      .literals
      .iter()
      .find(|lit| lit.assn(&assns).is_none())
      .unwrap();
    if let Entry::Vacant(v) = self.occurrences[unassn.raw() as usize].entry(cref.clone()) {
      v.insert(false_lit);
      assert!(self.occurrences[false_lit.raw() as usize]
        .insert(cref.clone(), unassn)
        .is_none());
    }
    unassn
  }
  pub fn set<T>(&mut self, lit: Literal, assns: &[Option<bool>], into: &mut T)
  where
    T: Extend<(ClauseRef, Literal)>, {
    // Sanity check that we actually assigned this variable
    debug_assert_eq!(lit.assn(assns), Some(true));
    self.set_false(!lit, assns, into)
  }
  /// Sets a given literal to false in this watch list
  fn set_false<T>(&mut self, lit: Literal, assns: &[Option<bool>], into: &mut T)
  where
    T: Extend<(ClauseRef, Literal)>, {
    use std::mem::swap;
    let mut swap_map = HashMap::new();
    swap(&mut self.occurrences[lit.raw() as usize], &mut swap_map);
    // removing items from the list without draining
    // should help improve efficiency
    swap_map.retain(|cref, &mut o_lit| {
      assert_ne!(lit, o_lit);
      // If the other one is set to true, we shouldn't update the watch list
      if o_lit.assn(assns) == Some(true) {
        debug_assert_eq!(self.occurrences[o_lit.raw() as usize][&cref], lit);
        return true;
      }
      let mut next = None;
      let mut lits = cref.literals.iter().filter(|&&lit| lit != o_lit);
      while let Some(lit) = lits.next() {
        match lit.assn(assns) {
          Some(false) => (),
          None => {
            next.replace(lit);
          },
          Some(true) => {
            next.replace(lit);
            break;
          },
        };
      }
      match next {
        // In the case of none, then it implies this is a unit clause,
        // so return it and the literal that needs to be set in it.
        None => {
          debug_assert_eq!(self.occurrences[o_lit.raw() as usize][&cref], lit);
          into.extend(std::iter::once((cref.clone(), o_lit)));
          true
        },
        Some(&next) => {
          debug_assert_ne!(lit, next);
          debug_assert_ne!(o_lit, next);
          *self.occurrences[o_lit.raw() as usize]
            .get_mut(&cref)
            .unwrap() = next;
          self.occurrences[next.raw() as usize].insert(cref.clone(), o_lit);
          debug_assert_eq!(self.occurrences[next.raw() as usize][&cref], o_lit);
          debug_assert_eq!(self.occurrences[o_lit.raw() as usize][&cref], next);
          debug_assert!(next.assn(assns) != Some(false));
          false
        },
      }
    });
    swap(&mut self.occurrences[lit.raw() as usize], &mut swap_map);
  }
  /// Adds a transferred clause to this watchlist.
  /// If all literals are false
  /// - And none have causes => Pick one at random(Maybe one with lowest priority)
  /// - And some have causes => Pick one with highest level
  /// Else if one literal is true, watch true lit and any false
  /// Else if one literal is unassigned, watch it and any false and return it
  /// Else watch unassigneds.
  pub fn add_transfer(
    &mut self,
    assns: &[Option<bool>],
    causes: &[Option<ClauseRef>],
    levels: &[Option<usize>],
    cref: &ClauseRef,
  ) -> Option<Literal> {
    let literals = &cref.literals;
    assert!(!literals.is_empty());
    if literals.len() == 1 {
      return match literals[0].assn(assns) {
        Some(false) | None => Some(literals[0]),
        Some(true) => None,
      };
    }
    if self.already_exists(cref) {
      return None;
    }
    let mut watchable = literals
      .iter()
      .filter(|lit| lit.assn(&assns) != Some(false));
    match watchable.next() {
      None => {
        // this case can cause unsoundness on some rare occasions
        let to_backtrack = *literals
          .iter()
          .filter(|lit| causes[lit.var()].is_some())
          // max by seems to work better than min by but both work
          .max_by_key(|lit| levels[lit.var()])
          .unwrap_or_else(|| literals.iter().max_by_key(|lit| levels[lit.var()]).unwrap());
        let other_false = *literals
          .iter()
          .filter(|lit| levels[lit.var()].unwrap() < levels[to_backtrack.var()].unwrap())
          .find(|&&lit| lit != to_backtrack)?;
        debug_assert_ne!(to_backtrack, other_false);
        debug_assert!(levels[to_backtrack.var()] > levels[other_false.var()]);
        assert!(self.add_clause_with_lits(cref.clone(), to_backtrack, other_false));
        Some(to_backtrack)
      },
      Some(&lit) => match watchable.next() {
        None => match lit.assn(assns) {
          // Don't track clauses which have a true literal
          Some(true) => None,
          Some(false) => unreachable!(),
          None => {
            if !self.occurrences[lit.raw() as usize].contains_key(&cref) {
              let other = *literals
                .iter()
                .find(|lit| lit.assn(&assns) == Some(false))?;
              self.activities.push(Arc::downgrade(&cref.activity));
              assert!(self.add_clause_with_lits(cref.clone(), lit, other));
            }
            Some(lit)
          },
        },
        Some(&o_lit) => {
          self.activities.push(Arc::downgrade(&cref.activity));
          assert!(self.add_clause_with_lits(cref.clone(), lit, o_lit));
          None
        },
      },
    }
  }
  fn already_exists(&self, cref: &ClauseRef) -> bool {
    cref
      .literals
      .iter()
      .any(|lit| self.occurrences[lit.raw() as usize].contains_key(cref))
  }
  /// Adds a clause with the given literals into the watch list.
  /// Returns true if another clause was evicted, which likely implies an invariant
  /// was broken.
  #[must_use]
  fn add_clause_with_lits(&mut self, cref: ClauseRef, lit: Literal, o_lit: Literal) -> bool {
    self.occurrences[lit.raw() as usize]
      .insert(cref.clone(), o_lit)
      .is_none()
      && self.occurrences[o_lit.raw() as usize]
        .insert(cref, lit)
        .is_none()
  }

  pub fn remove_satisfied(&mut self, assns: &[Option<bool>]) {
    // TODO could I swap the ordering here of which lit is being removed
    self
      .occurrences
      .iter_mut()
      .enumerate()
      .filter(|(_, watches)| !watches.is_empty())
      .for_each(|(lit, watches)| {
        if Literal::from(lit as u32).assn(assns) == Some(true) {
          watches.clear();
        } else {
          watches.retain(|_, other_lit| other_lit.assn(assns) != Some(true));
        }
        watches.shrink_to_fit();
      });
  }
}
