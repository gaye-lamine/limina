# MCP demo

**Every result on this page was returned by Kiro.** The five calls were made from a Kiro session through the `limina` MCP server (`limina-mcp`, registered in the workspace file `.kiro/settings/mcp.json`). That file points the server at the public Limina API, `https://limina.backnd-api.cloud`, which has extraction on, so `analyze_text` reads the sentence with a language model there. The results are pasted exactly as Kiro returned them, with nothing edited, shortened or reordered.

The tool list is what Kiro gave back when the five tools were loaded, one entry per tool, each with its description and input schema. Kiro does not show the raw JSON-RPC `tools/list` message, so that message is not on this page.

The page is a record of one session, not a fixture: the API is live and its answers can change. Its verdicts are computed by the deterministic engine of the Limina API, never by the MCP server and never by a language model; the model only reads the sentence in `analyze_text`.

## Tool list (as returned by Kiro)

### `list_limits`

```json
{
  "loaded": [
    {
      "tool_id": "tool::limina::list_limits",
      "resource_identity": {
        "kind": "mcp",
        "serverName": "limina",
        "toolName": "list_limits"
      },
      "description": "List the physical limits in the Limina library: id, title, direction (Max, Min or Equal), claim unit, formula and a one-sentence statement. Call it first to find a limit id.",
      "input_schema": {
        "type": "object",
        "properties": {}
      }
    }
  ],
  "not_found": [],
  "tool_id_format": "resource-v2",
  "note": "Call tool_call with the same tool_id and arguments matching its input_schema. Load it again only if the schema is no longer in context."
}
```

### `explain_limit`

```json
{
  "loaded": [
    {
      "tool_id": "tool::limina::explain_limit",
      "resource_identity": {
        "kind": "mcp",
        "serverName": "limina",
        "toolName": "explain_limit"
      },
      "description": "Describe one limit in full: formula, statement, assumptions with their options, parameters with units, validity domain, limitations and references. Takes an id from list_limits.",
      "input_schema": {
        "type": "object",
        "properties": {
          "id": {
            "description": "The id of a limit, as listed by `list_limits`, for example `carnot`.",
            "type": "string"
          }
        },
        "required": [
          "id"
        ]
      }
    }
  ],
  "not_found": [],
  "tool_id_format": "resource-v2",
  "note": "Call tool_call with the same tool_id and arguments matching its input_schema. Load it again only if the schema is no longer in context."
}
```

### `evaluate_claim`

```json
{
  "loaded": [
    {
      "tool_id": "tool::limina::evaluate_claim",
      "resource_identity": {
        "kind": "mcp",
        "serverName": "limina",
        "toolName": "evaluate_claim"
      },
      "description": "Check a claimed value against a limit with the Limina deterministic engine. Returns the engine's own answer unchanged: verdict (IMPOSSIBLE, CONDITIONAL, COMPATIBLE or INSUFFICIENT_DATA), wording, bounds, margin, configurations and missing parameters. This tool computes nothing itself. constraints: assumption id to the option ids the text allows. parameters: name to value and unit, temperatures in kelvin.",
      "input_schema": {
        "type": "object",
        "$defs": {
          "ParameterInput": {
            "description": "A parameter value with its unit.",
            "properties": {
              "unit": {
                "description": "A unit symbol, for example `K`, `m`, `W` or `dimensionless`.",
                "type": "string"
              },
              "value": {
                "description": "The value, in `unit`.",
                "format": "double",
                "type": "number"
              }
            },
            "required": [
              "value",
              "unit"
            ],
            "type": "object"
          }
        },
        "properties": {
          "constraints": {
            "additionalProperties": {
              "items": {
                "type": "string"
              },
              "type": "array"
            },
            "default": {},
            "description": "Assumption id to the option ids the text allows, for example\n`{\"conversion_process\": [\"heat_engine_cycle\"]}`. An assumption the text\ndoes not state is left out.",
            "type": "object"
          },
          "limit_id": {
            "description": "The id of a limit, for example `carnot`.",
            "type": "string"
          },
          "parameters": {
            "additionalProperties": {
              "$ref": "#/$defs/ParameterInput"
            },
            "default": {},
            "description": "Parameter name to its value and unit, for example\n`{\"Th\": {\"value\": 600, \"unit\": \"K\"}}`. Temperatures are in kelvin.",
            "type": "object"
          },
          "uncertainty": {
            "default": null,
            "description": "The uncertainty of the claim, in the same unit; absent means none.",
            "format": "double",
            "type": [
              "number",
              "null"
            ]
          },
          "unit": {
            "description": "A unit symbol, for example `dimensionless`, `W` or `m/s`.",
            "type": "string"
          },
          "value": {
            "description": "The claimed value, in `unit`.",
            "format": "double",
            "type": "number"
          }
        },
        "required": [
          "limit_id",
          "value",
          "unit"
        ]
      }
    }
  ],
  "not_found": [],
  "tool_id_format": "resource-v2",
  "note": "Call tool_call with the same tool_id and arguments matching its input_schema. Load it again only if the schema is no longer in context."
}
```

### `solve_threshold`

```json
{
  "loaded": [
    {
      "tool_id": "tool::limina::solve_threshold",
      "resource_identity": {
        "kind": "mcp",
        "serverName": "limina",
        "toolName": "solve_threshold"
      },
      "description": "Find the value of one free parameter at which a limit's bound equals the claim, with the engine's verdict just below and just above it. Answers questions such as: how hot must the hot reservoir be for this efficiency to be possible? The search is done by the Limina engine, not by this tool.",
      "input_schema": {
        "type": "object",
        "$defs": {
          "ClaimInput": {
            "description": "A claimed value.",
            "properties": {
              "uncertainty": {
                "description": "The uncertainty of the claim, in the same unit; absent means none.",
                "format": "double",
                "type": [
                  "number",
                  "null"
                ]
              },
              "unit": {
                "description": "A unit symbol, for example `dimensionless`, `W` or `m/s`.",
                "type": "string"
              },
              "value": {
                "description": "The claimed value, in `unit`.",
                "format": "double",
                "type": "number"
              }
            },
            "required": [
              "value",
              "unit"
            ],
            "type": "object"
          },
          "ParameterInput": {
            "description": "A parameter value with its unit.",
            "properties": {
              "unit": {
                "description": "A unit symbol, for example `K`, `m`, `W` or `dimensionless`.",
                "type": "string"
              },
              "value": {
                "description": "The value, in `unit`.",
                "format": "double",
                "type": "number"
              }
            },
            "required": [
              "value",
              "unit"
            ],
            "type": "object"
          }
        },
        "properties": {
          "claim": {
            "$ref": "#/$defs/ClaimInput",
            "description": "The claim to match."
          },
          "constraints": {
            "additionalProperties": {
              "items": {
                "type": "string"
              },
              "type": "array"
            },
            "default": {},
            "description": "Assumption id to the option ids the text allows.",
            "type": "object"
          },
          "free_parameter": {
            "description": "The parameter to solve for, for example `Th`.",
            "type": "string"
          },
          "limit_id": {
            "description": "The id of a limit, for example `carnot`.",
            "type": "string"
          },
          "parameters": {
            "additionalProperties": {
              "$ref": "#/$defs/ParameterInput"
            },
            "default": {},
            "description": "Every parameter other than the free one, with its value and unit.",
            "type": "object"
          }
        },
        "required": [
          "limit_id",
          "claim",
          "free_parameter"
        ]
      }
    }
  ],
  "not_found": [],
  "tool_id_format": "resource-v2",
  "note": "Call tool_call with the same tool_id and arguments matching its input_schema. Load it again only if the schema is no longer in context."
}
```

### `analyze_text`

```json
{
  "loaded": [
    {
      "tool_id": "tool::limina::analyze_text",
      "resource_identity": {
        "kind": "mcp",
        "serverName": "limina",
        "toolName": "analyze_text"
      },
      "description": "Read the numeric performance claims out of a paragraph (at most 5000 characters) with the language model behind the Limina API, then check each one against the library. Needs extraction on the Limina server; when it is not available this tool says so explicitly.",
      "input_schema": {
        "type": "object",
        "properties": {
          "text": {
            "description": "The text to read the claims from, at most 5000 characters.",
            "type": "string"
          }
        },
        "required": [
          "text"
        ]
      }
    }
  ],
  "not_found": [],
  "tool_id_format": "resource-v2",
  "note": "Call tool_call with the same tool_id and arguments matching its input_schema. Load it again only if the schema is no longer in context."
}
```

## Raw results

For `evaluate_claim` and `solve_threshold` the arguments state the heat engine cycle (`conversion_process: heat_engine_cycle`), as the sentence of the `analyze_text` call does.

### 1. `list_limits`

Arguments:

```json
{}
```

Result:

```json
{
  "count": 35,
  "limits": [
    {
      "claim_unit": "dimensionless",
      "classification": "theoretical_bound",
      "direction": "Max",
      "formula": "Cp <= 16/27",
      "id": "betz",
      "statement": "An ideal rotor in unconfined, incompressible flow cannot turn more than 16/27 of the wind's kinetic power through its reference area into shaft power.",
      "title": "Betz limit"
    },
    {
      "claim_unit": "dimensionless",
      "classification": "theorem",
      "direction": "Max",
      "formula": "efficiency <= 1 - Tc/Th",
      "id": "carnot",
      "statement": "A heat engine working between a hot reservoir at Th and a cold reservoir at Tc, both in kelvin, cannot turn a larger fraction of the heat it takes in into work than 1 - Tc/Th.",
      "title": "Carnot limit"
    },
    {
      "claim_unit": "dimensionless",
      "classification": "theorem",
      "direction": "Max",
      "formula": "COP <= Th/(Th-Tc) (heating), COP <= Tc/(Th-Tc) (cooling)",
      "id": "cop",
      "statement": "A heat pump or refrigerator working between a hot reservoir at Th and a cold reservoir at Tc, both in kelvin, cannot reach a coefficient of performance above Th/(Th-Tc) when heating or Tc/(Th-Tc) when cooling.",
      "title": "Carnot COP limit"
    },
    {
      "claim_unit": "m",
      "classification": "theoretical_bound",
      "direction": "Min",
      "formula": "wavelength/(2*aperture)",
      "id": "abbe_limit",
      "statement": "Smallest resolvable period d = lambda / (2 NA) of a far-field optical microscope with a lens of numerical aperture NA, at vacuum wavelength lambda, with conventional (linear, non-saturated) imaging.",
      "title": "Abbe diffraction limit"
    },
    {
      "claim_unit": "W",
      "classification": "theoretical_bound",
      "direction": "Max",
      "formula": "",
      "id": "eddington_luminosity",
      "statement": "Luminosity at which the outward radiation force on fully ionized hydrogen, by Thomson scattering on electrons, balances the inward gravitational force on the protons, for a steady, spherically symmetric flow around a mass M: L_Edd = 4 pi G M m_p c / sigma_T.",
      "title": "Eddington luminosity"
    },
    {
      "claim_unit": "m/s",
      "classification": "theoretical_bound",
      "direction": "Min",
      "formula": "sqrt(2*GM/r)",
      "id": "escape_speed_min",
      "statement": "A body launched at distance r from the centre of a spherically symmetric mass with gravitational parameter GM escapes to infinity only if its launch speed is at least sqrt(2 GM / r), when no further propulsion acts on it (Newtonian gravity, energy conservation).",
      "title": "Minimum speed to escape a body's gravity"
    },
    {
      "claim_unit": "m",
      "classification": "theorem",
      "direction": "Min",
      "formula": "hbar/(2*dp)",
      "id": "heisenberg",
      "statement": "Standard deviations dx and dp of position and momentum along the same axis, for any quantum state of a particle: dx * dp >= hbar / 2 (Kennard 1927). The law bounds dx from below for a given dp.",
      "title": "Heisenberg position-momentum relation"
    },
    {
      "claim_unit": "J",
      "classification": "theoretical_bound",
      "direction": "Min",
      "formula": "kB*T*ln2",
      "id": "landauer",
      "statement": "Heat dissipated into a thermal environment at absolute temperature T by the logically irreversible erasure of one bit of information.",
      "title": "Landauer limit"
    },
    {
      "claim_unit": "N",
      "classification": "theorem",
      "direction": "Max",
      "formula": "abs(charge)*(abs(e_field)+abs(speed)*abs(b_field))",
      "id": "lorentz_force",
      "statement": "Magnitude of the force on a point charge moving at speed `speed` in a field with electric magnitude `e_field` and magnetic magnitude `b_field`, all in one inertial frame. Bound: |F| = |q (E + v x B)| <= |q| (|E| + |v| |B|), by the triangle inequality and |v x B| <= |v| |B|.",
      "title": "Lorentz force magnitude bound"
    },
    {
      "claim_unit": "lm/W",
      "classification": "theoretical_bound",
      "direction": "Max",
      "formula": "Kcd",
      "id": "luminous_efficacy",
      "statement": "Luminous efficacy of radiation (lumens per watt of optical power) of any light, for a standard photopic observer.",
      "title": "Luminous efficacy of radiation"
    },
    {
      "claim_unit": "J",
      "classification": "fundamental_law",
      "direction": "Max",
      "formula": "dm*c^2",
      "id": "mass_energy",
      "statement": "The energy released by converting a mass dm into energy is at most dm c^2; complete conversion (matter-antimatter annihilation) reaches it, nuclear reactions convert only a fraction of their mass.",
      "title": "Mass-energy equivalence"
    },
    {
      "claim_unit": "W",
      "classification": "fundamental_law",
      "direction": "Max",
      "formula": "P_in",
      "id": "power_conservation",
      "statement": "A device in steady state whose only source of energy is the stated input power cannot deliver more output power than it takes in (first law of thermodynamics).",
      "title": "Conservation of energy (power balance)"
    },
    {
      "claim_unit": "rad",
      "classification": "theoretical_bound",
      "direction": "Min",
      "formula": "1.22*wavelength/D",
      "id": "rayleigh_criterion",
      "statement": "Smallest angular separation a diffraction-limited telescope or camera with a single circular aperture of diameter D resolves at wavelength lambda: 1.22 lambda / D by the Rayleigh criterion, 0.947 lambda / D by the stricter Sparrow criterion.",
      "title": "Diffraction limit of a circular aperture"
    },
    {
      "claim_unit": "bit/s",
      "classification": "theorem",
      "direction": "Max",
      "formula": "B*ln(1 + snr)/ln2",
      "id": "shannon_hartley",
      "statement": "Error-free information rate over a band-limited channel of bandwidth B with additive white Gaussian noise and a signal-to-noise power ratio snr (a plain ratio, not decibels).",
      "title": "Shannon-Hartley limit"
    },
    {
      "claim_unit": "dimensionless",
      "classification": "theoretical_bound",
      "direction": "Max",
      "formula": "0.337",
      "id": "shockley_queisser",
      "statement": "Power conversion efficiency of a single-junction solar cell under unconcentrated sunlight (AM1.5G, cell at 300 K): at most 33.7 %, the detailed-balance maximum over all band gaps (reached near 1.34 eV). The limit of any particular material, silicon included, is lower or equal.",
      "title": "Shockley-Queisser limit"
    },
    {
      "claim_unit": "m/s",
      "classification": "theoretical_bound",
      "direction": "Max",
      "formula": "c",
      "id": "speed_limit",
      "statement": "Speed of a body with non-zero rest mass, measured in an inertial frame, through space.",
      "title": "Speed limit of a massive body"
    },
    {
      "claim_unit": "W/m^2",
      "classification": "theorem",
      "direction": "Max",
      "formula": "sigma_SB*T^4",
      "id": "stefan_boltzmann",
      "statement": "Power per unit area emitted into the half space by the surface of a body at uniform absolute temperature T, in vacuum or a non-absorbing medium.",
      "title": "Stefan-Boltzmann limit"
    },
    {
      "claim_unit": "m/s",
      "classification": "theoretical_bound",
      "direction": "Max",
      "formula": "ve*ln(max(m0, mf)/min(m0, mf))",
      "id": "tsiolkovsky",
      "statement": "Non-relativistic rocket with constant effective exhaust speed ve, burning from initial mass m0 to final mass mf, with no external force.",
      "title": "Tsiolkovsky rocket equation"
    },
    {
      "claim_unit": "N",
      "classification": "fundamental_law",
      "direction": "Equal",
      "formula": "predicted = abs(q1*q2)/(4*pi*eps0*r^2)",
      "id": "coulomb",
      "statement": "Magnitude of the electrostatic force between two point charges at rest in vacuum, separated by r.",
      "title": "Coulomb force"
    },
    {
      "claim_unit": "m",
      "classification": "fundamental_law",
      "direction": "Equal",
      "formula": "predicted = h/p",
      "id": "de_broglie",
      "statement": "Wavelength associated with a free particle of momentum magnitude p: lambda = h / p.",
      "title": "de Broglie wavelength"
    },
    {
      "claim_unit": "J",
      "classification": "theorem",
      "direction": "Equal",
      "formula": "predicted = sqrt((p*c)^2 + (m*c^2)^2)",
      "id": "energy_momentum",
      "statement": "Total energy of a free body or particle of rest mass m and momentum magnitude p in an inertial frame, in special relativity.",
      "title": "Relativistic energy-momentum relation"
    },
    {
      "claim_unit": "m/s",
      "classification": "model",
      "direction": "Equal",
      "formula": "predicted = sqrt(2*G*M/r)",
      "id": "escape_velocity",
      "statement": "Speed needed at distance r from the centre of a spherically symmetric, non-rotating mass M to reach infinity at rest, with no atmosphere and no other body.",
      "title": "Escape velocity"
    },
    {
      "claim_unit": "V",
      "classification": "fundamental_law",
      "direction": "Equal",
      "formula": "predicted = n_turns*area*abs(dB)/dt",
      "id": "faraday_emf",
      "statement": "Magnitude of the EMF induced in a coil of n_turns turns of area `area`, fixed and perpendicular to a uniform field whose magnitude changes by dB at a constant rate over a time dt.",
      "title": "Faraday induced EMF"
    },
    {
      "claim_unit": "Hz",
      "classification": "model",
      "direction": "Equal",
      "formula": "predicted = sqrt(8*pi*G*rho/3)",
      "id": "friedmann_flat",
      "statement": "Expansion rate H of a homogeneous, isotropic (FLRW) universe with zero spatial curvature (k = 0), H^2 = 8 pi G rho / 3, where rho is the total mass-energy density of all components divided by c^2, including dark energy treated as an effective density.",
      "title": "Friedmann equation, flat universe"
    },
    {
      "claim_unit": "J",
      "classification": "model",
      "direction": "Equal",
      "formula": "predicted = R_inf*h*c/n^2",
      "id": "hydrogen_levels",
      "statement": "Binding energy |E_n| = R_inf h c / n^2 of the level n of hydrogen in the non-relativistic model with an infinitely heavy nucleus. The claim is the magnitude of the binding energy, the energy E_n itself being negative.",
      "title": "Hydrogen binding energy of level n"
    },
    {
      "claim_unit": "s",
      "classification": "model",
      "direction": "Equal",
      "formula": "predicted = 2*pi*sqrt(a^3/(G*(m1 + m2)))",
      "id": "kepler_third",
      "statement": "Orbital period T = 2 pi sqrt(a^3 / (G (m1 + m2))) of an isolated two-body system under Newtonian gravity, with semi-major axis a.",
      "title": "Kepler's third law"
    },
    {
      "claim_unit": "m",
      "classification": "theorem",
      "direction": "Equal",
      "formula": "predicted = L0*sqrt(1 - (v/c)^2)",
      "id": "length_contraction",
      "statement": "Length along the direction of motion of a body at constant speed v < c, measured in a frame where it moves, given its proper length L0.",
      "title": "Length contraction"
    },
    {
      "claim_unit": "N",
      "classification": "model",
      "direction": "Equal",
      "formula": "predicted = G*m1*m2/r^2",
      "id": "newton_gravity",
      "statement": "Force between two point masses or spherically symmetric bodies at centre distance r, weak field and speeds much below c.",
      "title": "Newtonian gravitational force"
    },
    {
      "claim_unit": "J",
      "classification": "fundamental_law",
      "direction": "Equal",
      "formula": "predicted = abs(m_initial - m_final)*c^2",
      "id": "nuclear_q",
      "statement": "Magnitude of the energy Q = |sum of initial masses - sum of final masses| c^2 released or absorbed by a nuclear reaction, the masses being the total rest masses on each side.",
      "title": "Energy released by a nuclear reaction"
    },
    {
      "claim_unit": "J",
      "classification": "fundamental_law",
      "direction": "Equal",
      "formula": "predicted = h*f",
      "id": "photon_energy",
      "statement": "Energy E = h f of one photon of frequency f in vacuum (Planck-Einstein relation).",
      "title": "Photon energy from frequency"
    },
    {
      "claim_unit": "J",
      "classification": "fundamental_law",
      "direction": "Equal",
      "formula": "predicted = h*c/wavelength",
      "id": "photon_energy_wavelength",
      "statement": "Energy E = h c / lambda of one photon of vacuum wavelength lambda.",
      "title": "Photon energy from wavelength"
    },
    {
      "claim_unit": "dimensionless",
      "classification": "empirical_law",
      "direction": "Equal",
      "formula": "predicted = n0*exp(-t*ln2/t_half)",
      "id": "radioactive_decay",
      "statement": "Expected number N(t) = N0 exp(-t ln2 / T_half) of undecayed nuclei of one isotope after a time t, from N0 nuclei at t = 0.",
      "title": "Radioactive decay law"
    },
    {
      "claim_unit": "J",
      "classification": "theorem",
      "direction": "Equal",
      "formula": "predicted = m*c^2*(1/sqrt(1 - (v/c)^2) - 1)",
      "id": "relativistic_kinetic_energy",
      "statement": "Kinetic energy (gamma - 1) m c^2 of a body of non-zero rest mass m at speed v < c in an inertial frame.",
      "title": "Relativistic kinetic energy"
    },
    {
      "claim_unit": "m",
      "classification": "model",
      "direction": "Equal",
      "formula": "predicted = 2*G*M/c^2",
      "id": "schwarzschild_radius",
      "statement": "Radius of the event horizon of a non-rotating, uncharged mass M in vacuum (Schwarzschild solution of general relativity): r_s = 2 G M / c^2. The claim is the radius in the Schwarzschild coordinate, the circumference divided by 2 pi.",
      "title": "Schwarzschild radius"
    },
    {
      "claim_unit": "s",
      "classification": "theorem",
      "direction": "Equal",
      "formula": "predicted = tau/sqrt(1 - (v/c)^2)",
      "id": "time_dilation",
      "statement": "Interval measured by an inertial observer for a clock moving at constant speed v < c, given its proper time tau.",
      "title": "Time dilation"
    }
  ]
}
```

### 2. `explain_limit` on `carnot`

Arguments:

```json
{"id": "carnot"}
```

Result:

```json
{
  "assumption_labels": {
    "conversion_process": "conversion process",
    "cycle_model": "cycle model"
  },
  "assumptions": [
    {
      "id": "cycle_model",
      "options": [
        "ideal_reversible_two_reservoirs"
      ]
    },
    {
      "id": "conversion_process",
      "options": [
        "heat_engine_cycle",
        "not_a_heat_engine_cycle"
      ]
    }
  ],
  "claim_unit": "dimensionless",
  "classification": "theorem",
  "direction": "Max",
  "formula": "efficiency <= 1 - Tc/Th",
  "id": "carnot",
  "limitations": [
    "Applies to an engine exchanging heat with two reservoirs only",
    "Reached only by a reversible cycle, so a theoretical bound that real engines do not reach"
  ],
  "option_labels": {
    "heat_engine_cycle": "a heat engine cycle between two reservoirs",
    "ideal_reversible_two_reservoirs": "ideal reversible cycle between two reservoirs",
    "not_a_heat_engine_cycle": "a conversion that is not a heat engine cycle"
  },
  "parameter_labels": {
    "Tc": "cold reservoir temperature",
    "Th": "hot reservoir temperature"
  },
  "parameters": [
    {
      "id": "Th",
      "kind": "AbsoluteTemperature",
      "required": true,
      "units": [
        "K"
      ]
    },
    {
      "id": "Tc",
      "kind": "AbsoluteTemperature",
      "required": true,
      "units": [
        "K"
      ]
    }
  ],
  "references": [
    "Carnot, S., Réflexions sur la puissance motrice du feu, Bachelier, Paris (1824)",
    "Callen, H. B., Thermodynamics and an Introduction to Thermostatistics, 2nd ed., Wiley (1985)"
  ],
  "statement": "A heat engine working between a hot reservoir at Th and a cold reservoir at Tc, both in kelvin, cannot turn a larger fraction of the heat it takes in into work than 1 - Tc/Th.",
  "title": "Carnot limit",
  "validity_domain": "Heat engine cycle exchanging heat with two reservoirs at constant absolute temperatures Th > Tc > 0."
}
```

### 3. `evaluate_claim`: Carnot, claim 0.9, Th = 600 K, Tc = 300 K

Arguments:

```json
{"unit": "dimensionless", "limit_id": "carnot", "value": 0.9, "constraints": {"conversion_process": ["heat_engine_cycle"], "cycle_model": ["ideal_reversible_two_reservoirs"]}, "parameters": {"Th": {"value": 600, "unit": "K"}, "Tc": {"value": 300, "unit": "K"}}}
```

Result:

```json
{
  "bounds": {
    "permissive": {
      "b_max": 0.5,
      "b_min": 0.5,
      "margin": {
        "high": 1.8,
        "low": 1.8
      }
    },
    "restrictive": {
      "b_max": 0.5,
      "b_min": 0.5,
      "margin": {
        "high": 1.8,
        "low": 1.8
      }
    }
  },
  "configurations": [],
  "engine_version": "0.1.0",
  "limit_id": "carnot",
  "missing_parameters": [],
  "verdict": "IMPOSSIBLE",
  "wording": "inconsistent with limit carnot under assumptions conversion process = a heat engine cycle between two reservoirs; cycle model = ideal reversible cycle between two reservoirs"
}
```

### 4. `solve_threshold`: Carnot, claim 0.6, free parameter Th, Tc = 300 K

Arguments:

```json
{"claim": {"value": 0.6, "unit": "dimensionless"}, "free_parameter": "Th", "limit_id": "carnot", "constraints": {"conversion_process": ["heat_engine_cycle"], "cycle_model": ["ideal_reversible_two_reservoirs"]}, "parameters": {"Tc": {"value": 300, "unit": "K"}}}
```

Result:

```json
{
  "above": 750.00075,
  "below": 749.99925,
  "direction": "at least",
  "free_parameter": "Th",
  "limit_id": "carnot",
  "monotonicity": "Increasing",
  "range": [
    400.0,
    1000.0
  ],
  "sentence": "Not inconsistent with limit carnot only if Th is at least 750 K (other parameters fixed at Tc = 300 K)",
  "threshold": 750.0,
  "unit": "K",
  "verdict_above": "COMPATIBLE",
  "verdict_below": "IMPOSSIBLE"
}
```

### 5. `analyze_text`

Arguments:

```json
{"text": "The Aurora heat engine runs between 600 K and 300 K at 90 % efficiency."}
```

Result:

```json
{
  "claims": [
    {
      "code": null,
      "constraints": {
        "conversion_process": "heat_engine_cycle"
      },
      "evidence": {
        "end": 71,
        "start": 0,
        "text": "The Aurora heat engine runs between 600 K and 300 K at 90 % efficiency."
      },
      "flags": [
        "constraint not stated in the text"
      ],
      "law": {
        "attempts": [],
        "comparison": null,
        "limit_id": "carnot",
        "sentence": null,
        "source": "built_in"
      },
      "limit_id": "carnot",
      "notes": [
        "90 % becomes 0.90",
        "the text states a heat engine cycle between two reservoirs"
      ],
      "parameters": {
        "Tc": {
          "evidence": {
            "end": 71,
            "start": 0,
            "text": "The Aurora heat engine runs between 600 K and 300 K at 90 % efficiency."
          },
          "flags": [],
          "raw_value": 300.0,
          "unit": "K",
          "value": 300.0
        },
        "Th": {
          "evidence": {
            "end": 71,
            "start": 0,
            "text": "The Aurora heat engine runs between 600 K and 300 K at 90 % efficiency."
          },
          "flags": [],
          "raw_value": 600.0,
          "unit": "K",
          "value": 600.0
        }
      },
      "quantity": "efficiency",
      "raw_unit": "%",
      "raw_value": 90.0,
      "reason": null,
      "result": {
        "bounds": {
          "permissive": {
            "b_max": 0.5,
            "b_min": 0.5,
            "margin": {
              "high": 1.8,
              "low": 1.8
            }
          },
          "restrictive": {
            "b_max": 0.5,
            "b_min": 0.5,
            "margin": {
              "high": 1.8,
              "low": 1.8
            }
          }
        },
        "configurations": [],
        "engine_version": "0.1.0",
        "limit_id": "carnot",
        "missing_parameters": [],
        "verdict": "IMPOSSIBLE",
        "wording": "inconsistent with limit carnot under assumptions conversion process = a heat engine cycle between two reservoirs; cycle model = ideal reversible cycle between two reservoirs"
      },
      "status": "evaluated",
      "trace": [
        {
          "detail": "read efficiency: 90 %, evidence \"The Aurora heat engine runs between 600 K and 300 K at 90 % efficiency.\"; flags: constraint not stated in the text; model nvidia/nemotron-3.5-bolt-e4b; skipped: nvidia/nemotron-3.5-lightning-30b-a3b (the answer does not match the schema)",
          "status": "done",
          "step": "extraction"
        },
        {
          "detail": "built-in limit carnot",
          "status": "done",
          "step": "limit"
        },
        {
          "detail": "found Th = 600 K, Tc = 300 K",
          "status": "done",
          "step": "parameters"
        },
        {
          "detail": "engine verdict IMPOSSIBLE",
          "status": "done",
          "step": "evaluation"
        }
      ],
      "unit": "dimensionless",
      "value": 0.9
    }
  ],
  "extraction": {
    "cached": false,
    "model": "nvidia/nemotron-3.5-bolt-e4b",
    "ms": 3574
  },
  "missing": []
}
```

## Reading the results

- `evaluate_claim`: the engine computes the Carnot bound 1 - 300/600 = 0.5. The claim 0.9 is 1.8 times the bound, the verdict is `IMPOSSIBLE`, and the wording is the engine's own: "inconsistent with limit carnot under assumptions ...".
- `solve_threshold`: for a claim of 0.6, the hot reservoir must be at least 750 K; the engine's verdict is `IMPOSSIBLE` just below and `COMPATIBLE` just above.
- `analyze_text`: the language model read the quantity (90 %, which became 0.90) and the two temperatures with their evidence; the verdict in `result` is the engine's. The trace shows that the first model's answer was skipped because it did not match the schema and a second model answered.
- In `analyze_text`, the flag "constraint not stated in the text" sits next to the note "the text states a heat engine cycle between two reservoirs". Both are the API's output and are left as returned.

## Check it yourself

```
cargo test -p limina-mcp
```

The tests start the real binary against a mock HTTP server. They check the five tool names and schemas, that each request body reaches the API unchanged, that a mock answer of COMPATIBLE for a claim far above the bound comes back as COMPATIBLE (the tool computes no verdict), that an unreachable API is reported, and that a key placed in the environment of the process never appears in any request. The configuration to register the server is in the README, section "MCP server".
