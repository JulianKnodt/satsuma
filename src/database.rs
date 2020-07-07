use crate::Literal;
use std::mem::swap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CRef {
  idx: u32,
  len: u16,
}

impl CRef {
  pub fn iter<'a>(&'a self, db: &'a Database) -> impl Iterator<Item = &Literal> + 'a {
    self.as_slice(db).iter()
  }
  #[inline]
  pub fn as_slice<'a>(&'a self, db: &'a Database) -> &'a [Literal] {
    &db.literals[self.idx as usize..(self.idx + self.len as u32) as usize]
  }
  pub const fn len(&self) -> usize { self.len as usize }
  pub const fn is_empty(&self) -> bool { self.len == 0 }
}

#[derive(Debug, PartialEq)]
pub struct Database {
  /// All clauses in this database
  pub literals: Vec<Literal>,
  swap_space: Vec<Literal>,
  pub max_var: u32,
  pub num_clauses: u32,
}

impl Database {
  pub const fn new() -> Self {
    Database {
      literals: vec![],
      swap_space: vec![],
      max_var: 0,
      num_clauses: 0,
    }
  }
  /*
  #[must_use]
  pub fn add_clause(&mut self, ls: impl Iterator<Item = Literal>) -> CRef {
    self.num_clauses += 1;
    let idx = self.literals.len() as u32;
    self.literals.extend(ls);
    CRef {
      idx,
      len: (self.literals.len() as u32 - idx) as u16,
    }
  }
  */
  #[must_use]
  pub fn add_clause_from_slice(&mut self, ls: &[Literal]) -> CRef {
    self.num_clauses += 1;
    let idx = self.literals.len() as u32;
    self.literals.extend_from_slice(ls);
    CRef {
      idx,
      len: ls.len() as u16,
    }
  }

  /// Removes satisfied clauses at level 0.
  pub fn compact<'a>(
    &'a mut self,
    assns: &'a [Option<bool>],
    clauses: impl Iterator<Item = CRef> + 'a,
  ) -> impl Iterator<Item = (CRef, Literal, Literal)> + 'a {
    swap(&mut self.literals, &mut self.swap_space);
    self.num_clauses = 1;
    self.literals.clear();
    clauses.filter_map(move |c| {
      if !self.swap_space[c.idx as usize].is_valid() {
        return None;
      }
      let lits = &self.swap_space[c.idx as usize..c.idx as usize + c.len as usize];
      if lits.iter().any(|l| l.assn(assns) == Some(true)) {
        return None;
      }
      let idx = self.literals.len() as u32;
      self.literals.extend_from_slice(lits);
      let len = (self.literals.len() as u32 - idx) as u16;
      self.swap_space[c.idx as usize] = Literal::INVALID;
      if len == 0 {
        None
      } else {
        self.num_clauses += 1;
        let cref = CRef { idx, len };
        let mut lits = cref.iter(&self).copied();
        let l_0 = lits.next().unwrap_or(Literal::INVALID);
        let l_1 = lits.next().unwrap_or(Literal::INVALID);
        Some((cref, l_0, l_1))
      }
    })
  }
}
