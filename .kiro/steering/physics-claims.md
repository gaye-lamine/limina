---
inclusion: fileMatch
fileMatchPattern: ["docs/**", "demo/**"]
---

# Writing numeric performance claims

- State every numeric performance claim in full: the quantity, the value, the unit, the operating conditions (for example the hot and cold temperatures in kelvin) and the assumptions it depends on.
- Always state the assumptions a limit cannot guess: the mode of a heat pump (heating or cooling), the reference area of a wind rotor (unshrouded rotor, shrouded with total capture area), and every temperature the bound needs.
- Never write a figure that the physics-lint report (`limina check <file>`) flags as IMPOSSIBLE.
- When the report flags a claim, never change a number just to make it pass. Either state the missing assumption, or ask the author to confirm the figure and its conditions.
- Keep the wording of the report: "inconsistent with limit X under assumptions Y". Never use the words fraud, lie, scam or hoax about a person or a company.
- Mark imaginary machines and figures as illustrative.
