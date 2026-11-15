---

inclusion: fileMatch

fileMatchPattern: "crates/**/tests/**/*.rs"

---

# Testing rules

- Every decision rule needs property-based tests. Use fixed seeds in CI; keep failing cases as regression tests.

- Required property: never classify a value below the bound as IMPOSSIBLE.

- Shrunk counterexamples must print in a human-readable format.

- Unit tests alone are not enough for the decision engine.
