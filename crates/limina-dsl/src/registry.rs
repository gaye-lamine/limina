//! Loads declarative limits from a directory of JSON files.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use limina_core::Limit;

use crate::definition::{DirectionName, LimitDefinition};
use crate::equality::EqualityLimit;
use crate::error::DslError;
use crate::limit::DeclarativeLimit;
use crate::report::{check_limit, CheckReport};

/// The `data/limits` directory of this repository.
pub fn default_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/limits")
}

/// The loaded limits, sorted by id.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    limits: Vec<DeclarativeLimit>,
    equalities: Vec<EqualityLimit>,
}

impl Registry {
    /// Loads every `*.json` file of `dir`. A file that does not parse or does
    /// not compile, or a repeated id, refuses the whole directory.
    pub fn load_dir(dir: &Path) -> Result<Registry, DslError> {
        let io = |p: &Path, e: std::io::Error| DslError::Io {
            path: p.display().to_string(),
            message: e.to_string(),
        };
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| io(dir, e))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        let mut definitions = Vec::new();
        for path in files {
            let text = std::fs::read_to_string(&path).map_err(|e| io(&path, e))?;
            let definition: LimitDefinition =
                serde_json::from_str(&text).map_err(|e| DslError::Json {
                    path: path.display().to_string(),
                    message: e.to_string(),
                })?;
            definitions.push(definition);
        }
        Registry::from_definitions(definitions)
    }

    /// The definitions of `data/limits`, embedded at build time, so a
    /// program finds them wherever it runs.
    pub fn builtin() -> Result<Registry, DslError> {
        let files = [
            ("betz.json", include_str!("../../../data/limits/betz.json")),
            (
                "carnot.json",
                include_str!("../../../data/limits/carnot.json"),
            ),
            ("cop.json", include_str!("../../../data/limits/cop.json")),
            (
                "landauer.json",
                include_str!("../../../data/limits/landauer.json"),
            ),
            (
                "lorentz_force.json",
                include_str!("../../../data/limits/lorentz_force.json"),
            ),
            (
                "shannon_hartley.json",
                include_str!("../../../data/limits/shannon_hartley.json"),
            ),
            (
                "stefan_boltzmann.json",
                include_str!("../../../data/limits/stefan_boltzmann.json"),
            ),
            (
                "energy_momentum.json",
                include_str!("../../../data/limits/energy_momentum.json"),
            ),
            (
                "escape_velocity.json",
                include_str!("../../../data/limits/escape_velocity.json"),
            ),
            (
                "kepler_third.json",
                include_str!("../../../data/limits/kepler_third.json"),
            ),
            (
                "length_contraction.json",
                include_str!("../../../data/limits/length_contraction.json"),
            ),
            (
                "newton_gravity.json",
                include_str!("../../../data/limits/newton_gravity.json"),
            ),
            (
                "relativistic_kinetic_energy.json",
                include_str!("../../../data/limits/relativistic_kinetic_energy.json"),
            ),
            (
                "speed_limit.json",
                include_str!("../../../data/limits/speed_limit.json"),
            ),
            (
                "time_dilation.json",
                include_str!("../../../data/limits/time_dilation.json"),
            ),
            (
                "abbe_limit.json",
                include_str!("../../../data/limits/abbe_limit.json"),
            ),
            (
                "coulomb.json",
                include_str!("../../../data/limits/coulomb.json"),
            ),
            (
                "faraday_emf.json",
                include_str!("../../../data/limits/faraday_emf.json"),
            ),
            (
                "luminous_efficacy.json",
                include_str!("../../../data/limits/luminous_efficacy.json"),
            ),
            (
                "photon_energy.json",
                include_str!("../../../data/limits/photon_energy.json"),
            ),
            (
                "photon_energy_wavelength.json",
                include_str!("../../../data/limits/photon_energy_wavelength.json"),
            ),
            (
                "de_broglie.json",
                include_str!("../../../data/limits/de_broglie.json"),
            ),
            (
                "heisenberg.json",
                include_str!("../../../data/limits/heisenberg.json"),
            ),
            (
                "hydrogen_levels.json",
                include_str!("../../../data/limits/hydrogen_levels.json"),
            ),
            (
                "nuclear_q.json",
                include_str!("../../../data/limits/nuclear_q.json"),
            ),
            (
                "radioactive_decay.json",
                include_str!("../../../data/limits/radioactive_decay.json"),
            ),
            (
                "eddington_luminosity.json",
                include_str!("../../../data/limits/eddington_luminosity.json"),
            ),
            (
                "friedmann_flat.json",
                include_str!("../../../data/limits/friedmann_flat.json"),
            ),
            (
                "schwarzschild_radius.json",
                include_str!("../../../data/limits/schwarzschild_radius.json"),
            ),
            (
                "tsiolkovsky.json",
                include_str!("../../../data/limits/tsiolkovsky.json"),
            ),
        ];
        let mut definitions = Vec::new();
        for (name, text) in files {
            definitions.push(serde_json::from_str(text).map_err(|e| DslError::Json {
                path: name.to_string(),
                message: e.to_string(),
            })?);
        }
        Registry::from_definitions(definitions)
    }

    /// Compiles these definitions.
    pub fn from_definitions(definitions: Vec<LimitDefinition>) -> Result<Registry, DslError> {
        let mut seen = BTreeSet::new();
        let mut limits = Vec::new();
        let mut equalities = Vec::new();
        for d in definitions {
            if !seen.insert(d.id.clone()) {
                return Err(DslError::InvalidDefinition {
                    limit_id: d.id.clone(),
                    message: "the id is used by another definition".to_string(),
                });
            }
            if d.direction == DirectionName::Equal {
                equalities.push(EqualityLimit::compile(d)?);
            } else {
                limits.push(DeclarativeLimit::compile(d)?);
            }
        }
        limits.sort_by(|a, b| a.id().cmp(b.id()));
        equalities.sort_by(|a, b| a.id().cmp(b.id()));
        Ok(Registry { limits, equalities })
    }

    /// The limits, sorted by id.
    pub fn limits(&self) -> &[DeclarativeLimit] {
        &self.limits
    }

    /// The limit with this id.
    pub fn get(&self, id: &str) -> Option<&DeclarativeLimit> {
        self.limits.iter().find(|l| l.id() == id)
    }

    /// The equality laws, sorted by id.
    pub fn equalities(&self) -> &[EqualityLimit] {
        &self.equalities
    }

    /// The equality law with this id.
    pub fn equality(&self, id: &str) -> Option<&EqualityLimit> {
        self.equalities.iter().find(|l| l.id() == id)
    }

    /// The limits as core `Limit`s.
    pub fn as_limits(&self) -> Vec<&dyn Limit> {
        self.limits.iter().map(|l| l as &dyn Limit).collect()
    }

    /// The check report of every limit.
    pub fn reports(&self) -> Vec<CheckReport> {
        self.limits.iter().map(check_limit).collect()
    }
}
