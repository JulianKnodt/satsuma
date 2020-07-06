use clap::{App, Arg};
use satsuma::solver_from_dimacs;

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
        .takes_value(true),
    )
    .get_matches();
  let file_name = matches.value_of("input").unwrap();
  let mut solver = solver_from_dimacs(file_name).expect("Failed to create solver");
  let has_solution = solver.solve();
  if has_solution {
    println!("SAT");
  } else {
    println!("UNSAT");
  }
}
