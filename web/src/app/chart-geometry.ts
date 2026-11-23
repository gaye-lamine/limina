import { Bounds, Margin } from './api.models';
import { STRINGS } from './strings';

/** Input of the bound-scale chart, in the chart's own coordinates. */
export interface BoundScaleInput {
  claim: number;
  /** Smallest bound. Equal to `boundMax` when the bound is a single value. */
  boundMin: number;
  /** Largest bound: the hatch starts here. */
  boundMax: number;
  plotLeft: number;
  plotWidth: number;
}

/** Positions of the chart elements. */
export interface BoundScale {
  /** Value at the right end of the axis: 1.25 times the larger of claim and bound. */
  axisMax: number;
  axisStartX: number;
  axisEndX: number;
  claimX: number;
  boundX: number;
  boundMinX: number;
  isRange: boolean;
  /** Region beyond the bound, from the bound to the right end of the axis. */
  hatch: { x: number; width: number };
  /** True when the claim is strictly above the bound. */
  claimInHatch: boolean;
}

const AXIS_HEADROOM = 1.25;

/** Pure geometry of the bound-scale chart. Values below 0 are drawn at 0. */
export function boundScale(input: BoundScaleInput): BoundScale {
  const { claim, boundMin, boundMax, plotLeft, plotWidth } = input;
  const largest = Math.max(claim, boundMax);
  const axisMax = largest > 0 ? largest * AXIS_HEADROOM : 1;
  const x = (v: number): number => plotLeft + (Math.min(Math.max(v, 0), axisMax) / axisMax) * plotWidth;
  const boundX = x(boundMax);
  const axisEndX = plotLeft + plotWidth;
  return {
    axisMax,
    axisStartX: plotLeft,
    axisEndX,
    claimX: x(claim),
    boundX,
    boundMinX: x(boundMin),
    isRange: boundMin !== boundMax,
    hatch: { x: boundX, width: axisEndX - boundX },
    claimInHatch: claim > boundMax,
  };
}

/** Bounds reduced to what the sheet and the chart show. */
export interface BoundsSummary {
  low: number;
  high: number;
  margin: Margin | null;
}

/**
 * Smallest `b_min` and largest `b_max` over both enclosures, and the hull of
 * the margins that are present. Null when the engine returned no bounds.
 */
export function summarizeBounds(bounds: Bounds | null): BoundsSummary | null {
  if (!bounds) return null;
  const encs = [bounds.permissive, bounds.restrictive];
  const margins = encs.map((e) => e.margin).filter((m): m is Margin => m !== null);
  return {
    low: Math.min(...encs.map((e) => e.b_min)),
    high: Math.max(...encs.map((e) => e.b_max)),
    margin:
      margins.length === 0
        ? null
        : {
            low: Math.min(...margins.map((m) => m.low)),
            high: Math.max(...margins.map((m) => m.high)),
          },
  };
}

/** One axis tick: its value, its label and its x position. */
export interface AxisTick {
  value: number;
  label: string;
  x: number;
}

/**
 * Tick steps, per decade. Consecutive steps differ by at most a factor 1.6, so
 * the smallest step giving at most 5 ticks always gives at least 4.
 */
const TICK_STEPS = [1, 1.5, 2, 2.5, 4, 5, 8];

/** Step of the axis ticks: 4 or 5 ticks from 0 to at most `axisMax`. */
export function tickStep(axisMax: number): number {
  const decade = 10 ** Math.floor(Math.log10(axisMax));
  for (const scale of [decade / 10, decade]) {
    for (const s of TICK_STEPS) {
      const step = s * scale;
      if (Math.floor(axisMax / step + 1e-9) <= 4) return step;
    }
  }
  return decade * 10;
}

/** 4 or 5 ticks from 0, evenly spaced, none beyond the end of the axis. */
export function axisTicks(scale: BoundScale): AxisTick[] {
  const step = tickStep(scale.axisMax);
  const count = Math.floor(scale.axisMax / step + 1e-9) + 1;
  const span = scale.axisEndX - scale.axisStartX;
  return Array.from({ length: count }, (_, i) => {
    const value = Number((i * step).toPrecision(12));
    return { value, label: String(value), x: scale.axisStartX + (value / scale.axisMax) * span };
  });
}

/** A value with its unit, the unit omitted when the quantity has none. */
export function withUnit(text: string, unit: string): string {
  return unit === '' || unit === 'dimensionless' ? text : `${text} ${unit}`;
}

/** The claim-to-bound ratio as "0.759 x the bound", or a range "0.900 to 1.20 x the bound". */
export function ratioText(margin: Margin): string {
  const value =
    margin.low === margin.high
      ? formatValue(margin.low)
      : `${formatValue(margin.low)} ${STRINGS.rangeTo} ${formatValue(margin.high)}`;
  return `${value} ${STRINGS.ratioSuffix}`;
}

/** Three significant digits, as shown on the sheet and the chart. */
export function formatValue(v: number): string {
  if (v !== 0 && (Math.abs(v) >= 1000 || Math.abs(v) < 0.001)) return v.toExponential(2);
  return v.toPrecision(3);
}
