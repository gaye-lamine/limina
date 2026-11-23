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
  },
  {
    id: 'carnot',
    direction: 'Max',
    title: 'Carnot limit',
    formula: 'efficiency <= 1 - Tc/Th',
    statement: 'A heat engine cannot turn a larger fraction of its heat into work than 1 - Tc/Th.',
    claim_unit: 'dimensionless',
    assumptions: [{ id: 'cycle_model', options: ['ideal_reversible_two_reservoirs'] }],
    parameters: [
      { id: 'Th', kind: 'AbsoluteTemperature', required: true, units: ['K'] },
      { id: 'Tc', kind: 'AbsoluteTemperature', required: true, units: ['K'] },
    ],
    assumption_labels: { cycle_model: 'cycle model' },
    option_labels: { ideal_reversible_two_reservoirs: 'ideal reversible cycle between two reservoirs' },
    parameter_labels: TEMPERATURE_LABELS,
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
