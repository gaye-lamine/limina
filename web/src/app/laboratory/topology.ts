/**
 * Which 3D shape a law gets in the laboratory, from its parameters only
 * (spec laboratory-3d, R-05). No law is named here.
 */
import { LimitInfo } from '../api.models';

/** surface: `/v1/surface` over two parameters; corridor: `/v1/curve`; chamber: `/v1/evaluate`. */
export type Topology = 'surface' | 'corridor' | 'chamber';

export function topologyFor(limit: Pick<LimitInfo, 'parameters'>): Topology {
  const n = limit.parameters.length;
  if (n >= 2) return 'surface';
  return n === 1 ? 'corridor' : 'chamber';
}

/** Whether "Find Breaking Point" (`/v1/solve`) can answer for this law: a bound law with a parameter. */
export function canSolve(limit: Pick<LimitInfo, 'parameters' | 'direction'>): boolean {
  return limit.direction !== 'Equal' && limit.parameters.length > 0;
}
