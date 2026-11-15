---

inclusion: always

---

# Product: Limina

Limina reads a physical performance claim (press release, product page), extracts the numbers, and checks them against a deterministic library of fundamental physical limits (Carnot, Betz, Shockley–Queisser, etc.).

## Non-negotiable rules

- The verdict is ALWAYS computed by the deterministic engine, NEVER by an LLM. The LLM only extracts quantities and rephrases an explanation of an already computed result.

- Four verdicts: IMPOSSIBLE (claim exceeds the bound in every configuration compatible with the text), CONDITIONAL (possible only under listed assumptions), COMPATIBLE (within the bound even in the most restrictive configuration), INSUFFICIENT_DATA (a missing parameter changes the verdict; list it).

- When in doubt, answer CONDITIONAL or INSUFFICIENT_DATA, never IMPOSSIBLE. A false IMPOSSIBLE is the worst error.

- Never invent a missing quantity. Mark it missing.

- Wording: say "inconsistent with limit X under assumptions Y". Never use fraud, lie, scam, hoax about any person or company.

- Be honest about the limits of the tool. Cite ONTO Limits as prior art.
