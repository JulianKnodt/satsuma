use crate::{Literal, Database, CRef};
use std::{
  fs::File,
  io::{self, BufRead, BufReader},
  path::Path,
};

pub fn from_dimacs<S: AsRef<Path>, C: FnMut(Literal)> (
  s: S,
  mut cb: C,
) -> io::Result<u32> {
  let file = File::open(s).expect("Failed to open file");
  let buf_reader = BufReader::new(file);
  // reported max seen variable
  let mut max_seen_var = 0;
  // file's label for maximum var
  let mut max_var = 0;
  for l in buf_reader.lines() {
    let l = l?;
    let l = l.trim();
    if l.starts_with('c') {
      continue
    } else if l.starts_with("p cnf") {
      let mut items = l.split_whitespace().filter_map(|v| v.parse::<u32>().ok());
      max_var = items.next().expect("Missing # variables from \"p cnf\"");
      // let _max_clauses = items.next().expect("Missing # clauses from \"p cnf\"");
      continue
    }
    for v in l.split_whitespace() {
      let v = v.parse::<i32>().expect("Failed to parse literal");
      if v == 0 {
        cb(Literal::INVALID);
      } else {
        let l = Literal::from(v);
        assert!(l.is_valid(), "Too many variables to handle CNF file properly");
        cb(Literal::from(v));
        max_seen_var = max_seen_var.max(l.var() as u32 + 1);
      }
    }
  }
  assert!(max_seen_var <= max_var);
  Ok(max_seen_var)
}

pub fn from_dimacs_2<S: AsRef<Path>> (
  s: S,
  db: &mut Database,
) -> io::Result<Vec<CRef>> {
  let file = File::open(s)?;
  let buf_reader = BufReader::new(file);
  let mut out = vec![];
  let mut buf = vec![];
  // reported max seen variable
  let mut max_seen_var = 0;
  // file's label for maximum var
  for l in buf_reader.lines() {
    let l = l?;
    let l = l.trim();
    if l.starts_with('c') {
      continue
    } else if l.starts_with("p cnf") {
      let mut items = l.split_whitespace().filter_map(|v| v.parse::<u32>().ok());
      db.max_var = items.next().expect("Missing # variables from \"p cnf\"");
      let max_clauses = items.next().expect("Missing # clauses from \"p cnf\"");
      out.reserve(max_clauses as usize - out.len());
      continue
    }
    for v in l.split_whitespace() {
      let v = v.parse::<i32>().expect("Failed to parse literal");
      if v == 0 {
        out.push(db.add_clause(buf.drain(..)));
      } else {
        let l = Literal::from(v);
        assert!(l.is_valid(), "Too many variables to handle CNF file properly");
        max_seen_var = max_seen_var.max(l.var() as u32 + 1);
        buf.push(l);
      }
    }
  }
  assert!(max_seen_var <= db.max_var);
  Ok(out)
}
