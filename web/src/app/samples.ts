/**
 * Built-in samples: illustrative texts about imaginary products. Each one
 * also carries its quantities as structured data, so that it can be checked
 * by the engine (POST /v1/evaluate) when the extraction model is not
 * available. The structured quantities are written here by hand from the
 * sample text; the verdicts still come from the engine.
 */
import { AnalyzedClaim } from './analysis.models';
import { EvaluateRequest, EvaluateResponse } from './api.models';
import { UI } from './strings';
import { charCount } from './workbench-logic';

export interface SampleQuantity {
  value: number;
  unit: string;
  raw_value: number;
  raw_unit: string;
  /** The exact words of the sample text the quantity is read from. */
  evidence: string;
}

export interface SampleClaim extends SampleQuantity {
  quantity: string;
  limit_id: string | null;
  parameters: Record<string, SampleQuantity>;
  constraints: Record<string, string>;
}

export interface Sample {
  id: string;
  label: string;
  text: string;
  claims: SampleClaim[];
}

const q = (value: number, unit: string, evidence: string, raw_value = value, raw_unit = unit): SampleQuantity => ({
  value,
  unit,
  raw_value,
  raw_unit,
  evidence,
});

/** The pre-filled text: synthetic claims about imaginary products, one per discipline. */
export const ILLUSTRATIVE_TEXT = [
  'The Helios probe cruises at a speed of 3.6e8 m/s relative to the Sun.',
  'The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.',
  'The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.',
  'The Aurora heat pump delivers heat with a COP of 14.5, hot reservoir at 300 K, cold reservoir at 280 K.',
  'The Quanta logic chip erases one bit at 300 K while dissipating 1e-22 J.',
  'The Volta cell stores 5 kWh of energy in a mass of 1 kg.',
].join('\n\n');

export const SAMPLES: Sample[] = [
  {
    id: 'six-laws',
    label: UI.sampleSix,
    text: ILLUSTRATIVE_TEXT,
    claims: [
      { quantity: 'speed of a massive body', limit_id: 'speed_limit', ...q(3.6e8, 'm/s', '3.6e8 m/s'), parameters: {}, constraints: {} },
      {
        quantity: 'position uncertainty',
        limit_id: 'heisenberg',
        ...q(1e-12, 'm', '1e-12 m'),
        parameters: { dp: q(1e-24, 'kg*m/s', '1e-24 kg*m/s') },
        constraints: {},
      },
      {
        quantity: 'resolution',
        limit_id: 'abbe_limit',
        ...q(1e-7, 'm', '100 nm', 100, 'nm'),
        parameters: { wavelength: q(5e-7, 'm', '500 nm', 500, 'nm'), aperture: q(1.4, 'dimensionless', '1.4') },
        constraints: {},
      },
      {
        quantity: 'coefficient of performance',
        limit_id: 'cop',
        ...q(14.5, 'dimensionless', '14.5'),
        parameters: { Th: q(300, 'K', '300 K'), Tc: q(280, 'K', '280 K') },
        constraints: {},
      },
      {
        quantity: 'energy dissipated per bit erased',
        limit_id: 'landauer',
        ...q(1e-22, 'J', '1e-22 J'),
        parameters: { T: q(300, 'K', '300 K') },
        constraints: {},
      },
      { quantity: 'specific energy', limit_id: null, ...q(5, 'kWh/kg', '5 kWh of energy in a mass of 1 kg'), parameters: {}, constraints: {} },
    ],
  },
  {
    id: 'heat',
    label: UI.sampleHeat,
    text: [
      'The Ember turbine converts heat to work with an efficiency of 60%, between a hot source at 600 K and a cold sink at 300 K.',
      'The Brook engine reaches an efficiency of 40% between a source at 600 K and a sink at 300 K.',
      'The Frost heat pump heats a house with a COP of 4, taking heat from the ground at 275 K and delivering it at 295 K.',
    ].join('\n\n'),
    claims: [
      {
        quantity: 'thermal efficiency',
        limit_id: 'carnot',
        ...q(0.6, 'dimensionless', '60%', 60, '%'),
        parameters: { Th: q(600, 'K', '600 K'), Tc: q(300, 'K', '300 K') },
        // A turbine and an engine: the text states a heat engine cycle.
        constraints: { conversion_process: 'heat_engine_cycle' },
      },
      {
        quantity: 'thermal efficiency',
        limit_id: 'carnot',
        ...q(0.4, 'dimensionless', '40%', 40, '%'),
        parameters: { Th: q(600, 'K', '600 K'), Tc: q(300, 'K', '300 K') },
        // A turbine and an engine: the text states a heat engine cycle.
        constraints: { conversion_process: 'heat_engine_cycle' },
      },
      {
        quantity: 'coefficient of performance',
        limit_id: 'cop',
        ...q(4, 'dimensionless', 'COP of 4'),
        parameters: { Tc: q(275, 'K', '275 K'), Th: q(295, 'K', '295 K') },
        constraints: { mode: 'heating' },
      },
    ],
  },
  {
    id: 'wind-optics',
    label: UI.sampleWind,
    text: [
      'The Gale rotor, an unshrouded wind turbine, extracts 70% of the power of the wind crossing its swept area.',
      'The Breeze rotor, also unshrouded, extracts 45% of that power.',
      'The Iris objective resolves details of 300 nm with light of 500 nm wavelength and a numerical aperture of 0.9, in conventional far-field imaging.',
    ].join('\n\n'),
    claims: [
      { quantity: 'power coefficient', limit_id: 'betz', ...q(0.7, 'dimensionless', '70%', 70, '%'), parameters: {}, constraints: { reference_area: 'unshrouded_rotor' } },
      { quantity: 'power coefficient', limit_id: 'betz', ...q(0.45, 'dimensionless', '45%', 45, '%'), parameters: {}, constraints: { reference_area: 'unshrouded_rotor' } },
      {
        quantity: 'resolution',
        limit_id: 'abbe_limit',
        ...q(3e-7, 'm', '300 nm', 300, 'nm'),
        parameters: { wavelength: q(5e-7, 'm', '500 nm', 500, 'nm'), aperture: q(0.9, 'dimensionless', '0.9') },
        constraints: { imaging_method: 'conventional_far_field' },
      },
    ],
  },
];

/** The sample whose text is exactly `text`, if any. */
export function sampleOf(text: string): Sample | null {
  return SAMPLES.find((s) => s.text === text) ?? null;
}

/** Offsets in code points of `needle` in `text`, searched from UTF-16 index `from`. */
function locate(text: string, needle: string, from: number): { start: number; end: number; next: number } | null {
  const at = text.indexOf(needle, from);
  if (at < 0) return null;
  const start = charCount(text.slice(0, at));
  return { start, end: start + charCount(needle), next: at + needle.length };
}

/** The evaluate request of a sample claim; null for a claim with no built-in law. */
export function evaluateRequest(c: SampleClaim): EvaluateRequest | null {
  if (!c.limit_id) return null;
  return {
    limit_id: c.limit_id,
    claim: { value: c.value, unit: c.unit },
    constraints: Object.fromEntries(Object.entries(c.constraints).map(([a, o]) => [a, [o]])),
    parameters: Object.fromEntries(Object.entries(c.parameters).map(([id, p]) => [id, { value: p.value, unit: p.unit }])),
  };
}

/**
 * The sample claims as analysed claims, with evidence offsets located in
 * the sample text, the engine result of each one (null when not evaluated)
 * and a trace that says no language model was used.
 */
export function sampleClaims(sample: Sample, results: (EvaluateResponse | null)[]): AnalyzedClaim[] {
  let cursor = 0;
  return sample.claims.map((c, i) => {
    const ev = locate(sample.text, c.evidence, cursor);
    if (ev) cursor = ev.next;
    const parameters = Object.fromEntries(
      Object.entries(c.parameters).map(([id, p]) => {
        const pe = locate(sample.text, p.evidence, ev ? sample.text.lastIndexOf('\n\n', ev.next) + 1 : 0);
        return [
          id,
          {
            value: p.value,
            raw_value: p.raw_value,
            unit: p.unit,
            evidence: pe ? { text: p.evidence, start: pe.start, end: pe.end } : null,
            flags: [],
          },
        ];
      }),
    );
    const result = results[i] ?? null;
    return {
      limit_id: c.limit_id,
      quantity: c.quantity,
      value: c.value,
      unit: c.unit,
      raw_value: c.raw_value,
      raw_unit: c.raw_unit,
      evidence: ev ? { text: c.evidence, start: ev.start, end: ev.end } : null,
      constraints: c.constraints,
      parameters,
      notes: [],
      flags: [],
      status: result ? 'evaluated' : 'needs_input',
      result,
      reason: result ? null : c.limit_id ? UI.sampleNotEvaluated : UI.sampleNoLaw,
      code: null,
      trace: [
        { step: 'extraction', status: 'skipped', detail: UI.sampleTrace },
        result
          ? { step: 'evaluation', status: 'done', detail: `engine verdict ${result.verdict}` }
          : { step: 'limit', status: 'failed', detail: UI.sampleNoLaw },
      ],
      law: { source: c.limit_id ? 'built_in' : 'no_law_identified', limit_id: c.limit_id, sentence: null, attempts: [], comparison: null },
    };
  });
}
