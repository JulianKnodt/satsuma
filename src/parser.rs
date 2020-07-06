use crate::{CRef, Database, Literal};
use std::{
  fs::File,
  io::{self, BufRead, BufReader},
  path::Path,
};

// TODO might need to eventually convert this into a streaming
pub fn from_dimacs_2<S: AsRef<Path>>(s: S, db: &mut Database) -> io::Result<Vec<CRef>> {
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
        let cref = db.add_clause(buf.drain(..));
        out.push(cref);
        assert!(buf.is_empty());
      } else {
        let l = Literal::from(v);
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
  Ok(out)
}
