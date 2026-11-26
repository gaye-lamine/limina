import { LimitInfo, ParameterSpec } from '../api.models';
import { LIMITS } from '../testing-fixtures';
import { skinFor } from './skin';
import { canSolve, topologyFor } from './topology';

function law(claim_unit: string, parameters: Partial<ParameterSpec>[], direction: LimitInfo['direction'] = 'Max'): LimitInfo {
  return {
    ...LIMITS[0],
    id: 'x',
    direction,
    claim_unit,
    parameters: parameters.map((p, i) => ({ id: `p${i}`, kind: 'Plain', required: true, units: [], ...p })),
  };
}

describe('laboratory: topology and skin from the metadata only', () => {
  it('chooses the shape from the number of parameters', () => {
    expect(topologyFor(law('m', [{ units: ['m'] }, { units: ['kg'] }]))).toBe('surface');
    expect(topologyFor(law('m', [{ units: ['m'] }, { units: ['kg'] }, { units: ['s'] }]))).toBe('surface');
    expect(topologyFor(law('J', [{ units: ['K'] }]))).toBe('corridor');
    expect(topologyFor(law('dimensionless', []))).toBe('chamber');
    // The fixtures: Betz has no parameter, Carnot and COP two temperatures.
    const byId = (id: string) => LIMITS.find((l) => l.id === id)!;
    expect(topologyFor(byId('betz'))).toBe('chamber');
    expect(topologyFor(byId('carnot'))).toBe('surface');
  });

  it('offers Find Breaking Point only where the solver can answer', () => {
    expect(canSolve(law('m', [{ units: ['m'] }], 'Min'))).toBe(true);
    expect(canSolve(law('m', [{ units: ['m'] }], 'Equal'))).toBe(false);
    expect(canSolve(law('dimensionless', [], 'Max'))).toBe(false);
  });

  it('chooses the skin from the declared dimensions', () => {
    expect(skinFor(LIMITS.find((l) => l.id === 'carnot')!)).toBe('thermal');
    expect(skinFor(law('m/s', [{ units: ['m^3/s^2'] }, { units: ['m', 'km'] }]))).toBe('gravitational');
    expect(skinFor(law('m', [{ units: ['kg', 'g'] }]))).toBe('gravitational');
    expect(skinFor(law('rad', [{ units: ['m', 'um', 'nm'] }, { units: ['m', 'cm'] }]))).toBe('wave');
    expect(skinFor(law('bit/s', [{ units: ['Hz', 'MHz'] }, { units: ['dimensionless'] }]))).toBe('wave');
    // An energy claim from a mass: the claim counts twice.
    expect(skinFor(law('J', [{ units: ['kg', 'g'] }]))).toBe('energy');
    expect(skinFor(law('W', [{ units: ['W', 'kW'] }]))).toBe('energy');
    expect(skinFor(law('dimensionless', []))).toBe('neutral');
    expect(skinFor(law('m', [{ units: ['kg*m/s'] }]))).toBe('neutral');
  });
});
