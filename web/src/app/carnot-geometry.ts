/**
 * Energy flows of the Carnot apparatus, normalized to Qh = 1.
 *
 * This is a drawing aid only: the verdict always comes from the engine
 * through the API. The figure shows what the second law allows for the
 * temperatures on the bench, and what the claim would require.
 */
export interface CarnotFlows {
  /** Heat taken from the hot reservoir, the unit of the figure. */
  qh: number;
  /** Largest work allowed: 1 - Tc/Th. */
  boundWork: number;
  /** Smallest heat that must be rejected to the cold reservoir: Tc/Th. */
  minReject: number;
  /** Work the claim asks for, or null without a claim. */
  claimWork: number | null;
  /** Heat the claim leaves for the cold reservoir: 1 - claim. Can be below minReject. */
  claimReject: number | null;
  /** True when the claimed work is strictly above the allowed work. */
  exceedsBound: boolean;
}

/** Flows for valid temperatures `th > tc > 0`, or null when they are missing or invalid. */
export function carnotFlows(th: number | null, tc: number | null, claim: number | null): CarnotFlows | null {
  if (th === null || tc === null || !Number.isFinite(th) || !Number.isFinite(tc)) return null;
  if (!(tc > 0 && th > tc)) return null;
  const minReject = tc / th;
  const boundWork = 1 - minReject;
  const hasClaim = claim !== null && Number.isFinite(claim);
  return {
    qh: 1,
    boundWork,
    minReject,
    claimWork: hasClaim ? claim : null,
    claimReject: hasClaim ? 1 - claim : null,
    exceedsBound: hasClaim && claim > boundWork,
  };
}

/** Arrow widths of the figure, in viewBox units. */
export interface CarnotArrows {
  qh: number;
  work: number;
  reject: number;
  /** Width of the dashed claimed-work arrow; 0 without a claim or for a claim at or below 0. */
  claimWork: number;
  /** Width of the dashed claimed-reject arrow; 0 when the claim leaves nothing to reject. */
  claimReject: number;
}

/** Width drawn for Qh = 1. */
export const UNIT_WIDTH = 40;

/** Arrow widths proportional to the flows; the claimed work is capped at twice Qh so it stays in the frame. */
export function carnotArrows(flows: CarnotFlows, unit = UNIT_WIDTH): CarnotArrows {
  const clamp = (v: number | null, max: number) => (v === null ? 0 : Math.min(Math.max(v, 0), max));
  return {
    qh: flows.qh * unit,
    work: flows.boundWork * unit,
    reject: flows.minReject * unit,
    claimWork: clamp(flows.claimWork, 2) * unit,
    claimReject: clamp(flows.claimReject, 1) * unit,
  };
}
