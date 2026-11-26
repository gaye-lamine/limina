//! Deterministic choice of a built-in law for a claim, tried before any
//! model answer is trusted and before any law is authored.
//!
//! A built-in law matches a claim when both hold:
//! - the dimension of the claim unit equals the dimension of the quantity
//!   the law bounds (its claim unit), and
//! - the quantity name contains one of the law's keywords (whole words,
//!   case and punctuation ignored). A weak keyword (for example
//!   "efficiency" for Carnot) counts only when the claim's sentence also
//!   states a number in a unit of one of the law's parameters.
//!
//! The longest matching keyword wins. The model's `limit_id` is only a hint:
//! it breaks a tie, and it is kept when no keyword matches but its law has
//! the claim's dimension. Nothing else is guessed: a tie the hint does not
//! break gives no match.

use crate::catalog::{Catalog, CatalogLimit};
use crate::check::{locate, numbers_with_units, same_unit, sentence_around};
use crate::parse::ModelClaim;

/// Keywords of one built-in law: strong ones match alone, weak ones need a
/// parameter of the law stated in the sentence.
struct Keywords {
    id: &'static str,
    strong: &'static [&'static str],
    weak: &'static [&'static str],
}

const PHOTON: &[&str] = &[
    "photon energy",
    "energy of a photon",
    "energy per photon",
    "energy of the photon",
    "energy of each photon",
];

const KEYWORDS: &[Keywords] = &[
    Keywords {
        id: "betz",
        strong: &[
            "power coefficient",
            "betz",
            "wind turbine efficiency",
            "rotor efficiency",
            "fraction of the wind power",
            "fraction of the power of the wind",
            "share of the wind power",
            "wind power extracted",
            "power extracted from the wind",
        ],
        weak: &[],
    },
    Keywords {
        id: "carnot",
        strong: &[
            "thermal efficiency",
            "carnot",
            "heat engine efficiency",
            "engine efficiency",
            "heat to work efficiency",
            "turbine thermal efficiency",
        ],
        weak: &["efficiency", "conversion efficiency"],
    },
    Keywords {
        id: "cop",
        strong: &[
            "coefficient of performance",
            "cop",
            "heat pump cop",
            "refrigeration cop",
            "cooling cop",
            "heating cop",
        ],
        weak: &["performance coefficient"],
    },
    Keywords {
        id: "abbe_limit",
        strong: &[
            "resolution",
            "resolution limit",
            "spatial resolution",
            "lateral resolution",
            "resolving power",
            "smallest resolvable detail",
            "resolvable detail",
            "diffraction limit",
            "resolved detail size",
        ],
        weak: &["detail size", "feature size"],
    },
    Keywords {
        id: "eddington_luminosity",
        strong: &[
            "luminosity",
            "eddington",
            "eddington luminosity",
            "radiated luminosity",
        ],
        weak: &[],
    },
    Keywords {
        id: "heisenberg",
        strong: &[
            "position uncertainty",
            "uncertainty in position",
            "uncertainty of position",
            "positional uncertainty",
            "position precision",
            "localization precision",
            "localization",
            "position spread",
            "heisenberg",
        ],
        weak: &["uncertainty"],
    },
    Keywords {
        id: "landauer",
        strong: &[
            "landauer",
            "energy per bit",
            "energy per erased bit",
            "energy per bit erased",
            "energy dissipated per bit",
            "energy dissipated per bit erased",
            "bit erasure energy",
            "erasure energy",
            "energy to erase a bit",
            "energy to erase one bit",
            "dissipation per bit",
            "energy per bit erasure",
        ],
        weak: &[],
    },
    Keywords {
        id: "lorentz_force",
        strong: &[
            "lorentz force",
            "force on a charge",
            "force on the charge",
            "force on a moving charge",
            "magnetic force on a charge",
            "electromagnetic force on a charge",
        ],
        weak: &[],
    },
    Keywords {
        id: "luminous_efficacy",
        strong: &[
            "luminous efficacy",
            "efficacy",
            "lumens per watt",
            "luminous efficiency",
        ],
        weak: &[],
    },
    Keywords {
        id: "shannon_hartley",
        strong: &[
            "channel capacity",
            "data rate",
            "bit rate",
            "transmission rate",
            "throughput",
            "shannon",
            "information rate",
            "link capacity",
        ],
        weak: &["capacity", "rate"],
    },
    Keywords {
        id: "speed_limit",
        strong: &[
            "speed",
            "velocity",
            "cruise speed",
            "cruising speed",
            "travel speed",
            "top speed",
            "speed of a massive body",
        ],
        weak: &[],
    },
    Keywords {
        id: "stefan_boltzmann",
        strong: &[
            "radiant exitance",
            "emissive power",
            "radiated power per unit area",
            "radiated power per area",
            "radiant flux density",
            "radiated flux",
            "thermal radiation flux",
            "radiated heat flux",
        ],
        weak: &["power per unit area", "heat flux", "flux"],
    },
    Keywords {
        id: "tsiolkovsky",
        strong: &[
            "delta v",
            "change in velocity",
            "velocity change",
            "velocity increment",
            "rocket delta v",
            "speed gain",
            "change in speed",
        ],
        weak: &[],
    },
    Keywords {
        id: "coulomb",
        strong: &[
            "coulomb force",
            "electrostatic force",
            "electric force",
            "force between the charges",
            "force between charges",
            "force between two charges",
        ],
        weak: &[],
    },
    Keywords {
        id: "de_broglie",
        strong: &[
            "de broglie",
            "de broglie wavelength",
            "matter wavelength",
            "matter wave wavelength",
        ],
        weak: &[],
    },
    Keywords {
        id: "energy_momentum",
        strong: &[
            "total energy",
            "relativistic energy",
            "relativistic total energy",
        ],
        weak: &[],
    },
    Keywords {
        id: "escape_velocity",
        strong: &["escape velocity", "escape speed"],
        weak: &[],
    },
    Keywords {
        id: "faraday_emf",
        strong: &[
            "emf",
            "induced emf",
            "induced voltage",
            "electromotive force",
            "induced electromotive force",
        ],
        weak: &["voltage"],
    },
    Keywords {
        id: "friedmann_flat",
        strong: &[
            "hubble parameter",
            "hubble constant",
            "hubble rate",
            "expansion rate",
        ],
        weak: &[],
    },
    Keywords {
        id: "hydrogen_levels",
        strong: &[
            "hydrogen binding energy",
            "binding energy of level",
            "energy of level",
            "hydrogen energy level",
            "energy level",
            "ionization energy",
        ],
        weak: &["binding energy"],
    },
    Keywords {
        id: "kepler_third",
        strong: &[
            "orbital period",
            "orbit period",
            "period of revolution",
            "revolution period",
        ],
        weak: &["period"],
    },
    Keywords {
        id: "length_contraction",
        strong: &[
            "contracted length",
            "length contraction",
            "observed length",
            "measured length",
        ],
        weak: &["length"],
    },
    Keywords {
        id: "newton_gravity",
        strong: &[
            "gravitational force",
            "gravitational attraction",
            "force of gravity",
            "gravitational pull",
        ],
        weak: &[],
    },
    Keywords {
        id: "nuclear_q",
        strong: &[
            "energy released",
            "released energy",
            "q value",
            "reaction energy",
            "energy liberated",
        ],
        weak: &[],
    },
    Keywords {
        id: "photon_energy",
        strong: &[],
        weak: PHOTON,
    },
    Keywords {
        id: "photon_energy_wavelength",
        strong: &[],
        weak: PHOTON,
    },
    Keywords {
        id: "radioactive_decay",
        strong: &[
            "remaining fraction",
            "fraction remaining",
            "surviving fraction",
            "fraction left",
            "undecayed fraction",
            "remaining nuclei",
        ],
        weak: &[],
    },
    Keywords {
        id: "relativistic_kinetic_energy",
        strong: &["kinetic energy", "relativistic kinetic energy"],
        weak: &[],
    },
    Keywords {
        id: "schwarzschild_radius",
        strong: &[
            "schwarzschild radius",
            "event horizon radius",
            "horizon radius",
        ],
        weak: &[],
    },
    Keywords {
        id: "time_dilation",
        strong: &[
            "time dilation",
            "dilated time",
            "dilated duration",
            "elapsed time",
            "observed time",
            "measured time",
        ],
        weak: &["time", "duration"],
    },
    Keywords {
        id: "escape_speed_min",
        strong: &[
            "launch speed",
            "launch velocity",
            "initial speed",
            "initial velocity",
            "departure speed",
            "speed at launch",
            // Also a keyword of escape_velocity; only an object that escapes
            // (the context below) makes this law win with its context point.
            "escape speed",
            "escape velocity",
        ],
        weak: &["speed", "velocity"],
    },
    Keywords {
        id: "power_conservation",
        strong: &[
            "output power",
            "power output",
            "delivered power",
            "power delivered",
            "electrical output",
            "electric output power",
        ],
        weak: &[],
    },
    Keywords {
        id: "shockley_queisser",
        strong: &[
            "solar cell efficiency",
            "photovoltaic efficiency",
            "shockley queisser",
        ],
        weak: &[
            "efficiency",
            "conversion efficiency",
            "power conversion efficiency",
            "cell efficiency",
        ],
    },
    Keywords {
        id: "rayleigh_criterion",
        strong: &[
            "angular resolution",
            "angular resolving power",
            "resolution angle",
            "angular separation",
            "diffraction limited resolution",
        ],
        weak: &["resolution", "resolving power"],
    },
    Keywords {
        id: "mass_energy",
        strong: &[
            "energy released",
            "released energy",
            "energy produced",
            "energy output",
            "energy yield",
        ],
        weak: &["energy"],
    },
];

/// Laws that match only when their sentence contains one of these phrases
/// (whole words). With the phrase present, their weak keywords count as strong
/// and their score gains two points (more than a strong one-word keyword
/// scores over a weak one), so they win over a law that only shares
/// the quantity name.
const CONTEXT: &[(&str, &[&str])] = &[
    (
        "escape_speed_min",
        &[
            "escapes",
            "escaping",
            "escaped",
            "to escape",
            "can escape",
            "will escape",
            "break free",
            "breaks free",
            "leave the gravity",
            "leaves the gravity",
        ],
    ),
    (
        "shockley_queisser",
        &[
            "solar cell",
            "solar cells",
            "photovoltaic",
            "pv cell",
            "solar panel",
            "solar module",
        ],
    ),
    (
        "mass_energy",
        &[
            "convert",
            "converts",
            "converted",
            "converting",
            "conversion of",
            "annihilation",
            "annihilates",
            "annihilating",
        ],
    ),
];

/// Cues that state an option of an assumption: every group must have one
/// phrase in the sentence (whole words, case and punctuation ignored).
const OPTION_CUES: &[(&str, &[&[&str]])] = &[
    (
        "ballistic",
        &[&[
            "no further propulsion",
            "without propulsion",
            "no propulsion",
            "ballistic",
            "unpowered",
            "coasting",
            "no engine",
        ]],
    ),
    (
        "powered",
        &[&[
            "powered flight",
            "under thrust",
            "with propulsion",
            "engines firing",
        ]],
    ),
    (
        "input_only",
        &[&[
            "only energy source",
            "only source of energy",
            "only power source",
            "only source of power",
            "sole energy source",
            "sole source of energy",
            "no other energy source",
            "no other source of energy",
        ]],
    ),
    (
        "other_sources",
        &[&[
            "other energy sources",
            "additional energy source",
            "also burns fuel",
        ]],
    ),
    (
        "single_junction_one_sun",
        &[
            &["single junction"],
            &[
                "unconcentrated",
                "non concentrated",
                "one sun",
                "1 sun",
                "am1 5",
                "am 1 5",
            ],
        ],
    ),
    (
        "multi_junction_or_concentrated",
        &[&[
            "multi junction",
            "multijunction",
            "tandem",
            "concentrated sunlight",
            "concentrator",
            "under concentration",
        ]],
    ),
    (
        "unshrouded_rotor",
        &[&[
            "unshrouded",
            "open rotor",
            "bare rotor",
            "without a duct",
            "ductless",
        ]],
    ),
    ("shrouded_rotor_area_only", &[&["rotor area only"]]),
    (
        "conventional_far_field",
        &[&[
            "conventional",
            "wide field",
            "widefield",
            "bright field",
            "brightfield",
        ]],
    ),
    (
        "super_resolution",
        &[&[
            "sted",
            "palm",
            "storm",
            "super resolution",
            "structured illumination",
            "near field",
            "localization microscopy",
            "localisation microscopy",
        ]],
    ),
    ("rayleigh", &[&["rayleigh"]]),
    ("sparrow", &[&["sparrow"]]),
];

/// The context phrases of a law, when it has some.
fn context_of(id: &str) -> Option<&'static [&'static str]> {
    CONTEXT.iter().find(|(k, _)| *k == id).map(|(_, c)| *c)
}

/// Noun phrases that do not count as context: "Earth's escape velocity is
/// 11.2 km/s" names a property of a body, not an object that escapes.
const NOT_CONTEXT: &[&str] = &[
    "escape speed",
    "escape velocity",
    "escape speeds",
    "escape velocities",
];

/// Whether the sentence has a context phrase of a law, outside the noun
/// phrases of [`NOT_CONTEXT`].
fn in_context(sentence: &str, phrases: &[&str]) -> bool {
    let mut w = words(sentence);
    for p in NOT_CONTEXT {
        w = w.replace(&words(p), " ");
    }
    has_any(&w, phrases)
}

/// Whether a sentence (as words) contains one of these phrases.
fn has_any(sentence_words: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| sentence_words.contains(&words(p)))
}

/// The option of `options` that the sentence states, when exactly one is
/// stated by its cues; None when none or several are.
pub fn stated_option<'a>(sentence: &str, options: &[&'a str]) -> Option<&'a str> {
    let w = words(sentence);
    let stated: Vec<&'a str> = options
        .iter()
        .copied()
        .filter(|o| {
            OPTION_CUES
                .iter()
                .find(|(id, _)| id == o)
                .is_some_and(|(_, groups)| groups.iter().all(|g| has_any(&w, g)))
        })
        .collect();
    match stated.as_slice() {
        [one] => Some(one),
        _ => None,
    }
}

/// How a built-in law was chosen.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchedBy {
    /// A keyword of the law found in the quantity name, with the dimension.
    Keyword(&'static str),
    /// The model's choice, with the dimension, when no keyword matched or to break a tie.
    Hint,
}

/// A built-in law chosen for a claim.
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltInMatch {
    /// Id of the law in the catalog.
    pub id: String,
    /// How it was chosen.
    pub by: MatchedBy,
}

/// Lower case, every run of characters other than letters and digits made
/// one space, padded with spaces so that phrases match whole words.
fn words(s: &str) -> String {
    let mut out = String::from(" ");
    let mut gap = false;
    for c in s.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            if gap && out.len() > 1 {
                out.push(' ');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
    }
    out.push(' ');
    out
}

/// SI exponents of a unit symbol; "", "%" and "dimensionless" are dimensionless.
fn dimension(unit: &str) -> Option<[i8; 7]> {
    let u = unit.trim();
    if u.is_empty() || u == "%" || u.eq_ignore_ascii_case("dimensionless") {
        return Some([0; 7]);
    }
    limina_core::parse_unit(u)
        .ok()
        .map(|p| p.dimension.exponents())
}

/// Whether the claim unit has the dimension of the quantity the law bounds.
fn same_dimension(limit: &CatalogLimit, unit: &str) -> bool {
    match (dimension(&limit.claim_unit), dimension(unit)) {
        (Some(a), Some(b)) => a == b,
        _ => same_unit(&limit.claim_unit, unit),
    }
}

/// Whether the sentence states a number in a unit of one of the law's parameters.
fn states_a_parameter(limit: &CatalogLimit, sentence: Option<&str>) -> bool {
    let Some(sentence) = sentence else {
        return false;
    };
    let found = numbers_with_units(sentence);
    limit.parameters.iter().any(|p| {
        found
            .iter()
            .any(|(_, u, _)| p.units.iter().any(|pu| same_unit(pu, u)))
    })
}

/// The best keyword of a law in this quantity: its length in words, then the keyword.
fn best_keyword(k: &Keywords, quantity: &str, weak_allowed: bool) -> Option<(usize, &'static str)> {
    let strong = k.strong.iter().map(|p| (p, true));
    let weak = k.weak.iter().map(|p| (p, false));
    strong
        .chain(weak)
        .filter(|(p, is_strong)| (*is_strong || weak_allowed) && quantity.contains(&words(p)))
        .map(|(p, is_strong)| {
            (
                2 * words(p).split_whitespace().count() + usize::from(is_strong),
                *p,
            )
        })
        .max()
}

/// The built-in law of a claim, chosen deterministically; see the module
/// documentation. `text` is the text the claim was read from.
pub fn match_built_in(claim: &ModelClaim, text: &str, catalog: &Catalog) -> Option<BuiltInMatch> {
    let quantity = words(&claim.quantity);
    let sentence = locate(text, &claim.evidence).map(|span| sentence_around(text, &span));
    let mut scored: Vec<(usize, &str, &'static str)> = Vec::new();
    for k in KEYWORDS {
        let Some(limit) = catalog.limit(k.id) else {
            continue;
        };
        if !same_dimension(limit, &claim.unit) {
            continue;
        }
        let in_context = match context_of(k.id) {
            Some(phrases) => {
                if !sentence.as_deref().is_some_and(|s| in_context(s, phrases)) {
                    continue;
                }
                true
            }
            None => false,
        };
        let weak_allowed = in_context || states_a_parameter(limit, sentence.as_deref());
        if let Some((score, phrase)) = best_keyword(k, &quantity, weak_allowed) {
            scored.push((score + 2 * usize::from(in_context), k.id, phrase));
        }
    }
    let hint = claim
        .limit_id
        .as_deref()
        .and_then(|id| catalog.limit(id))
        .filter(|l| same_dimension(l, &claim.unit));
    let top = scored.iter().map(|(s, _, _)| *s).max();
    let best: Vec<&(usize, &str, &'static str)> =
        scored.iter().filter(|(s, _, _)| Some(*s) == top).collect();
    if let Some(h) = hint {
        if best.iter().any(|(_, id, _)| *id == h.id) || best.is_empty() {
            let by = best
                .iter()
                .find(|(_, id, _)| *id == h.id)
                .map_or(MatchedBy::Hint, |(_, _, p)| MatchedBy::Keyword(p));
            return Some(BuiltInMatch {
                id: h.id.clone(),
                by,
            });
        }
    }
    match best.as_slice() {
        [(_, id, phrase)] => Some(BuiltInMatch {
            id: (*id).to_string(),
            by: MatchedBy::Keyword(phrase),
        }),
        _ => None,
    }
}

/// Ids of the built-in laws that have keywords, for tests and docs.
pub fn laws_with_keywords() -> Vec<&'static str> {
    KEYWORDS.iter().map(|k| k.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_matched_whole_and_punctuation_is_ignored() {
        assert_eq!(
            words("Position-momentum (uncertainty)"),
            " position momentum uncertainty "
        );
        assert!(!words("coefficient of performance").contains(&words("cop")));
        assert!(words("the COP").contains(&words("cop")));
        assert!(!words("copper loss").contains(&words("cop")));
    }
}
