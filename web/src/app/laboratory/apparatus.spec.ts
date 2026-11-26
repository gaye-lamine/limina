import { LimitInfo, ParameterSpec } from '../api.models';
import { LIMITS } from '../testing-fixtures';
import {
  agitation,
  apertureParameter,
  airy,
  apparatusFor,
  besselJ1,
  beta,
  boxHalfWidth,
  contraction,
  converterSpeed,
  decayFractions,
  discreteParameter,
  flowShares,
  initialParameter,
  isDecay,
  isLength,
  levelOf,
  massParameter,
  orbitRadii,
  peakSeparation,
  shellRadii,
  spectralParameter,
  spectrumRgb,
  toNanometres,
  trajectory,
  undecayed,
  wellDepth,
} from './apparatus';

/** A law from metadata only: no id the routing could know. */
function law(claim_unit: string, params: (Partial<ParameterSpec> & { label?: string })[], status?: string): LimitInfo {
  const parameters = params.map((p, i) => ({ id: p.id ?? `p${i}`, kind: p.kind ?? 'Plain', required: true, units: p.units ?? [] }));
  return {
    ...LIMITS[0],
    id: 'never_seen_before',
    status,
    claim_unit,
    parameters,
    parameter_labels: Object.fromEntries(params.map((p, i) => [parameters[i].id, p.label ?? ''])),
  } as LimitInfo;
}

describe('laboratory: the physical apparatus from the metadata only', () => {
  it('routes energy, heat and efficiency laws to the energy bench', () => {
    expect(apparatusFor(LIMITS.find((l) => l.id === 'carnot') as LimitInfo)).toBe('flow');
    expect(apparatusFor(LIMITS.find((l) => l.id === 'betz') as LimitInfo)).toBe('flow');
    expect(apparatusFor(law('W/m^2', [{ kind: 'AbsoluteTemperature', units: ['K'], label: 'temperature' }]))).toBe('flow');
    expect(apparatusFor(law('W', [{ units: ['W', 'kW'], label: 'input power' }]))).toBe('flow');
  });
  it('routes speed, mass and force laws to the field bench', () => {
    expect(apparatusFor(law('m/s', [{ units: ['m^3/s^2'], label: 'gravitational parameter' }, { units: ['m', 'km'], label: 'distance' }]))).toBe('trajectory');
    expect(apparatusFor(law('N', [{ units: ['C'], label: 'charge' }, { units: ['T'], label: 'magnetic field' }]))).toBe('trajectory');
    // Energy and mass tie: the field bench is the more specific.
    expect(apparatusFor(law('J', [{ units: ['kg'], label: 'mass' }, { units: ['m/s'], label: 'speed' }]))).toBe('trajectory');
  });
  it('routes wavelength, frequency, bandwidth and resolution laws to the optics bench', () => {
    expect(apparatusFor(law('m', [{ id: 'wavelength', units: ['m', 'nm'], label: 'wavelength' }, { units: ['dimensionless'], label: 'numerical aperture' }]))).toBe('wave');
    expect(apparatusFor(law('rad', [{ units: ['m'], label: 'wavelength' }, { units: ['m'], label: 'aperture diameter' }]))).toBe('wave');
    expect(apparatusFor(law('bit/s', [{ units: ['Hz'], label: 'bandwidth' }, { units: ['dimensionless'], label: 'signal-to-noise ratio' }]))).toBe('wave');
    expect(apparatusFor(law('J', [{ units: ['Hz', 'kHz'], label: 'frequency' }]))).toBe('wave');
  });
  it('routes discrete levels, momentum uncertainty and decay to the quantum bench, and is its fallback', () => {
    expect(apparatusFor(law('J', [{ id: 'n', units: ['dimensionless'], label: 'level number' }]))).toBe('quantum');
    expect(apparatusFor(law('m', [{ units: ['kg*m/s'], label: 'momentum uncertainty' }]))).toBe('quantum');
    expect(apparatusFor(law('dimensionless', [{ units: ['dimensionless'], label: 'initial number' }, { units: ['s'], label: 'half-life' }]))).toBe('quantum');
    expect(apparatusFor(law('furlong', [{ units: ['parsec^7'], label: 'something new' }]))).toBe('quantum');
  });
  it('gives a law written by AI, never seen before, a bench from its dimensions', () => {
    const draft = law('W', [{ kind: 'AbsoluteTemperature', units: ['K'], label: 'wall temperature' }, { units: ['m^2'], label: 'area' }], 'draft');
    expect(draft.status).toBe('draft');
    expect(apparatusFor(draft)).toBe('flow');
    expect(apparatusFor(law('Hz', [{ units: ['m'], label: 'cavity length' }], 'draft'))).toBe('wave');
  });
  it('finds the level, mass and spectral parameters by their metadata', () => {
    expect(discreteParameter(law('J', [{ id: 'n', units: ['dimensionless'], label: 'level number' }]))).toBe('n');
    expect(discreteParameter(law('V', [{ id: 'n_turns', units: ['dimensionless'], label: 'number of turns' }]))).toBeNull();
    expect(massParameter(law('m', [{ id: 'M', units: ['kg', 'g'], label: 'mass of the body' }]))).toBe('M');
    expect(spectralParameter(law('m', [{ id: 'wavelength', units: ['m', 'nm'], label: 'wavelength' }]))).toEqual({ id: 'wavelength', as: 'wavelength' });
    expect(spectralParameter(law('J', [{ id: 'f', units: ['Hz'], label: 'frequency' }]))).toEqual({ id: 'f', as: 'frequency' });
    expect(apertureParameter(law('rad', [{ id: 'wavelength', units: ['m'], label: 'wavelength' }, { id: 'D', units: ['m'], label: 'aperture diameter' }]))).toBe('D');
  });
});

describe('laboratory: decaying populations and claimed lengths', () => {
  const decay = law('dimensionless', [
    { id: 'n0', units: ['dimensionless'], label: 'initial number' },
    { id: 't', units: ['s', 'min'], label: 'elapsed time' },
    { id: 't_half', units: ['s'], label: 'half-life' },
  ]);
  it('recognises a decay from a time parameter or its labels, not an uncertainty', () => {
    expect(isDecay(decay)).toBe(true);
    expect(isDecay(law('dimensionless', [{ units: ['dimensionless'], label: 'activity after decay' }]))).toBe(true);
    expect(isDecay(law('m', [{ units: ['kg*m/s'], label: 'momentum uncertainty' }]))).toBe(false);
    expect(initialParameter(decay)).toBe('n0');
  });
  it('draws the claimed remaining and the expected fractions of the initial count', () => {
    expect(decayFractions(250, 500, 1000)).toEqual({ shown: 0.25, expected: 0.5 });
    expect(decayFractions(2000, null, 1000)).toEqual({ shown: 1, expected: null });
    expect(decayFractions(0.3, 0.4, null)).toEqual({ shown: 0.3, expected: 0.4 });
    const lit = undecayed(343, 0.25);
    expect(lit.filter(Boolean).length).toBe(86);
    // A larger fraction keeps every nucleus a smaller one kept lit.
    const more = undecayed(343, 0.5);
    expect(lit.every((on, i) => !on || more[i])).toBe(true);
  });
  it('rings a claimed length around the reference sphere', () => {
    expect(isLength('m')).toBe(true);
    expect(isLength('m/s')).toBe(false);
    expect(orbitRadii(2, 1)).toEqual({ measured: 5, reference: 2.5 });
    expect(orbitRadii(100, 1).measured).toBe(7.5);
    expect(orbitRadii(1, null).measured).toBe(2.5);
  });
});

describe('laboratory: the quantities the benches draw', () => {
  it('collapses the rejected flow past the limit', () => {
    expect(flowShares(0.5, false).rejected).toBeGreaterThan(0);
    expect(flowShares(0.5, false).work + flowShares(0.5, false).rejected).toBeCloseTo(1, 12);
    expect(flowShares(3, true)).toEqual({ input: 1, work: 1, rejected: 0 });
    expect(converterSpeed(10)).toBe(3);
    expect(converterSpeed(null)).toBe(0.4);
  });
  it('bends the well with the strength and squashes fast probes', () => {
    expect(wellDepth(1)).toBeGreaterThan(wellDepth(0));
    expect(beta(1.5e8, 'm/s')).toBeCloseTo(0.5, 2);
    expect(beta(1e9, 'm/s')).toBe(0.995);
    expect(beta(1, 'J')).toBe(0);
    expect(contraction(0)).toBe(1);
    expect(contraction(0.995)).toBeCloseTo(0.1, 1);
  });
  it('draws an escaping path outward and a falling one inward', () => {
    const r = (p: [number, number]) => Math.hypot(p[0], p[1]);
    const out = trajectory(true);
    const fall = trajectory(false);
    expect(r(out[out.length - 1])).toBeGreaterThan(r(out[0]));
    expect(r(fall[fall.length - 1])).toBeLessThan(r(fall[0]));
  });
  it('colours the beam by the real spectrum', () => {
    const red = spectrumRgb(700) as number[];
    const blue = spectrumRgb(460) as number[];
    const green = spectrumRgb(530) as number[];
    expect(red[0]).toBeGreaterThan(red[2]);
    expect(blue[2]).toBeGreaterThan(blue[0]);
    expect(green[1]).toBe(1);
    expect(spectrumRgb(null)).toBeNull();
    expect(toNanometres(5e-7, 'wavelength')).toBeCloseTo(500, 9);
    expect(toNanometres(5.99584916e14, 'frequency')).toBeCloseTo(500, 3);
  });
  it('computes the Airy pattern: peak 1, first dark ring at its width', () => {
    expect(besselJ1(0)).toBeCloseTo(0, 6);
    expect(besselJ1(1.8412)).toBeCloseTo(0.5819, 3);
    expect(airy(0, 1)).toBe(1);
    expect(airy(1, 1)).toBeLessThan(1e-4);
  });
  it('merges the peaks past the limit and separates them within it, on the engine side', () => {
    expect(peakSeparation(0.5, 'Min', true, false)).toBeLessThan(1);
    expect(peakSeparation(2, 'Min', false, true)).toBeGreaterThanOrEqual(1);
    expect(peakSeparation(0.9, 'Min', false, true)).toBeGreaterThanOrEqual(1.1);
    expect(peakSeparation(2, 'Max', true, false)).toBeLessThan(1);
  });
  it('spaces the levels as n squared and tightens the box into agitation', () => {
    const r = shellRadii(4, 16);
    expect(r).toEqual([1, 4, 9, 16]);
    expect(levelOf(2.6)).toBe(3);
    expect(levelOf(null)).toBe(1);
    expect(agitation(boxHalfWidth(0))).toBeGreaterThan(agitation(boxHalfWidth(1)));
  });
});
