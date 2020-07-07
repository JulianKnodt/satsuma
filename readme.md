# Satsuma 🍊

A SAT solver based off of Minisat written in Rust.
It should offer the same efficiency as MiniSAT, but further testing still needs to be done.
It's intended to be highly readable as compared to the original codebase, which was small
spaghetti but spaghetti nonetheless.

---

The use of unsafe in this code is purely to bypass bounds checking, because it's hard for the
compiler to reason about bounds checks for variables since they cannot be shown to be smaller
than the size of a vector locally.

