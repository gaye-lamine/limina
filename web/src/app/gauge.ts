import { Component, computed, input } from '@angular/core';
import { formatValue, withUnit } from './chart-geometry';
import { gaugeGeometry, placeBoundLabels, polar, sectorPath } from './gauge-geometry';
import { STRINGS } from './strings';

/** Dial frame, in viewBox units. */
export const GAUGE_VIEW_WIDTH = 450;
export const GAUGE_VIEW_HEIGHT = 204;
/** Font sizes in viewBox units. */
export const GAUGE_FONT = { bound: 16, scale: 12, claim: 15 } as const;
/** The dial is never drawn narrower than this (CSS min-width), so bound labels stay at least 14 px. */
export const GAUGE_MIN_RENDER_WIDTH = Math.ceil((GAUGE_VIEW_WIDTH * 14) / GAUGE_FONT.bound);
export const CX = 225;
export const CY = 160;
/** Outer and inner radius of the dial band. */
export const R = 120;
export const R0 = 94;
/** Bound ticks reach out to this radius; their labels sit outside it. */
export const TICK_OUTER = R + 8;
export const LABEL_RADIUS = R + 14;
/** Baseline of the claim label, below the hub. */
export const CLAIM_LABEL_Y = CY + 36;

/**
 * Semicircular gauge, reusable for every limit: one tick per configuration
 * bound labelled "bound value", a needle for the claim, the zone beyond the
 * largest bound hatched.
 */
@Component({
  selector: 'app-gauge',
  templateUrl: './gauge.html',
})
export class Gauge {
  readonly claim = input.required<number>();
  /** Bound of each configuration (duplicates are merged). */
  readonly bounds = input.required<number[]>();
  readonly unit = input<string>('');
  /** Figure number in the caption; 0 shows no number. */
  readonly figure = input<number>(0);

  protected readonly s = STRINGS;
  protected readonly font = GAUGE_FONT;
  protected readonly cx = CX;
  protected readonly cy = CY;
  protected readonly claimLabelY = CLAIM_LABEL_Y;
  protected readonly viewBox = `0 0 ${GAUGE_VIEW_WIDTH} ${GAUGE_VIEW_HEIGHT}`;
  protected readonly minWidth = GAUGE_MIN_RENDER_WIDTH;

  protected readonly g = computed(() => gaugeGeometry(this.claim(), this.bounds()));

  /** The dial outline, from the left end to the right end. */
  protected readonly dialPath = sectorPath(CX, CY, R, R0, 180, 0);

  protected readonly hatchPath = computed(() => {
    const start = this.g().hatchStart;
    return start > 0 ? sectorPath(CX, CY, R, R0, start, 0) : null;
  });

  /** Graduations on the inner edge of the band; the end values are written under the dial. */
  protected readonly scale = computed(() =>
    this.g().scaleTicks.map((t) => {
      const a = polar(CX, CY, R0, t.angle);
      const b = polar(CX, CY, R0 + 8, t.angle);
      // Only the two ends are labelled, under the dial, so the needle never crosses a label.
      const e = polar(CX, CY, (R + R0) / 2, t.angle);
      return { ...t, a, b, e };
    }),
  );

  protected readonly boundMarks = computed(() => {
    const ticks = this.g().boundTicks;
    const places = placeBoundLabels(
      ticks.map((t) => t.angle),
      { cx: CX, cy: CY, outer: TICK_OUTER, radius: LABEL_RADIUS, size: GAUGE_FONT.bound },
    );
    return ticks.map((t, i) => ({
      ...t,
      a: polar(CX, CY, R0 - 6, t.angle),
      b: polar(CX, CY, TICK_OUTER, t.angle),
      place: places[i],
      text: `${STRINGS.gaugeBound} ${withUnit(t.label, this.unit())}`,
    }));
  });

  protected readonly needle = computed(() => polar(CX, CY, R - 4, this.g().needleAngle));
  /** Scale label as computed; a label longer than 6 characters is written short and only every other one is shown. */
  protected scaleLabel(label: string, value: number, index: number): string {
    if (label.length <= 6) return label;
    return index % 2 === 0 ? formatValue(value) : '';
  }
  /** Start rotation of the needle sweep: from the left end of the dial to the claim. */
  protected readonly needleFrom = computed(() => `${this.g().needleAngle - 180}deg`);

  protected readonly claimText = computed(() => withUnit(formatValue(this.claim()), this.unit()));

  /** The text alternative: the same information as the dial. */
  protected readonly alt = computed(() => {
    const g = this.g();
    const bounds = g.boundTicks.map((t) => withUnit(t.label, this.unit())).join(', ');
    const position = g.claimBeyond ? STRINGS.gaugeBeyond : STRINGS.gaugeWithin;
    return (
      `${STRINGS.gaugeClaim} ${this.claimText()}; ${STRINGS.gaugeBounds} ${bounds}. ${position} ` +
      `${STRINGS.chartScale} ${STRINGS.chartAxisStart} ${STRINGS.rangeTo} ${formatValue(g.max)}.`
    );
  });

  protected readonly caption = computed(() =>
    this.figure() > 0
      ? `${STRINGS.figurePrefix} ${this.figure()}: ${STRINGS.gaugeFigure}`
      : STRINGS.gaugeFigure,
  );
}
