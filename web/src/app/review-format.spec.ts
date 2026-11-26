import { EvaluateResponse } from './api.models';
import {
  boundDisplay,
  calculation,
  decisive,
  doiUrl,
  focusOption,
  formalStatement,
  inClaimUnit,
  missingAssumptions,
  plainNumber,
  prettyFormula,
  ratioDisplay,
  readableAssumptions,
  stateAssumptions,
  toCanonical,
  valuesUsed,
} from './review-format';
import { LIMITS } from './testing-fixtures';
import { claimOf } from './workbench-logic.spec';

const carnot = LIMITS.find((l) => l.id === 'carnot')!;
const betz = LIMITS.find((l) => l.id === 'betz')!;

describe('display of engine results', () => {
  it('writes formulas with real symbols', () => {
    expect(prettyFormula('efficiency <= 1 - Tc/Th')).toBe('efficiency ≤ 1 − Tc/Th');
    expect(prettyFormula('kB*T*ln2')).toBe('kB·T·ln2');
    expect(prettyFormula('COP <= Th/(Th-Tc)')).toBe('COP ≤ Th/(Th−Tc)');
  });

  it('names the law in the formal statement instead of its id, keeping the wording', () => {
    expect(formalStatement('inconsistent with limit carnot under assumptions cycle model = ideal', carnot)).toBe(
      'inconsistent with the Carnot limit under assumptions cycle model: ideal',
    );
  });

  it('shows engine values in the unit the claim was written in', () => {
    expect(toCanonical('%', 'dimensionless')).toBe(0.01);
    expect(toCanonical('nm', 'm')).toBe(1e-9);
    expect(toCanonical('kWh/kg', 'J/kg')).toBeNull();
    const c = claimOf();
    expect(inClaimUnit(0.5, c)).toBe('50.0 %');
    expect(inClaimUnit(2.5e-7, claimOf({ raw_unit: 'nm', unit: 'm' }))).toBe('250 nm');
  });

  it('lists the values used and puts them in the formula with the engine result', () => {
    const c = claimOf({
      parameters: {
        Th: { value: 600, raw_value: 600, unit: 'K', evidence: null, flags: [] },
        Tc: { value: 300, raw_value: 300, unit: 'K', evidence: null, flags: [] },
      },
    });
    expect(valuesUsed(c)).toBe('Th = 600 K, Tc = 300 K');
    const withFormula = { ...carnot, formula: 'efficiency <= 1 - Tc/Th' };
    expect(calculation(c, withFormula)).toBe('1 − 300 K/600 K = 50.0 %');
    // A parameter without value: no calculation is shown.
    expect(calculation(claimOf({ parameters: {} }), withFormula)).toBeNull();
  });

  it('shows the bound of the option the claim fails under, and "no limit" for a zero lower bound', () => {
    const r = (verdict: EvaluateResponse['verdict'], b: number, m: number | null): EvaluateResponse => ({
      ...claimOf().result!,
      verdict,
      bounds: { permissive: { b_min: b, b_max: b, margin: m === null ? null : { low: m, high: m } }, restrictive: { b_min: b, b_max: b, margin: null } },
    });
    const options = [
      { assumption: 'imaging method', option: 'super-resolution techniques', result: r('COMPATIBLE', 0, null) },
      { assumption: 'imaging method', option: 'conventional far-field imaging', result: r('IMPOSSIBLE', 2.5e-7, 0.6) },
    ];
    const focus = focusOption(options)!;
    expect(focus.option).toBe('conventional far-field imaging');
    const c = claimOf({ raw_unit: 'nm', unit: 'm' });
    expect(boundDisplay(c, focus.result, 'Min')).toBe('250 nm');
    expect(ratioDisplay(c, focus.result, 'Min')).toBe('0.600×');
    expect(boundDisplay(c, options[0].result, 'Min')).toBe('no limit');
  });

  it('shows one decisive bound out of a range, never "0 – x"', () => {
    const range = (verdict: EvaluateResponse['verdict'], lo: number, hi: number, low: number | null, high: number | null): EvaluateResponse => ({
      ...claimOf().result!,
      verdict,
      bounds: {
        permissive: { b_min: lo, b_max: hi, margin: low === null ? null : { low, high: high ?? low } },
        restrictive: { b_min: lo, b_max: hi, margin: null },
      },
    });
    const c = claimOf({ value: 1e-5, raw_value: 1e-5, raw_unit: 'rad', unit: 'rad' });
    // A minimum met by the strictest assumption: the strictest bound.
    const ok = range('COMPATIBLE', 5.2085e-6, 6.71e-6, 1.49, 1.92);
    expect(decisive(c, ok, 'Min')).toEqual({ bound: 6.71e-6, ratio: 1.49 });
    // A minimum broken under every assumption: the loosest bound, still broken.
    const bad = range('IMPOSSIBLE', 5.2085e-6, 6.71e-6, 0.745, 0.96);
    expect(decisive(c, bad, 'Min')).toEqual({ bound: 5.2085e-6, ratio: 0.96 });
    // A zero lower end means no limit: the other end, and the ratio from the values.
    const abbe = range('CONDITIONAL', 0, 2e-5, null, null);
    expect(decisive(c, abbe, 'Min')).toEqual({ bound: 2e-5, ratio: 0.5 });
    // A maximum: the strictest bound is the lower end.
    const p = claimOf({ value: 120, raw_value: 120, raw_unit: 'W', unit: 'W' });
    expect(boundDisplay(p, range('CONDITIONAL', 100, 1e32, 1.2e-30, 1.2), 'Max')).toBe('100 W');
  });

  it('reads the assumptions of a statement in words', () => {
    expect(readableAssumptions('inconsistent with the Mass-energy equivalence under assumptions none')).toBe('inconsistent with the Mass-energy equivalence');
    expect(readableAssumptions('inconsistent with X under assumptions imaging method = conventional imaging')).toBe('inconsistent with X under assumptions imaging method: conventional imaging');
    expect(readableAssumptions('under assumptions resolution criterion in {Rayleigh criterion, Sparrow criterion}')).toBe(
      'under assumptions resolution criterion: Rayleigh criterion or Sparrow criterion',
    );
  });

  it('writes numbers without reading noise and with their units in the calculation', () => {
    expect(plainNumber(1.0000000000000001e-24)).toBe('1e-24');
    expect(plainNumber(300)).toBe('300');
    const c = claimOf({
      parameters: { B: { value: 1, raw_value: 1, unit: 'MHz', evidence: null, flags: [] }, snr: { value: 15, raw_value: 15, unit: 'dimensionless', evidence: null, flags: [] } },
    });
    expect(valuesUsed(c, { ...carnot, parameter_labels: { B: 'bandwidth', snr: 'signal-to-noise ratio' } })).toBe('bandwidth B = 1 MHz, signal-to-noise ratio snr = 15');
  });

  it('links a reference to its DOI', () => {
    expect(doiUrl('Landauer, R., ..., doi:10.1147/rd.53.0183')).toBe('https://doi.org/10.1147/rd.53.0183');
    expect(doiUrl('Callen, H. B., Thermodynamics, Wiley (1985)')).toBeNull();
  });

  it('names the assumption to state with its options', () => {
    const missing = missingAssumptions(claimOf({ limit_id: 'betz', constraints: {} }), betz);
    expect(missing.map((m) => m.id)).toEqual(['reference_area']);
    expect(stateAssumptions(missing)).toMatch(/^State the .+: .+ or .+\.$/);
    // Carnot: a heat engine cycle must be stated, so it is a choice to make.
    expect(missingAssumptions(claimOf({ constraints: {} }), carnot).map((m) => m.id)).toEqual(['conversion_process']);
    expect(missingAssumptions(claimOf({ constraints: { conversion_process: 'heat_engine_cycle' } }), carnot)).toEqual([]);
  });
});
