import { AnalyzedClaim } from './analysis.models';
import { LimitInfo } from './api.models';

/** Display labels of the temperatures, as served by the API. */
const TEMPERATURE_LABELS = { Th: 'hot reservoir temperature', Tc: 'cold reservoir temperature' };

/** Limit metadata shaped like GET /v1/limits, shared by component tests. */
export const LIMITS: LimitInfo[] = [
  {
    id: 'betz',
    direction: 'Max',
    title: 'Betz limit',
    formula: 'Cp <= 16/27',
    statement: 'An ideal rotor cannot turn more than 16/27 of the kinetic power into shaft power.',
    claim_unit: 'dimensionless',
    assumptions: [
      { id: 'reference_area', options: ['unshrouded_rotor', 'shrouded_rotor_area_only'] },
    ],
    parameters: [],
    assumption_labels: { reference_area: 'reference area' },
    option_labels: {
      unshrouded_rotor: 'unshrouded rotor',
      shrouded_rotor_area_only: 'shrouded, rotor area only',
    },
    parameter_labels: {},
    classification: 'theoretical_bound',
    references: ['Betz, A., Zeitschrift fuer das gesamte Turbinenwesen 26, 307-309 (1920)'],
    validity_domain: 'Actuator disk in steady, incompressible, unconfined axial flow.',
    limitations: ['A bound of the model, not a figure that real turbines reach'],
  },
  {
    id: 'carnot',
    direction: 'Max',
    title: 'Carnot limit',
    formula: 'efficiency <= 1 - Tc/Th',
    statement: 'A heat engine cannot turn a larger fraction of its heat into work than 1 - Tc/Th.',
    claim_unit: 'dimensionless',
    assumptions: [
      { id: 'cycle_model', options: ['ideal_reversible_two_reservoirs'] },
      { id: 'conversion_process', options: ['heat_engine_cycle', 'not_a_heat_engine_cycle'] },
    ],
    parameters: [
      { id: 'Th', kind: 'AbsoluteTemperature', required: true, units: ['K'] },
      { id: 'Tc', kind: 'AbsoluteTemperature', required: true, units: ['K'] },
    ],
    assumption_labels: { cycle_model: 'cycle model', conversion_process: 'conversion process' },
    option_labels: {
      ideal_reversible_two_reservoirs: 'ideal reversible cycle between two reservoirs',
      heat_engine_cycle: 'a heat engine cycle between two reservoirs',
      not_a_heat_engine_cycle: 'a conversion that is not a heat engine cycle',
    },
    parameter_labels: TEMPERATURE_LABELS,
    classification: 'theorem',
  },
];

/** The COP limit, whose bound depends on an assumption (CONDITIONAL is possible). */
export const COP_LIMIT: LimitInfo = {
  id: 'cop',
  direction: 'Max',
  title: 'Carnot COP limit',
  formula: 'COP <= Th/(Th-Tc) (heating), COP <= Tc/(Th-Tc) (cooling)',
  statement: 'A heat pump or refrigerator cannot exceed the Carnot coefficient of performance.',
  claim_unit: 'dimensionless',
  assumptions: [{ id: 'mode', options: ['heating', 'cooling'] }],
  parameters: [
    { id: 'Th', kind: 'AbsoluteTemperature', required: true, units: ['K'] },
    { id: 'Tc', kind: 'AbsoluteTemperature', required: true, units: ['K'] },
  ],
  assumption_labels: { mode: 'mode' },
  option_labels: { heating: 'heating', cooling: 'cooling' },
  parameter_labels: TEMPERATURE_LABELS,
};

/** A limit whose parameter allows several units, to test the unit select. */
export const LIMIT_SEVERAL_UNITS: LimitInfo = {
  id: 'power',
  direction: 'Max',
  title: 'Power limit',
  formula: 'P <= P_max',
  statement: 'A test limit.',
  claim_unit: 'W',
  assumptions: [],
  parameters: [{ id: 'P', kind: 'Plain', required: false, units: ['W', 'kW', 'MW'] }],
  assumption_labels: {},
  option_labels: {},
  parameter_labels: { P: 'power' },
};

/** An analysed claim for tests: Carnot, 60 % at 600 K, IMPOSSIBLE; over replaces fields. */
export function claimOf(over: Partial<AnalyzedClaim> = {}): AnalyzedClaim {
  return {
    limit_id: 'carnot',
    quantity: 'efficiency',
    value: 0.6,
    unit: 'dimensionless',
    raw_value: 60,
    raw_unit: '%',
    evidence: { text: '60%', start: 3, end: 6 },
    constraints: {},
    parameters: { Th: { value: 600, raw_value: 600, unit: 'K', evidence: { text: '600 K', start: 10, end: 15 }, flags: [] } },
    notes: [],
    flags: [],
    status: 'evaluated',
    result: {
      verdict: 'IMPOSSIBLE',
      limit_id: 'carnot',
      engine_version: '0.1.0',
      wording: 'inconsistent with limit carnot under assumptions cycle model = ideal',
      bounds: {
        permissive: { b_min: 0.5, b_max: 0.5, margin: { low: 1.2, high: 1.2 } },
        restrictive: { b_min: 0.5, b_max: 0.5, margin: { low: 1.2, high: 1.2 } },
      },
      configurations: [],
      missing_parameters: [],
    },
    reason: null,
    code: null,
    trace: [],
    law: { source: 'built_in', limit_id: 'carnot', sentence: null, attempts: [], comparison: null },
    ...over,
  };
}
