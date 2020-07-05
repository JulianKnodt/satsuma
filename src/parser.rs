use crate::{database::Database, literal::Literal};
use std::{
  fs::File,
  io::{self, BufRead, BufReader},
  path::Path,
};

pub fn from_dimacs<S: AsRef<Path>>(s: S) -> io::Result<Database> {
  let file = File::open(s).expect("Failed to open file");
  let buf_reader = BufReader::new(file);
  let mut db = Database::new();
  let mut max_seen_var = 0;
  let mut buffer = vec![];
  for l in buf_reader.lines() {
    let l = l?;
    let l = l.trim().to_string();
    if l.starts_with('c') {
      continue;
    } else if l.starts_with("p cnf") {
      let mut items = l.split_whitespace().filter_map(|v| v.parse::<u32>().ok());
      db.max_var = items.next().expect("Missing # variables from \"p cnf\"");
      let _max_clauses = items.next().expect("Missing # clauses from \"p cnf\"");
      continue;
    }
    for v in l.split_whitespace() {
      let v = v.parse::<i32>().expect("Failed to parse literal");
      if v == 0 {
        db.add_clause(buffer.drain(..));
      } else {
        let lit = Literal::from(v);
        max_seen_var = max_seen_var.max(lit.var() as u32 + 1);
        buffer.push(lit);
      }
    }
  }
  assert!(max_seen_var <= db.max_var);
  // TODO could check if they're not equal
  Ok(db)
}
