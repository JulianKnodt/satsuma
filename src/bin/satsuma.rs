use clap::{App, Arg};
use satsuma::Solver;

fn main() {
  let matches = App::new("satsuma")
    .version("0.1")
    .author("jk")
    .about("DIMACS SAT solver")
    .arg(
      Arg::with_name("input")
        .short("i")
        .long("input")
        .value_name("DIMACs")
        .help("The input DIMACs file")
        .required(true)
        .takes_value(true)
        .multiple(true),
    )
    .get_matches();
  let mut solver = Solver::new();
  for file_name in matches.values_of("input").unwrap() {
    solver.clear();
    let no_conflict = solver.load_dimacs(file_name).expect("Failed to load DIMACs file");
    if !no_conflict {
      println!("{:?} UNSAT", file_name);
      continue
    }
    let has_solution = solver.solve();
    if has_solution {
      println!("{} SAT", file_name);
    } else {
      println!("{} UNSAT", file_name);
    }
  }
}
