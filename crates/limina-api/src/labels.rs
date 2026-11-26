//! Display labels for the assumption and option ids of the served limits.
//!
//! The engine works with ids. The wording sentence shows these labels
//! instead. An id without a label is shown as it is, never invented.

/// Assumption id to its display label.
const ASSUMPTIONS: &[(&str, &str)] = &[
    ("reference_area", "reference area"),
    ("cycle_model", "cycle model"),
    ("mode", "mode"),
    ("imaging_method", "imaging method"),
    ("propulsion", "propulsion"),
    ("energy_sources", "energy sources"),
    ("cell_type", "cell type"),
    ("resolution_criterion", "resolution criterion"),
];

/// Option id to its display label.
const OPTIONS: &[(&str, &str)] = &[
    ("unshrouded_rotor", "unshrouded rotor"),
    (
        "shrouded_total_capture_area",
        "shrouded, total capture area",
    ),
    ("shrouded_rotor_area_only", "shrouded, rotor area only"),
    (
        "ideal_reversible_two_reservoirs",
        "ideal reversible cycle between two reservoirs",
    ),
    ("conventional_far_field", "conventional far-field imaging"),
    ("super_resolution", "super-resolution techniques"),
    ("heating", "heating"),
    ("cooling", "cooling"),
    ("ballistic", "no further propulsion"),
    ("powered", "powered flight"),
    ("input_only", "the input is the only energy source"),
    ("other_sources", "other energy sources"),
    (
        "single_junction_one_sun",
        "single-junction cell, unconcentrated sunlight",
    ),
    (
        "multi_junction_or_concentrated",
        "multi-junction cell or concentrated sunlight",
    ),
    ("rayleigh", "Rayleigh criterion"),
    ("sparrow", "Sparrow criterion"),
];

/// Parameter id to its display label. The id itself (Th, Tc) is the symbol.
const PARAMETERS: &[(&str, &str)] = &[
    ("Th", "hot reservoir temperature"),
    ("Tc", "cold reservoir temperature"),
    ("T", "temperature"),
    ("wavelength", "wavelength"),
    ("aperture", "numerical aperture"),
    ("D", "aperture diameter"),
    ("dp", "momentum uncertainty"),
    ("p", "momentum"),
    ("M", "mass of the body"),
    ("GM", "gravitational parameter of the body"),
    ("r", "distance"),
    ("m", "mass"),
    ("dm", "mass converted"),
    ("m0", "initial mass"),
    ("mf", "final mass"),
    ("m1", "first mass"),
    ("m2", "second mass"),
    ("m_initial", "initial mass"),
    ("m_final", "final mass"),
    ("ve", "exhaust velocity"),
    ("v", "speed"),
    ("speed", "speed"),
    ("charge", "charge"),
    ("q1", "first charge"),
    ("q2", "second charge"),
    ("e_field", "electric field"),
    ("b_field", "magnetic field"),
    ("B", "bandwidth"),
    ("snr", "signal-to-noise ratio"),
    ("n_turns", "number of turns"),
    ("area", "area"),
    ("dB", "change in magnetic field"),
    ("dt", "duration of the change"),
    ("rho", "density"),
    ("n", "level number"),
    ("a", "semi-major axis"),
    ("L0", "proper length"),
    ("tau", "proper time"),
    ("f", "frequency"),
    ("n0", "initial number"),
    ("t", "elapsed time"),
    ("t_half", "half-life"),
    ("P_in", "input power"),
];

/// Display label of a parameter id; the id itself when it has none.
pub fn parameter_label(id: &str) -> &str {
    lookup(PARAMETERS, id)
}

fn lookup<'a>(table: &[(&str, &'static str)], id: &'a str) -> &'a str {
    table
        .iter()
        .find(|(key, _)| *key == id)
        .map_or(id, |(_, label)| *label)
}

/// Display label of an assumption id; the id itself when it has none.
pub fn assumption_label(id: &str) -> &str {
    lookup(ASSUMPTIONS, id)
}

/// Display label of an option id; the id itself when it has none.
pub fn option_label(id: &str) -> &str {
    lookup(OPTIONS, id)
}

#[cfg(test)]
mod tests {
    use limina_core::{Betz, Carnot, Cop, Limit};

    use super::*;

    #[test]
    fn labels_of_the_examples() {
        assert_eq!(assumption_label("reference_area"), "reference area");
        assert_eq!(option_label("unshrouded_rotor"), "unshrouded rotor");
        assert_eq!(assumption_label("cycle_model"), "cycle model");
        assert_eq!(
            option_label("ideal_reversible_two_reservoirs"),
            "ideal reversible cycle between two reservoirs"
        );
        assert_eq!(assumption_label("mode"), "mode");
        assert_eq!(option_label("heating"), "heating");
        assert_eq!(option_label("cooling"), "cooling");
    }

    #[test]
    fn every_assumption_and_option_of_the_served_limits_has_a_label() {
        let limits: [Box<dyn Limit>; 3] = [
            Box::new(Betz::new()),
            Box::new(Carnot::new()),
            Box::new(Cop::new()),
        ];
        for limit in &limits {
            for p in limit.parameters() {
                let id = p.id.as_str();
                assert!(
                    PARAMETERS.iter().any(|(k, _)| *k == id),
                    "{}: parameter {id} has no label",
                    limit.id()
                );
            }
            for a in limit.assumptions() {
                let id = a.id.as_str();
                assert!(
                    ASSUMPTIONS.iter().any(|(k, _)| *k == id),
                    "{}: assumption {id} has no label",
                    limit.id()
                );
                for o in &a.domain {
                    let id = o.id.as_str();
                    assert!(
                        OPTIONS.iter().any(|(k, _)| *k == id),
                        "{}: option {id} has no label",
                        limit.id()
                    );
                }
            }
        }
    }

    #[test]
    fn an_unknown_id_is_shown_as_it_is() {
        assert_eq!(assumption_label("something"), "something");
        assert_eq!(option_label("x"), "x");
    }

    #[test]
    fn labels_avoid_the_banned_words() {
        for (_, label) in ASSUMPTIONS.iter().chain(OPTIONS).chain(PARAMETERS) {
            for word in label.split(|c: char| !c.is_ascii_alphanumeric()) {
                assert!(!["fraud", "lie", "scam", "hoax"].contains(&word), "{label}");
            }
        }
    }
}
