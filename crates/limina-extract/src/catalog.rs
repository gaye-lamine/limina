//! The limits metadata the extraction works with, passed in by the API.

/// An option of an assumption.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogOption {
    /// Option id, as the engine knows it.
    pub id: String,
    /// Display label, for example "unshrouded rotor".
    pub label: String,
}

/// An assumption and its options.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogAssumption {
    /// Assumption id.
    pub id: String,
    /// Its options.
    pub options: Vec<CatalogOption>,
}

/// A parameter and its allowed units.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogParameter {
    /// Parameter name, for example "Th".
    pub name: String,
    /// Allowed unit symbols.
    pub units: Vec<String>,
}

/// A served limit.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogLimit {
    /// Limit id.
    pub id: String,
    /// Display title, for example "Carnot limit".
    pub title: String,
    /// The bound as text.
    pub formula: String,
    /// Unit of the claimed quantity, for example "dimensionless".
    pub claim_unit: String,
    /// Declared parameters.
    pub parameters: Vec<CatalogParameter>,
    /// Declared assumptions.
    pub assumptions: Vec<CatalogAssumption>,
}

/// Every served limit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Catalog {
    /// The limits, in listing order.
    pub limits: Vec<CatalogLimit>,
}

impl Catalog {
    /// The limit with this id.
    pub fn limit(&self, id: &str) -> Option<&CatalogLimit> {
        self.limits.iter().find(|l| l.id == id)
    }
}

impl CatalogLimit {
    /// The parameter with this name.
    pub fn parameter(&self, name: &str) -> Option<&CatalogParameter> {
        self.parameters.iter().find(|p| p.name == name)
    }

    /// The assumption with this id.
    pub fn assumption(&self, id: &str) -> Option<&CatalogAssumption> {
        self.assumptions.iter().find(|a| a.id == id)
    }
}
