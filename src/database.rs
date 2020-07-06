use crate::literal::Literal;
use std::mem::swap;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CRef {
  idx: u32,
  len: u16,
}

impl CRef {
  // TODO convert this to return literals instead of references
  pub fn iter<'a>(&'a self, db: &'a Database) -> impl Iterator<Item = &Literal> + 'a {
    db.literals[self.idx as usize..(self.idx + self.len as u32) as usize].iter()
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
}

impl Database {
  pub const fn new() -> Self {
    Database {
      literals: vec![],
      swap_space: vec![],
      max_var: 0,
    }
  }
  #[must_use]
  pub fn add_clause(&mut self, ls: impl Iterator<Item = Literal>) -> CRef {
    let idx = self.literals.len() as u32;
    self.literals.extend(ls);
    CRef {
      idx,
      len: (self.literals.len() as u32 - idx) as u16,
    }
  }
  pub fn compact<'a>(
    &'a mut self,
    assns: &'a [Option<bool>],
    clauses: impl Iterator<Item = CRef> + 'a,
  ) -> impl Iterator<Item = CRef> + 'a {
    swap(&mut self.literals, &mut self.swap_space);
    self.literals.clear();
    clauses.filter_map(move |c| {
      if !self.swap_space[c.idx as usize].is_valid() {
        return None;
      }
      let idx = self.literals.len() as u32;
      self.literals.extend(
        self.swap_space[c.idx as usize..c.idx as usize + c.len as usize]
          .iter()
          .filter(|l| l.assn(assns) != Some(true)),
      );
      let len = (self.literals.len() as u32 - idx) as u16;
      self.swap_space[c.idx as usize] = Literal::INVALID;
      Some(CRef { idx, len })
    })
  }
}
