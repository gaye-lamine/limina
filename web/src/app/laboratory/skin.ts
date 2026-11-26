/**
 * The physical skin of a law in the laboratory, chosen from its declared
 * dimensions only (spec laboratory-3d, R-06): the claim unit and the
 * parameters' kinds and units. The skin changes materials and labels,
 * never a number. No law is named here.
 */
import { LimitInfo } from '../api.models';

export type Skin = 'thermal' | 'gravitational' | 'wave' | 'energy' | 'neutral';

/** Unit symbols of each family, as `GET /v1/limits` declares them (exact symbols). */
const FAMILIES: Record<Exclude<Skin, 'thermal' | 'neutral'>, readonly string[]> = {
  gravitational: ['kg', 'g', 't', 'm^3/s^2', 'm/s', 'km/s', 'au'],
  wave: ['nm', 'um', 'µm', 'pm', 'Hz', 'kHz', 'MHz', 'GHz', 'THz', 'PHz', 'rad', 'lm/W', 'bit/s'],
  energy: ['J', 'eV', 'Wh', 'kWh', 'W', 'mW', 'kW', 'MW', 'GW', 'W/m^2'],
};

/** Order used to break a tie: the most specific first. */
const ORDER: Skin[] = ['wave', 'gravitational', 'energy'];

function familyOf(unit: string): Skin | null {
  for (const s of ORDER) if ((FAMILIES[s as keyof typeof FAMILIES] as readonly string[]).includes(unit)) return s;
  return null;
}

/**
 * Thermal when a parameter is a temperature. Otherwise the family with the
 * highest score: the claim unit counts 2, each parameter 1 (by its first
 * declared unit that belongs to a family); neutral when nothing matches.
 */
export function skinFor(limit: Pick<LimitInfo, 'claim_unit' | 'parameters'>): Skin {
  if (limit.parameters.some((p) => p.kind === 'AbsoluteTemperature' || p.kind === 'TemperatureDifference')) return 'thermal';
  const score = new Map<Skin, number>();
  const add = (s: Skin | null, w: number) => {
    if (s) score.set(s, (score.get(s) ?? 0) + w);
  };
  add(familyOf(limit.claim_unit), 2);
  for (const p of limit.parameters) add(p.units.map(familyOf).find((s) => s !== null) ?? null, 1);
  let best: Skin = 'neutral';
  let top = 0;
  for (const s of ORDER) {
    const v = score.get(s) ?? 0;
    if (v > top) {
      best = s;
      top = v;
    }
  }
  return best;
}
