/**
 * A light unit converter for the laboratory (spec laboratory-3d, D-01).
 * Values are kept in the base unit the law declares (its first unit, or
 * its claim unit, which are SI); people may read and type them in a
 * friendlier unit of the same quantity (°C for K, km/s for m/s, nm for m,
 * % for a ratio). Requests to the engine always carry the base unit, which
 * the core parses. Display only: no physics here.
 */

/** A unit: its factor to the family's SI unit, and an offset for °C. */
interface UnitDef {
  factor: number;
  offset?: number;
}

/** Families of units the laboratory can show, keyed by their SI unit. */
const FAMILIES: Record<string, Record<string, UnitDef>> = {
  K: { K: { factor: 1 }, '°C': { factor: 1, offset: 273.15 } },
  dimensionless: { dimensionless: { factor: 1 }, '%': { factor: 0.01 } },
  m: {
    m: { factor: 1 },
    km: { factor: 1e3 },
    cm: { factor: 1e-2 },
    mm: { factor: 1e-3 },
    um: { factor: 1e-6 },
    nm: { factor: 1e-9 },
    pm: { factor: 1e-12 },
    au: { factor: 149_597_870_700 },
  },
  s: { s: { factor: 1 }, ms: { factor: 1e-3 }, us: { factor: 1e-6 }, µs: { factor: 1e-6 }, ns: { factor: 1e-9 }, min: { factor: 60 }, h: { factor: 3600 }, d: { factor: 86_400 } },
  kg: { kg: { factor: 1 }, g: { factor: 1e-3 }, mg: { factor: 1e-6 }, t: { factor: 1e3 } },
  'm/s': { 'm/s': { factor: 1 }, 'km/s': { factor: 1e3 } },
  Hz: { Hz: { factor: 1 }, kHz: { factor: 1e3 }, MHz: { factor: 1e6 }, GHz: { factor: 1e9 }, THz: { factor: 1e12 }, PHz: { factor: 1e15 } },
  J: { J: { factor: 1 }, kJ: { factor: 1e3 }, MJ: { factor: 1e6 }, eV: { factor: 1.602_176_634e-19 }, kWh: { factor: 3.6e6 } },
  W: { W: { factor: 1 }, mW: { factor: 1e-3 }, kW: { factor: 1e3 }, MW: { factor: 1e6 }, GW: { factor: 1e9 } },
  C: { C: { factor: 1 }, mC: { factor: 1e-3 }, uC: { factor: 1e-6 }, nC: { factor: 1e-9 } },
  T: { T: { factor: 1 }, mT: { factor: 1e-3 } },
  'V/m': { 'V/m': { factor: 1 }, 'kV/m': { factor: 1e3 } },
  'm^2': { 'm^2': { factor: 1 }, 'cm^2': { factor: 1e-4 } },
  'kg/m^3': { 'kg/m^3': { factor: 1 }, 'g/cm^3': { factor: 1e3 } },
  'bit/s': { 'bit/s': { factor: 1 }, 'kbit/s': { factor: 1e3 }, 'Mbit/s': { factor: 1e6 }, 'Gbit/s': { factor: 1e9 } },
};

/** The family holding `unit`, and the unit's definition in it. */
function lookup(unit: string): { family: Record<string, UnitDef>; def: UnitDef } | null {
  for (const family of Object.values(FAMILIES)) {
    const def = family[unit];
    if (def) return { family, def };
  }
  return null;
}

/** The units a value in `base` can be shown in: the base first, then its family; just the base when unknown. */
export function displayUnits(base: string): string[] {
  const hit = lookup(base);
  if (!hit) return [base];
  return [base, ...Object.keys(hit.family).filter((u) => u !== base)];
}

/** `value` in `from` expressed in `to` (same family); null when the units are not convertible. */
export function convert(value: number, from: string, to: string): number | null {
  if (from === to) return value;
  const a = lookup(from);
  const b = lookup(to);
  if (!a || !b || a.family !== b.family) return null;
  const si = value * a.def.factor + (a.def.offset ?? 0);
  return (si - (b.def.offset ?? 0)) / b.def.factor;
}

/** Whether a quantity in this unit is an absolute temperature with an offset unit (°C), so a negative display value is legitimate. */
export function hasOffset(unit: string): boolean {
  return lookup(unit)?.def.offset !== undefined;
}
