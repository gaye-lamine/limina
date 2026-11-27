/**
 * "Inspect in 3D Lab" (spec laboratory-3d, R-09): the values of an analysed
 * claim handed to the laboratory through the router's navigation state.
 * Kept apart from the laboratory so the analysis screen loads none of it.
 */
import { AnalyzedClaim } from '../analysis.models';
import { ParameterValue } from '../api.models';

/** Key of the analysed claim in the navigation state. */
export const INSPECT_KEY = 'liminaInspect';

/** Values brought from an analysed claim. */
export interface Inspect {
  limit_id: string;
  claim: ParameterValue;
  parameters: Record<string, ParameterValue>;
  constraints: Record<string, string>;
}

/** The values of an analysed claim the laboratory can take; null without a law or a value. */
export function inspectOf(c: AnalyzedClaim): Inspect | null {
  if (!c.limit_id || c.value === null) return null;
  const parameters: Record<string, ParameterValue> = {};
  for (const [id, p] of Object.entries(c.parameters)) if (p.value !== null) parameters[id] = { value: p.value, unit: p.unit };
  return { limit_id: c.limit_id, claim: { value: c.value, unit: c.unit }, parameters, constraints: { ...c.constraints } };
}

/** Key of the claims sent to the System stress test in the navigation state. */
export const CASCADE_KEY = 'liminaCascade';

/** The analysed claims of a text that can be stress-tested together: a law and a value each, one per law, four at most. */
export function cascadeOf(claims: AnalyzedClaim[]): Inspect[] {
  const out: Inspect[] = [];
  for (const c of claims) {
    const i = inspectOf(c);
    if (i && !out.some((x) => x.limit_id === i.limit_id)) out.push(i);
    if (out.length === 4) break;
  }
  return out;
}

/** The claims of a stress test in a navigation state, each with the expected shape. */
export function readCascade(state: unknown): Inspect[] {
  const list = (state as Record<string, unknown> | null)?.[CASCADE_KEY];
  if (!Array.isArray(list)) return [];
  return list.map((x) => readInspect({ [INSPECT_KEY]: x })).filter((x): x is Inspect => x !== null).slice(0, 4);
}

/** The analysed claim in a navigation state, when it has the expected shape. */
export function readInspect(state: unknown): Inspect | null {
  const s = (state as Record<string, unknown> | null)?.[INSPECT_KEY] as Inspect | undefined;
  return s && typeof s.limit_id === 'string' && s.claim && typeof s.claim.value === 'number' && typeof s.claim.unit === 'string'
    ? { limit_id: s.limit_id, claim: s.claim, parameters: s.parameters ?? {}, constraints: s.constraints ?? {} }
    : null;
}
