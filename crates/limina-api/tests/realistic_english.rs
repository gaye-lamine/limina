//! Routing and evaluation from realistic English sentences in the units
//! texts use: °C, arcsec, µs, solar masses, au, years, Mbps, km/s/Mpc, and
//! the dimensionless parameters named next to their numbers. The model's
//! answer is mocked (no network) and gives no law id: the deterministic
//! matcher chooses the law, the checks convert the units with a note, and
//! the engine decides. Expected values are worked out in the comments from
//! the textbook formulas with the constants written out.
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use limina_api::{registry, router_with, AppState};
use limina_extract::testing::{MockProvider, NamedMock};
use limina_extract::Extractor;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn analyze(reading: Value, text: &str) -> Value {
    let a = Arc::new(MockProvider::new());
    let b = Arc::new(NamedMock::new("model-b"));
    let state = AppState::with_extractor(
        Extractor::new(a.clone(), registry::catalog()).with_retry_backoff(Some(Duration::ZERO)),
    )
    .with_second_model(b);
    a.push_content(&reading.to_string());
    a.push_content("No law identified.");
    let request = Request::builder()
        .method("POST")
        .uri("/v1/analyze")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "text": text }).to_string()))
        .expect("request");
    let response = router_with(state)
        .oneshot(request)
        .await
        .expect("infallible");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("json")
}

/// One claim read with no law id, and these parameters (often none: the
/// checks read them from the sentence).
fn reading(quantity: &str, value: f64, unit: &str, evidence: &str, parameters: Value) -> Value {
    json!({"claims": [{"limit_id": null, "quantity": quantity, "value": value, "unit": unit,
        "evidence": evidence, "constraints": {}, "parameters": parameters}], "missing": []})
}

/// The law and the verdict of the only claim, and the claim.
async fn decide(reading: Value, text: &str) -> (String, String, Value) {
    let body = analyze(reading, text).await;
    let claims = body["claims"].as_array().expect("claims");
    assert_eq!(claims.len(), 1, "{body}");
    let c = claims[0].clone();
    (
        c["limit_id"].as_str().unwrap_or("").to_string(),
        c["result"]["verdict"].as_str().unwrap_or("").to_string(),
        c,
    )
}

fn notes(c: &Value) -> String {
    c["notes"].to_string()
}

// --- Thermodynamics in °C ---

/// 327 °C = 600.15 K, 27 °C = 300.15 K: Carnot bound 1 - 300.15/600.15 = 0.4999.
#[tokio::test]
async fn a_steam_turbine_in_celsius_is_checked_against_carnot() {
    let text = "The Ember steam turbine converts heat to work with a thermal efficiency of 60% between a boiler at 327 °C and a condenser at 27 °C.";
    let params = json!({"Th": {"value": 327, "unit": "°C", "evidence": "boiler at 327 °C"},
                        "Tc": {"value": 27, "unit": "°C", "evidence": "condenser at 27 °C"}});
    let (law, verdict, c) = decide(
        reading(
            "thermal efficiency",
            60.0,
            "%",
            "thermal efficiency of 60%",
            params.clone(),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("carnot", "IMPOSSIBLE"),
        "{c}"
    );
    assert!(notes(&c).contains("Th = 327 °C becomes 600.15 K"), "{c}");
    let text40 = text.replace("60%", "40%");
    let (_, verdict, c) = decide(
        reading(
            "thermal efficiency",
            40.0,
            "%",
            "thermal efficiency of 40%",
            params,
        ),
        &text40,
    )
    .await;
    assert_eq!(verdict, "COMPATIBLE", "{c}");
}

/// 27 °C = 300.15 K, read from the sentence: kB T ln2 = 2.873e-21 J > 1e-22 J.
#[tokio::test]
async fn landauer_at_a_temperature_in_celsius() {
    let text = "The Quanta logic chip erases one bit at 27 °C while dissipating 1e-22 J.";
    let (law, verdict, c) = decide(
        reading(
            "energy dissipated per bit erased",
            1e-22,
            "J",
            "dissipating 1e-22 J",
            json!({}),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("landauer", "IMPOSSIBLE"),
        "{c}"
    );
    assert_eq!(c["parameters"]["T"]["value"], 300.15, "{c}");
}

/// 620 °F = 599.82 K, 80 °F = 299.82 K: Carnot bound 1 - 299.82/599.82 = 0.5002.
#[tokio::test]
async fn a_steam_turbine_in_fahrenheit_is_checked_against_carnot() {
    let text = "The Ember steam turbine converts heat to work with a thermal efficiency of 60% between a boiler at 620 °F and a condenser at 80 °F.";
    let params = json!({"Th": {"value": 620, "unit": "°F", "evidence": "boiler at 620 °F"},
                        "Tc": {"value": 80, "unit": "°F", "evidence": "condenser at 80 °F"}});
    let (law, verdict, c) = decide(
        reading(
            "thermal efficiency",
            60.0,
            "%",
            "thermal efficiency of 60%",
            params,
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("carnot", "IMPOSSIBLE"),
        "{c}"
    );
    assert!(notes(&c).contains("Th = 620 °F becomes"), "{c}");
}

// --- Engineering units: hp, miles, BTU ---

/// 150 hp = 150 × 745.70 W = 111.85 kW from a 100 kW input, its only source.
#[tokio::test]
async fn a_generator_in_horsepower_is_checked_against_energy_conservation() {
    let text = "The Atlas generator delivers an output power of 150 hp from an input power of 100 kW, its only energy source.";
    let (law, verdict, c) = decide(
        reading(
            "output power",
            150.0,
            "hp",
            "output power of 150 hp",
            json!({"P_in": {"value": 100, "unit": "kW", "evidence": "input power of 100 kW"}}),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("power_conservation", "IMPOSSIBLE"),
        "{c}"
    );
    assert!(notes(&c).contains("150 hp becomes"), "{c}");
}

/// Earth's escape velocity, M = 5.972e24 kg, r = 3959 miles = 6.371e6 m:
/// sqrt(2GM/r) = 11.19 km/s; 11.2 km/s is within 0.5 %.
#[tokio::test]
async fn an_escape_velocity_with_a_radius_in_miles() {
    let text =
        "For a planet of 5.972e24 kg and a radius of 3959 miles, the escape velocity is 11.2 km/s.";
    let (law, verdict, c) = decide(
        reading(
            "escape velocity",
            11.2,
            "km/s",
            "11.2 km/s",
            json!({"M": {"value": 5.972e24, "unit": "kg", "evidence": "5.972e24 kg"},
                   "r": {"value": 3959, "unit": "miles", "evidence": "3959 miles"}}),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("escape_velocity", "COMPATIBLE"),
        "{c}"
    );
}

/// 1e7 erg = 1 J of energy released from 1 mg: dm c² = 8.99e10 J ≥ 1 J.
#[tokio::test]
async fn an_energy_in_erg_is_converted_before_the_engine() {
    let text = "The Nova cell releases 1e7 erg of energy by converting 1 mg of mass.";
    let (law, verdict, c) = decide(
        reading("energy released", 1e7, "erg", "1e7 erg", json!({})),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("mass_energy", "COMPATIBLE"),
        "{c}"
    );
    assert_eq!(c["value"], 1.0, "{c}");
}

// --- Optics in arcsec ---

/// 550 nm through 0.2 m: Sparrow 0.947 λ/D = 2.604e-6 rad = 0.537 arcsec,
/// Rayleigh 1.22 λ/D = 3.355e-6 rad = 0.692 arcsec.
#[tokio::test]
async fn a_telescope_resolution_in_arcsec_is_checked_against_rayleigh() {
    let text = |theta: &str| {
        format!("The Lyra telescope reaches an angular resolution of {theta} arcsec at a wavelength of 550 nm with an aperture diameter of 0.2 m.")
    };
    for (theta, value, expected) in [
        ("0.5", 0.5, "IMPOSSIBLE"),
        ("0.6", 0.6, "CONDITIONAL"),
        ("0.8", 0.8, "COMPATIBLE"),
    ] {
        let (law, verdict, c) = decide(
            reading(
                "angular resolution",
                value,
                "arcsec",
                &format!("{theta} arcsec"),
                json!({"wavelength": {"value": 550, "unit": "nm", "evidence": "550 nm"},
                       "D": {"value": 0.2, "unit": "m", "evidence": "0.2 m"}}),
            ),
            &text(theta),
        )
        .await;
        assert_eq!(
            (law.as_str(), verdict.as_str()),
            ("rayleigh_criterion", expected),
            "{theta} arcsec: {c}"
        );
        assert!(notes(&c).contains("arcsec becomes"), "{c}");
    }
}

// --- Quantum ---

/// ħ/(2 Δp) = 1.054572e-34 / 2e-24 = 5.273e-11 m > 5e-11 m.
#[tokio::test]
async fn heisenberg_from_a_realistic_sentence() {
    let text = "The Nanoscope locates an electron within 5e-11 m while its momentum is known to within 1e-24 kg*m/s.";
    let (law, verdict, c) = decide(
        reading(
            "position uncertainty",
            5e-11,
            "m",
            "within 5e-11 m",
            json!({}),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("heisenberg", "IMPOSSIBLE"),
        "{c}"
    );
}

/// h/p = 6.62607e-34 / 1e-24 = 0.6626 nm; 0.66 nm is within 0.5 %, 0.5 nm is not.
#[tokio::test]
async fn de_broglie_from_a_realistic_sentence() {
    for (lambda, expected) in [("0.66", "COMPATIBLE"), ("0.5", "IMPOSSIBLE")] {
        let text = format!("Electrons with a momentum of 1e-24 kg*m/s have a de Broglie wavelength of {lambda} nm.");
        let value: f64 = lambda.parse().expect("number");
        let (law, verdict, c) = decide(
            reading(
                "de Broglie wavelength",
                value,
                "nm",
                &format!("{lambda} nm"),
                json!({}),
            ),
            &text,
        )
        .await;
        assert_eq!(
            (law.as_str(), verdict.as_str()),
            ("de_broglie", expected),
            "{lambda}: {c}"
        );
    }
}

// --- Relativity in µs ---

/// v/c = 2.97e8/2.99792458e8 = 0.990686, γ = 7.3436, γ τ = 16.156 µs:
/// 16.2 µs is within 0.5 %, 15.6 µs is not.
#[tokio::test]
async fn muon_time_dilation_in_microseconds() {
    for (life, expected) in [("16.2", "COMPATIBLE"), ("15.6", "IMPOSSIBLE")] {
        let text = format!("Muons with a proper lifetime of 2.2 µs moving at 2.97e8 m/s have an observed time of {life} µs in the lab.");
        let value: f64 = life.parse().expect("number");
        let (law, verdict, c) = decide(
            reading(
                "observed time",
                value,
                "µs",
                &format!("{life} µs"),
                json!({"tau": {"value": 2.2, "unit": "µs", "evidence": "proper lifetime of 2.2 µs"},
                       "v": {"value": 2.97e8, "unit": "m/s", "evidence": "2.97e8 m/s"}}),
            ),
            &text,
        )
        .await;
        assert_eq!(
            (law.as_str(), verdict.as_str()),
            ("time_dilation", expected),
            "{life}: {c}"
        );
    }
}

// --- Astrophysics and cosmology ---

/// 2 G M / c² with M = 10 solar masses = 10 GM_sun/G: 2 × 1.3271244e21 /
/// 8.98755e16 = 29 532 m; 29.5 km is within 0.5 %, 20 km is not.
#[tokio::test]
async fn a_black_hole_in_solar_masses_is_checked_against_schwarzschild() {
    for (radius, expected) in [("29.5", "COMPATIBLE"), ("20", "IMPOSSIBLE")] {
        let text =
            format!("A black hole of 10 solar masses has an event horizon radius of {radius} km.");
        let value: f64 = radius.parse().expect("number");
        let (law, verdict, c) = decide(
            reading(
                "event horizon radius",
                value,
                "km",
                &format!("{radius} km"),
                json!({}),
            ),
            &text,
        )
        .await;
        assert_eq!(
            (law.as_str(), verdict.as_str()),
            ("schwarzschild_radius", expected),
            "{radius} km: {c}"
        );
        assert!(notes(&c).contains("10 solar masses becomes"), "{c}");
    }
}

/// a = 1 au, m1 = 1 solar mass, m2 = 6e24 kg: P = 2π sqrt(a³/(G(m1+m2))) =
/// 3.15583e7 s = 1.00002 Julian years; 1.0 years agrees, 2 years does not.
#[tokio::test]
async fn an_orbit_in_au_and_years_is_checked_against_kepler() {
    for (period, expected) in [("1.0", "COMPATIBLE"), ("2", "IMPOSSIBLE")] {
        let text = format!("An exoplanet of 6e24 kg orbits a star of 1 solar mass at 1 au with an orbital period of {period} years.");
        let value: f64 = period.parse().expect("number");
        let (law, verdict, c) = decide(
            reading(
                "orbital period",
                value,
                "years",
                &format!("{period} years"),
                json!({"a": {"value": 1, "unit": "au", "evidence": "at 1 au"},
                       "m1": {"value": 1, "unit": "solar mass", "evidence": "1 solar mass"},
                       "m2": {"value": 6e24, "unit": "kg", "evidence": "6e24 kg"}}),
            ),
            &text,
        )
        .await;
        assert_eq!(
            (law.as_str(), verdict.as_str()),
            ("kepler_third", expected),
            "{period} years: {c}"
        );
    }
}

/// sqrt(8πGρ/3) with ρ = 9.2e-27 kg/m³: 2.2681e-18 s⁻¹; 70 km/s/Mpc =
/// 70e3 / 3.0857e22 = 2.2685e-18 s⁻¹, within 0.5 %.
#[tokio::test]
async fn a_hubble_constant_in_km_s_mpc_is_checked_against_friedmann() {
    let text = "The model universe has a Hubble constant of 70 km/s/Mpc for a mean density of 9.2e-27 kg/m^3.";
    let (law, verdict, c) = decide(
        reading(
            "Hubble constant",
            70.0,
            "km/s/Mpc",
            "70 km/s/Mpc",
            json!({}),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("friedmann_flat", "COMPATIBLE"),
        "{c}"
    );
}

// --- Information in Mbps ---

/// 1 MHz × log2(1 + 15) = 4 Mbit/s; 10 Mbps is 2.5 times that. The ratio
/// is read from "signal-to-noise ratio of 15" when the model leaves it out.
#[tokio::test]
async fn a_modem_in_mbps_is_checked_against_shannon() {
    let text =
        "The Echo modem transmits 10 Mbps over a 1 MHz channel with a signal-to-noise ratio of 15.";
    let (law, verdict, c) = decide(
        reading(
            "data rate",
            10.0,
            "Mbps",
            "10 Mbps",
            json!({"B": {"value": 1, "unit": "MHz", "evidence": "1 MHz"}}),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("shannon_hartley", "IMPOSSIBLE"),
        "{c}"
    );
    assert_eq!(c["parameters"]["snr"]["value"], 15.0, "{c}");
}

// --- Dimensionless parameters named next to their numbers ---

/// N A ΔB/Δt = 200 × 50e-4 m² × 0.2 T / 0.02 s = 10 V.
#[tokio::test]
async fn faraday_reads_the_number_of_turns_next_to_its_number() {
    let text = "A coil of 200 turns with an area of 50 cm^2 sees the field change by 0.2 T in 20 ms and gives an induced EMF of 10 V.";
    let (law, verdict, c) = decide(
        reading(
            "induced emf",
            10.0,
            "V",
            "induced EMF of 10 V",
            json!({"area": {"value": 50, "unit": "cm^2", "evidence": "50 cm^2"},
                   "dB": {"value": 0.2, "unit": "T", "evidence": "0.2 T"},
                   "dt": {"value": 20, "unit": "ms", "evidence": "20 ms"}}),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("faraday_emf", "COMPATIBLE"),
        "{c}"
    );
    assert_eq!(c["parameters"]["n_turns"]["value"], 200.0, "{c}");
}

/// R∞ h c / n² for n = 3: 13.6057 / 9 = 1.51174 eV; 1.511 eV is within 0.1 %.
#[tokio::test]
async fn hydrogen_reads_the_level_next_to_its_number() {
    let text = "The n = 3 level of hydrogen has a binding energy of 1.511 eV.";
    let (law, verdict, c) = decide(
        reading(
            "hydrogen binding energy",
            1.511,
            "eV",
            "1.511 eV",
            json!({}),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("hydrogen_levels", "COMPATIBLE"),
        "{c}"
    );
    assert_eq!(c["parameters"]["n"]["value"], 3.0, "{c}");
}

/// N0 exp(-t ln2 / T½) = 1e6 × 2^-2 = 2.5e5 after two half-lives, the times
/// in years converted to seconds.
#[tokio::test]
async fn decay_reads_the_initial_nuclei_and_times_in_years() {
    let text = "Of 1e6 initial atoms with a half-life of 5730 years, 2.5e5 remaining nuclei are left after 11460 years.";
    let (law, verdict, c) = decide(
        reading(
            "remaining nuclei",
            2.5e5,
            "dimensionless",
            "2.5e5 remaining nuclei",
            json!({"t_half": {"value": 5730, "unit": "years", "evidence": "half-life of 5730 years"},
                   "t": {"value": 11460, "unit": "years", "evidence": "after 11460 years"}}),
        ),
        text,
    )
    .await;
    assert_eq!(
        (law.as_str(), verdict.as_str()),
        ("radioactive_decay", "COMPATIBLE"),
        "{c}"
    );
    assert_eq!(c["parameters"]["n0"]["value"], 1e6, "{c}");
    assert!(notes(&c).contains("5730 years becomes"), "{c}");
}
