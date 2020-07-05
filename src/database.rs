use crate::literal::Literal;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CRef {
  idx: u32,
  len: u16,
}

impl CRef {
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
  pub max_var: u32,
}

impl Database {
  pub const fn new() -> Self {
    Database {
      literals: vec![],
      max_var: 0,
    }
  }
  pub fn add_clause(&mut self, ls: impl Iterator<Item = Literal>) -> CRef {
    let idx = self.literals.len() as u32;
    self.literals.extend(ls);
    CRef {
      idx,
      len: (self.literals.len() as u32 - idx) as u16,
    }
  }
}
