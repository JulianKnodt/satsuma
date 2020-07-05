use crate::{CRef, Database, Literal};
use hashbrown::{hash_map::Entry, HashMap};
use rustc_hash::FxHasher;
use std::{hash::BuildHasherDefault, mem::replace};

#[derive(Debug)]
pub struct WatchList {
  // watched literal -> Watched clauses
  occs: Vec<HashMap<CRef, Literal, BuildHasherDefault<FxHasher>>>,
}

impl WatchList {
  pub fn new(vars: u32) -> Self {
    Self {
      occs: vec![HashMap::with_hasher(Default::default()); (vars as usize) << 1],
    }
  }
  pub fn watch(&mut self, cref: CRef, db: &Database) -> Literal {
    let mut lits = cref.iter(db).take(2);
    let l_0 = match lits.next() {
      None => panic!("Empty clause passed to watch"),
      Some(&lit) => lit,
    };
    if let Some(&l_1) = lits.next() {
      self.add_clause_with_lits(cref, l_0, l_1);
      Literal::INVALID
    } else {
      l_0
    }
  }
  pub fn set<CB>(&mut self, l_0: Literal, assns: &[Option<bool>], db: &Database, cb: CB)
  where
    CB: FnMut(CRef, Literal), {
    debug_assert_eq!(l_0.assn(assns), Some(true));
    self.set_false(!l_0, assns, db, cb);
  }
  pub fn set_false<CB>(&mut self, l_0: Literal, assns: &[Option<bool>], db: &Database, mut cb: CB)
  where
    CB: FnMut(CRef, Literal), {
    let temp = HashMap::with_hasher(Default::default());
    let mut set_map = replace(&mut self.occs[l_0.raw() as usize], temp);
    let occs = &mut self.occs;
    let out = set_map.drain_filter(move |&cref, &mut l_1| {
      assert_ne!(l_0, l_1);
      if Some(true) == l_1.assn(assns) {
        // debug_assert_eq!(&self.occs[l_1.raw() as usize][&cref], &l_0);
        return true;
      }
      let mut next = None;
      for &l in cref.iter(db).filter(|&&l| l != l_1) {
        match l.assn(assns) {
          Some(false) => (),
          Some(true) => {
            next.replace(l);
            break;
          },
          None => {
            next.replace(l);
          },
        }
      }
      let next = if let Some(next) = next {
        next
      } else {
        return false;
      };
      // TODO convert these to unchecked
      *occs[l_1.raw() as usize].get_mut(&cref).unwrap() = next;
      occs[next.raw() as usize].insert(cref, l_1);
      true
    });
    for (c, l) in out {
      cb(c, l);
    }
    self.occs[l_0.raw() as usize] = set_map;
  }
  fn add_clause_with_lits(&mut self, c: CRef, l_0: Literal, l_1: Literal) {
    let none_evicted = self.occs[l_0.raw() as usize].insert(c, l_1).is_none();
    debug_assert!(none_evicted);
    let none_evicted = self.occs[l_0.raw() as usize].insert(c, l_1).is_none();
    debug_assert!(none_evicted);
  }
  pub fn add_learnt(&mut self, assns: &[Option<bool>], cref: CRef, db: &Database) -> Literal {
    if cref.len() == 1 {
      return *cref.iter(&db).next().unwrap();
    }
    let mut lits = cref.iter(db);
    let (l_0, is_unassn) = lits
      .find_map(|&l| match l.assn(&assns) {
        None => Some((l, true)),
        Some(false) => Some((l, false)),
        Some(true) => None,
      })
      .unwrap();
    let (unassn, false_lit) = if is_unassn {
      (l_0, *lits.find(|l| l.assn(&assns) == Some(false)).unwrap())
    } else {
      (*lits.find(|l| l.assn(&assns).is_none()).unwrap(), l_0)
    };
    if let Entry::Vacant(v) = self.occs[unassn.raw() as usize].entry(cref) {
      v.insert(false_lit);
      let prev = self.occs[false_lit.raw() as usize].insert(cref, unassn);
      debug_assert!(prev.is_none());
    } else {
      panic!("unexpected testing to see if this is reached");
    }
    unassn
  }
  pub fn remove_satisfied(&mut self, assns: &[Option<bool>]) {
    for (l_0, watches) in self.occs.iter_mut().enumerate() {
      if watches.is_empty() {
        continue;
      }
      let l_0 = Literal::from(l_0 as u32);
      if l_0.assn(assns) == Some(true) {
        watches.clear();
      } else {
        watches.retain(|_, l_1| l_1.assn(assns) != Some(true));
      }
    }
  }
  pub fn drain(&mut self) -> impl Iterator<Item=(Literal, Literal, CRef)> + '_ {
    self.occs.iter_mut().enumerate().flat_map(|(l_0, watches)| {
      watches.drain().map(move |(cref, l_1)| {
        (Literal::from(l_0 as u32), l_1, cref)
      })
    })
  }
}