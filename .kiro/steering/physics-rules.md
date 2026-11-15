---

inclusion: fileMatch

fileMatchPattern: "crates/limina-core/src/limits/**/*.rs"

---

# Rules for every physical limit

- Declare: id, name, direction (Max or Min bound), bounded quantity dimension, assumptions with their domains, validity domain, references, epistemic status (theorem | model bound | contested).

- NEVER invent a constant, formula, or reference value. Use only values given in the spec. If a value is missing, STOP and ask the author.

- Use typed quantities; reject dimensionally incompatible inputs with a typed error.

- Temperatures are absolute (kelvin) inside the engine.

- Include golden-value tests taken only from values written in the spec.

- Include property tests for monotonicity and safety bounds listed in the spec.
