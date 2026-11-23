/** The two modes of a heat pump or refrigerator. */
export type CopMode = 'heating' | 'cooling';

/**
 * Energy flows of the COP apparatus, normalized to W = 1, with the balance
 * Qh = Qc + W.
 *
 * This is a drawing aid only: the verdict always comes from the engine
 * through the API.
 */
export interface CopFlows {
  /** Work put into the machine, the unit of the figure. */
  w: number;
  /** Carnot bound on the COP for this mode: Th/(Th-Tc) heating, Tc/(Th-Tc) cooling. */
  bound: number;
  /** Largest useful heat allowed per unit of work: Qh when heating, Qc when cooling. */
  usefulAllowed: number;
  /** Useful heat the claim asks for (claim x W), or null without a claim. */
  usefulClaimed: number | null;
  /** The other heat flow at the bound: Qc when heating, Qh when cooling. */
  otherHeatAllowed: number;
  /** True when the claim is strictly above the bound. */
  exceedsBound: boolean;
}

/** Flows for valid temperatures `th > tc > 0`, or null when they are missing or invalid. */
export function copFlows(
  th: number | null,
  tc: number | null,
  claim: number | null,
  mode: CopMode,
): CopFlows | null {
  if (th === null || tc === null || !Number.isFinite(th) || !Number.isFinite(tc)) return null;
  if (!(tc > 0 && th > tc)) return null;
  const w = 1;
  const bound = mode === 'heating' ? th / (th - tc) : tc / (th - tc);
  const hasClaim = claim !== null && Number.isFinite(claim);
  return {
    w,
    bound,
    usefulAllowed: bound * w,
    usefulClaimed: hasClaim ? claim * w : null,
    otherHeatAllowed: mode === 'heating' ? bound * w - w : bound * w + w,
    exceedsBound: hasClaim && claim > bound,
  };
}

/** Arrow widths of the COP figure, in viewBox units. */
export interface CopArrows {
  w: number;
  useful: number;
  other: number;
  /** Width of the dashed claimed arrow; 0 without a claim. */
  claimed: number;
}

/** Arrow width limits: never thinner than 2 units, never wider than `MAX_ARROW`. */
export const MIN_ARROW = 2;
export const MAX_ARROW = 44;

/**
 * Widths proportional to energy: the largest flow drawn gets MAX_ARROW and
 * the others scale with it, each kept at MIN_ARROW or more.
 */
export function copArrows(flows: CopFlows): CopArrows {
  const claimed = flows.usefulClaimed === null ? 0 : Math.max(flows.usefulClaimed, 0);
  const largest = Math.max(flows.w, flows.usefulAllowed, flows.otherHeatAllowed, claimed);
  const unit = MAX_ARROW / largest;
  const width = (v: number) => Math.min(Math.max(v * unit, MIN_ARROW), MAX_ARROW);
  return {
    w: width(flows.w),
    useful: width(flows.usefulAllowed),
    other: width(flows.otherHeatAllowed),
    claimed: flows.usefulClaimed === null ? 0 : width(claimed),
  };
}
