use crate::{CRef, Database, Literal};
use std::{
  fs::File,
  io::{self, BufRead, BufReader},
  path::Path,
};
// TODO might need to eventually convert this into a streaming
pub fn from_dimacs<S: AsRef<Path>>(s: S, db: &mut Database, out: &mut Vec<CRef>) -> io::Result<()> {
  let file = File::open(s)?;
  let buf_reader = BufReader::new(file);
  let mut buf = vec![];
  // reported max seen variable
  let mut max_seen_var = 0;
  // file's label for maximum var
  for l in buf_reader.lines() {
    let l = l?;
    let l = l.trim();
    if l.starts_with('c') {
      continue;
    } else if l.starts_with("p cnf") {
      let mut items = l.split_whitespace().filter_map(|v| v.parse::<u32>().ok());
      db.max_var = items.next().expect("Missing # variables from \"p cnf\"");
      let max_clauses = items.next().expect("Missing # clauses from \"p cnf\"");
      out.reserve(max_clauses as usize - out.len());
      continue;
    }
    for v in l.split_whitespace() {
      let v = v.parse::<i32>().expect("Failed to parse literal");
      if v == 0 {
        assert!(!buf.is_empty(), "Empty clause in input");
        // Some inputs have duplications which break the solver.
        // Sorting and then deduping might work best but this is cheaper for now.
        buf.dedup();
        let cref = db.add_clause_from_slice(&buf);
        out.push(cref);
        buf.clear();
        debug_assert!(buf.is_empty());
      } else {
        let l = Literal::from(v);
        debug_assert_eq!(l.var() + 1, v.abs() as u32);
        assert!(
          l.is_valid(),
          "Too many variables to handle CNF file properly"
        );
        max_seen_var = max_seen_var.max(l.var() as u32 + 1);
        buf.push(l);
      }
    }
  }
  assert!(max_seen_var <= db.max_var);
  Ok(())
}

// TODO might need to eventually convert this into a streaming
pub fn from_dimacs_2<S: AsRef<Path>>(s: S, db: &mut Database) -> io::Result<Vec<CRef>> {
  let mut out = vec![];
  from_dimacs(s, db, &mut out)?;
  Ok(out)
}
