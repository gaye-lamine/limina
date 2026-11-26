import { STRINGS } from './strings';

/** A one-click illustrative claim. Units come from the limit metadata, never from here. */
export interface Example {
  label: string;
  limitId: string;
  /** Claim value as typed in the form. */
  claim: string;
  /** Assumption id to chosen option id. */
  constraints: Record<string, string>;
  /** Parameter id to its value as typed in the form. Parameters not listed stay empty. */
  parameters: Record<string, string>;
}

export const EXAMPLES: Example[] = [
  {
    label: STRINGS.exampleBetzImpossible,
    limitId: 'betz',
    claim: '0.70',
    constraints: { reference_area: 'unshrouded_rotor' },
    parameters: {},
  },
  {
    label: STRINGS.exampleBetzCompatible,
    limitId: 'betz',
    claim: '0.45',
    constraints: { reference_area: 'unshrouded_rotor' },
    parameters: {},
  },
  {
    label: STRINGS.exampleCarnotImpossible,
    limitId: 'carnot',
    claim: '0.60',
    constraints: { conversion_process: 'heat_engine_cycle' },
    parameters: { Th: '600', Tc: '300' },
  },
  {
    label: STRINGS.exampleCarnotMissing,
    limitId: 'carnot',
    claim: '0.40',
    constraints: {},
    parameters: {},
  },
  {
    label: STRINGS.exampleCopConditional,
    limitId: 'cop',
    claim: '14.5',
    constraints: {},
    parameters: { Th: '300', Tc: '280' },
  },
];
