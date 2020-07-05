use std::{
  fmt::{self, Debug, Display},
  hash::Hash,
  ops::Not,
};

// TODO define a type for representing the internal type for a literal.
// allow for u8, u16, u32 and u64

// Defines a literal
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Literal(u32);
/// The invalid literal, used to represent if a clause is no longer valid

impl Literal {
  /// A marker literal to mark invalid values. Not actually unreachable by normal code,
  /// But would require a really large variable set.
  pub const INVALID: Literal = Literal(0u32.wrapping_sub(1));
  pub const fn new(var: u32, negated: bool) -> Self { Self((var << 1) | (negated as u32)) }
  /// returns the value for this literal given these assignments
  pub fn assn(self, assignments: &[Option<bool>]) -> Option<bool> {
    assignments[self.var() as usize].map(|val| self.negated() ^ val)
  }
  /// Returns the variable for this literal as a usize
  /// for convenient indexing
  pub const fn var(self) -> u32 { self.0 >> 1 }
  /// Returns what the var is assigned to if this lit is chosen.
  pub const fn val(self) -> bool { (self.0 & 1) == 0 }
  pub const fn negated(self) -> bool { (self.0 & 1) == 1 }
  pub const fn is_negation(self, o: Self) -> bool { (self.0 ^ 1) == o.0 }
  /// Returns the raw internal of the literal
  // chose not to make this a usize because then it might take extra space on some machines
  // despite the fact that it's always a u32, even though it is only used as an index
  pub const fn raw(self) -> u32 { self.0 }
  pub const fn is_valid(self) -> bool { self.0 != Self::INVALID.0 }
}

impl Not for Literal {
  type Output = Self;
  fn not(self) -> Self::Output { Literal(self.0 ^ 1) }
}

// Reads a literal from dimacs format
impl From<i32> for Literal {
  fn from(i: i32) -> Self {
    assert_ne!(i, 0, "Literal is not well-defined for 0");
    Literal::new((i.abs() as u32) - 1, i < 0)
  }
}

impl From<u32> for Literal {
  fn from(u: u32) -> Self { Literal(u) }
}

#[cfg(test)]
mod test {
  use super::*;
  #[test]
  pub fn test_new_literal() {
    (1..42i32).for_each(|var| {
      let lit = Literal::from(-var);
      assert_eq!(lit.var(), (var - 1) as usize);
      assert!(lit.negated());
      assert_eq!(lit.val(), false);
      assert_eq!((!lit).var(), (var - 1) as usize);
      assert!(!(!lit).negated());
      assert_eq!((!lit).val(), true);
    });
  }
}

impl Display for Literal {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{}", if self.negated() { "!" } else { "" }, self.var())
  }
}
