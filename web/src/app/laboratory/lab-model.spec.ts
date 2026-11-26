import { AnalyzedClaim } from '../analysis.models';
import { EvaluateResponse, LimitInfo } from '../api.models';
import { LAB } from '../strings';
import { LIMITS } from '../testing-fixtures';
import { INSPECT_KEY, inspectOf, readInspect } from './inspect';
import {
  FALLBACK_RANGE,
  STEPS,
  applyInspect,
  baseUnit,
  choosers,
  claimSlider,
  decisiveBound,
  evaluateRequest,
  failureOf,
  fmt,
  fromPosition,
  parameterSlider,
  resultOf,
  toPosition,
  withRange,
  withValue,
} from './lab-model';
import { convert, displayUnits, hasOffset } from './lab-units';
import { MIN_ZONE, cellPast, coreRatio, heightRange, pastBound, scale, ticks, zoneSpans } from './view-state';

const byId = (id: string) => LIMITS.find((l) => l.id === id) as LimitInfo;

function response(verdict: EvaluateResponse['verdict'], b_min: number, b_max: number): EvaluateResponse {
  const e = { b_min, b_max, margin: null };
  return { verdict, limit_id: 'x', engine_version: '1', wording: 'as written by the engine', bounds: { permissive: e, restrictive: e }, configurations: [], missing_parameters: [] };
}

describe('laboratory: unit conversions', () => {
  it('converts temperatures with the 273.15 offset', () => {
    expect(convert(0, '°C', 'K')).toBeCloseTo(273.15, 10);
    expect(convert(300, 'K', '°C')).toBeCloseTo(26.85, 10);
    expect(hasOffset('°C')).toBe(true);
    expect(hasOffset('K')).toBe(false);
  });
  it('converts speeds, lengths and ratios by their factor', () => {
    expect(convert(1, 'km/s', 'm/s')).toBe(1000);
    expect(convert(500, 'nm', 'm')).toBeCloseTo(5e-7, 20);
    expect(convert(42, '%', 'dimensionless')).toBeCloseTo(0.42, 12);
    expect(convert(0.42, 'dimensionless', '%')).toBeCloseTo(42, 10);
  });
  it('refuses to convert across quantities and keeps unknown units as they are', () => {
    expect(convert(1, 'm', 'K')).toBeNull();
    expect(convert(1, 'furlong', 'm')).toBeNull();
    expect(displayUnits('furlong')).toEqual(['furlong']);
    expect(displayUnits('K')).toEqual(['K', '°C']);
    expect(displayUnits('m/s')[0]).toBe('m/s');
  });
  it('round-trips every unit of a family', () => {
    for (const base of ['K', 'm', 'Hz', 'J', 'W', 'dimensionless']) for (const u of displayUnits(base)) expect(convert(convert(7.5, base, u) as number, u, base)).toBeCloseTo(7.5, 9);
  });
});

describe('laboratory: controls from the metadata', () => {
  it('builds one slider per parameter in its SI unit, with the fallback range', () => {
    const carnot = byId('carnot');
    const sliders = carnot.parameters.map((p) => parameterSlider(carnot, p));
    expect(sliders.map((s) => s.id)).toEqual(['Th', 'Tc']);
    expect(sliders[0].base).toBe('K');
    expect(sliders[0].units).toEqual(['K', '°C']);
    expect([sliders[0].min, sliders[0].max]).toEqual(FALLBACK_RANGE);
    expect(baseUnit(['km/s', 'm/s'])).toBe('m/s');
    expect(baseUnit([])).toBe('dimensionless');
  });
  it('takes the engine range and keeps a value that fits', () => {
    const carnot = byId('carnot');
    const s = withRange(parameterSlider(carnot, carnot.parameters[0]), [400, 1000]);
    expect([s.min, s.max, s.value]).toEqual([400, 1000, 700]);
    expect(withRange(s, [500, 900]).value).toBe(700);
    expect(withRange(s, [800, 900]).value).toBe(850);
    expect(withRange(s, [9, 1]).min).toBe(400);
  });
  it('maps track positions both ways, linear and log', () => {
    const lin = { ...parameterSlider(byId('carnot'), byId('carnot').parameters[0]), min: 0, max: 10, log: false, value: 2.5 };
    expect(toPosition(lin)).toBe(STEPS / 4);
    expect(fromPosition(lin, STEPS / 2)).toBe(5);
    const log = claimSlider(byId('carnot'), 0.5);
    expect(log.log).toBe(true);
    expect(fromPosition(log, 0)).toBeCloseTo(0.005, 12);
    expect(fromPosition(log, STEPS)).toBeCloseTo(1.5, 12);
    expect(fromPosition(log, toPosition(log))).toBeCloseTo(log.value, 2);
  });
  it('widens a range to hold a typed value', () => {
    const s = withValue(claimSlider(byId('carnot'), 0.5), 10);
    expect(s.value).toBe(10);
    expect(s.max).toBe(10);
  });
  it('starts the claim near the bound, on its allowed side', () => {
    expect(claimSlider(byId('carnot'), 0.5).value).toBe(0.4);
    expect(claimSlider({ ...byId('carnot'), direction: 'Min' }, 2).value).toBe(2.5);
    expect(claimSlider(byId('carnot'), null).value).toBe(0.8);
    expect(claimSlider(byId('carnot'), 0.5, 0.9).value).toBe(0.9);
  });
});

describe('laboratory: assumptions chosen by default', () => {
  it('pre-selects a heat engine cycle for Carnot, an unshrouded rotor for Betz', () => {
    const carnot = choosers(byId('carnot'));
    expect(carnot.map((c) => [c.id, c.value])).toEqual([['conversion_process', 'heat_engine_cycle']]);
    expect(choosers(byId('betz')).map((c) => c.value)).toEqual(['unshrouded_rotor']);
  });
  it('sends only stated assumptions, in core units', () => {
    const carnot = byId('carnot');
    const params = carnot.parameters.map((p) => parameterSlider(carnot, p));
    const cs = choosers(carnot);
    const req = evaluateRequest(carnot, claimSlider(carnot, 0.5), params, cs);
    expect(req.constraints).toEqual({ conversion_process: ['heat_engine_cycle'] });
    expect(req.parameters['Th'].unit).toBe('K');
    expect(req.claim.unit).toBe('dimensionless');
    const none = evaluateRequest(carnot, claimSlider(carnot, 0.5), params, cs.map((c) => ({ ...c, value: '' })));
    expect(none.constraints).toEqual({});
  });
});

describe('laboratory: reading the engine answer', () => {
  it('keeps the engine wording and verdict label, and the decisive bound', () => {
    const r = resultOf(response('IMPOSSIBLE', 0.5, 0.6), 0.9, 'Max');
    expect(r.kind).toBe('impossible');
    expect(r.label).toBe('IMPOSSIBLE');
    expect(r.wording).toBe('as written by the engine');
    expect(r.bound).toBe(0.6);
    expect(r.ratio).toBeCloseTo(1.5, 12);
    expect(decisiveBound(response('COMPATIBLE', 0.5, 0.6), 'Max')).toBe(0.5);
    expect(decisiveBound(response('COMPATIBLE', 2, 2), 'Min')).toBe(2);
    expect(decisiveBound(response('CONDITIONAL', 0, 5), 'Min')).toBe(5);
  });
  it('says plainly when there is no verdict', () => {
    expect(failureOf({ status: 422, code: 'OutsideModelDomain', message: 'x' }).label).toBe(LAB.outside);
    expect(failureOf({ status: 422, code: 'NotApplicable', message: 'x' }).wording).toBe(LAB.notApplicable);
    expect(failureOf({ status: 0, code: 'NetworkError', message: 'x' }).wording).toBe(LAB.noAnswer);
  });
  it('formats numbers for the panel', () => {
    expect(fmt(0.5)).toBe('0.5');
    expect(fmt(123456)).toBe('1.23e+5');
    expect(fmt(Number.NaN)).toBe('—');
  });
});

describe('laboratory: Inspect in 3D Lab', () => {
  const claim = {
    limit_id: 'carnot',
    value: 0.7,
    unit: 'dimensionless',
    constraints: { conversion_process: 'not_a_heat_engine_cycle' },
    parameters: {
      Th: { value: 600, raw_value: 600, unit: 'K', evidence: null, flags: [] },
      Tc: { value: null, raw_value: null, unit: 'K', evidence: null, flags: [] },
    },
  } as unknown as AnalyzedClaim;

  it('hands over the law, the values and the assumptions', () => {
    const i = inspectOf(claim);
    expect(i).toEqual({ limit_id: 'carnot', claim: { value: 0.7, unit: 'dimensionless' }, parameters: { Th: { value: 600, unit: 'K' } }, constraints: { conversion_process: 'not_a_heat_engine_cycle' } });
    expect(readInspect({ [INSPECT_KEY]: i })).toEqual(i);
    expect(readInspect(null)).toBeNull();
    expect(readInspect({ [INSPECT_KEY]: { limit_id: 3 } })).toBeNull();
    expect(inspectOf({ ...claim, limit_id: null })).toBeNull();
  });
  it('fills the sliders and choosers, widening a range when needed', () => {
    const carnot = byId('carnot');
    const params = carnot.parameters.map((p) => parameterSlider(carnot, p));
    const i = inspectOf({ ...claim, parameters: { Th: { value: 5000, raw_value: 5000, unit: 'K', evidence: null, flags: [] } } } as unknown as AnalyzedClaim);
    const r = applyInspect(i!, params, choosers(carnot));
    expect(r.params[0].value).toBe(5000);
    expect(r.params[0].max).toBe(5000);
    expect(r.params[1]).toEqual(params[1]);
    expect(r.choosers[0].value).toBe('not_a_heat_engine_cycle');
    const unknown = applyInspect({ ...i!, constraints: { conversion_process: 'nope' } }, params, choosers(carnot));
    expect(unknown.choosers[0].value).toBe('heat_engine_cycle');
  });
});

describe('laboratory: 3D view states', () => {
  it('places values on a linear or log scale, clamped near the ends', () => {
    const lin = scale(0, 10, -8, 8);
    expect(lin.to(5)).toBe(0);
    expect(lin.to(100)).toBeCloseTo(8 + 0.2 * 16, 10);
    const log = scale(1, 1e4, 0, 8);
    expect(log.log).toBe(true);
    expect(log.to(100)).toBeCloseTo(4, 10);
  });
  it('holds the claim and the bound in the height range', () => {
    const [lo, hi] = heightRange([0.2, 0.4, null], 0.9, 0.5);
    expect(lo).toBeLessThanOrEqual(0.2);
    expect(hi).toBe(0.9);
    const [a, b] = heightRange([], 3, null);
    expect(a).toBeCloseTo(1.2, 10);
    expect(b).toBe(4.5);
  });
  it('marks the side of a drawn bound per direction', () => {
    expect(pastBound(0.9, 0.5, 'Max')).toBe(true);
    expect(pastBound(0.4, 0.5, 'Max')).toBe(false);
    expect(pastBound(0.4, 0.5, 'Min')).toBe(true);
    expect(pastBound(0.4, 0.5, 'Equal')).toBe(false);
  });
  it('places the validity zones of a corridor, keeping a single point visible', () => {
    const xs = scale(400, 1000, -8, 8, false);
    expect(zoneSpans([[400, 550]], xs)).toEqual([[-8, -4]]);
    const [[a, b]] = zoneSpans([[400, 400]], xs);
    expect(b - a).toBeCloseTo(MIN_ZONE, 10);
    expect(a).toBeCloseTo(-8, 10);
    expect(zoneSpans([[0, 100]], xs)).toEqual([]);
    expect(zoneSpans(undefined, xs)).toEqual([]);
    expect(zoneSpans([[900, 2000]], xs)[0][1]).toBe(8);
  });
  it('hatches a surface cell only when all its corners are defined and passed', () => {
    expect(cellPast([0.5, 0.5, 0.6, 0.6], 0.9, 'Max')).toBe(true);
    expect(cellPast([0.5, 0.5, 0.6, null], 0.9, 'Max')).toBe(false);
    expect(cellPast([0.5, 0.5, 0.6, 0.6], 0.2, 'Max')).toBe(false);
    expect(cellPast([0.5, 0.5, 0.6, 0.6], 0.2, 'Min')).toBe(true);
  });
  it('spaces the axis ticks evenly', () => {
    expect(ticks(-8, 8)).toEqual([-8, -4, 0, 4, 8]);
    expect(ticks(0, 1, 1)).toEqual([0]);
  });
  it('sizes the chamber core from the claim over the bound', () => {
    expect(coreRatio(0.5, 1)).toBe(0.5);
    expect(coreRatio(10, 1)).toBe(2.5);
    expect(coreRatio(1, null)).toBe(0.5);
    expect(coreRatio(0, 1)).toBe(0.05);
  });
});
