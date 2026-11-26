import { EvaluateResponse } from './api.models';
import {
  boundDisplay,
  calculation,
  doiUrl,
  focusOption,
  formalStatement,
  inClaimUnit,
  missingAssumptions,
  prettyFormula,
  ratioDisplay,
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
      'inconsistent with the Carnot limit under assumptions cycle model = ideal',
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
    expect(calculation(c, withFormula)).toBe('1 − 300/600 = 50.0 %');
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
    expect(ratioDisplay(focus.result)).toBe('0.600×');
    expect(boundDisplay(c, options[0].result, 'Min')).toBe('no limit');
  });

  it('links a reference to its DOI', () => {
    expect(doiUrl('Landauer, R., ..., doi:10.1147/rd.53.0183')).toBe('https://doi.org/10.1147/rd.53.0183');
    expect(doiUrl('Callen, H. B., Thermodynamics, Wiley (1985)')).toBeNull();
  });

  it('names the assumption to state with its options', () => {
    const missing = missingAssumptions(claimOf({ limit_id: 'betz', constraints: {} }), betz);
    expect(missing.map((m) => m.id)).toEqual(['reference_area']);
    expect(stateAssumptions(missing)).toMatch(/^State the .+: .+ or .+\.$/);
    expect(missingAssumptions(claimOf({ constraints: {} }), carnot)).toEqual([]);
  });
});
