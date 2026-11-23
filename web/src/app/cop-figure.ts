import { Component, computed, input } from '@angular/core';
import { arrowRight, arrowUp } from './arrows';
import { BENCH_FIGURE_STYLES, MIN_FIGURE_SCALE } from './bench-figure';
import { formatValue } from './chart-geometry';
import { CopMode, copArrows, copFlows } from './cop-geometry';
import { parseDecimal } from './decimal';
import { STRINGS } from './strings';

/** Figure frame, in viewBox units. */
export const COP_VIEW_WIDTH = 460;
export const COP_VIEW_HEIGHT = 500;
/** Font sizes in viewBox units, full size and for the small twin figures. */
export const COP_FONT = { label: 15, symbol: 20, sub: 15, value: 15 } as const;
export const COP_FONT_COMPACT = { label: 20, symbol: 24, sub: 20, value: 20 } as const;
/** Smallest drawn width, so no text renders below 12 px. */
export const COP_MIN_WIDTH = Math.ceil(COP_VIEW_WIDTH * MIN_FIGURE_SCALE);
export const COP_MIN_WIDTH_COMPACT = Math.ceil((COP_VIEW_WIDTH * 12) / COP_FONT_COMPACT.label);

const CX = 230;
const HOT_BOTTOM = 84;
const MACHINE_Y = 250;
const MACHINE_R = 46;
const COLD_TOP = 416;
const WORK_START = 24;

/**
 * Schematic of a heat pump or refrigerator: hot reservoir on top, machine in
 * the middle, cold reservoir at the bottom. W enters from the side, Qc comes
 * up from the cold reservoir, Qh goes up to the hot reservoir.
 */
@Component({
  selector: 'app-cop-figure',
  templateUrl: './cop-figure.html',
  styles: [BENCH_FIGURE_STYLES, ':host { display: block; }'],
})
export class CopFigure {
  readonly th = input<string>('');
  readonly tc = input<string>('');
  readonly claim = input<string>('');
  readonly thUnit = input<string>('K');
  readonly tcUnit = input<string>('K');
  readonly mode = input.required<CopMode>();
  /** Small twin figure: larger text, fewer words. */
  readonly compact = input<boolean>(false);
  /** Prefix of the SVG ids, so two figures on the page do not share them. */
  readonly idPrefix = input<string>('cop');

  protected readonly s = STRINGS;
  protected readonly viewBox = `0 0 ${COP_VIEW_WIDTH} ${COP_VIEW_HEIGHT}`;
  protected readonly cx = CX;
  protected readonly machineY = MACHINE_Y;
  protected readonly machineR = MACHINE_R;
  protected readonly hotBottom = HOT_BOTTOM;
  protected readonly coldTop = COLD_TOP;
  protected readonly workStart = WORK_START;

  protected readonly font = computed(() => (this.compact() ? COP_FONT_COMPACT : COP_FONT));
  protected readonly minWidth = computed(() => (this.compact() ? COP_MIN_WIDTH_COMPACT : COP_MIN_WIDTH));
  protected readonly minHeight = computed(() =>
    Math.ceil((this.minWidth() * COP_VIEW_HEIGHT) / COP_VIEW_WIDTH),
  );

  protected readonly flows = computed(() =>
    copFlows(parseDecimal(this.th()), parseDecimal(this.tc()), parseDecimal(this.claim()), this.mode()),
  );

  protected readonly heating = computed(() => this.mode() === 'heating');

  protected readonly shapes = computed(() => {
    const f = this.flows();
    if (!f) return null;
    const a = copArrows(f);
    const qhWidth = this.heating() ? a.useful : a.other;
    const qcWidth = this.heating() ? a.other : a.useful;
    const up = (y0: number, y1: number, w: number) => arrowUp(CX, y0, y1, w);
    const qh = up(MACHINE_Y - MACHINE_R + 6, HOT_BOTTOM, qhWidth);
    const qc = up(COLD_TOP, MACHINE_Y + MACHINE_R - 6, qcWidth);
    const claimed =
      a.claimed > 0
        ? this.heating()
          ? up(MACHINE_Y - MACHINE_R + 6, HOT_BOTTOM, a.claimed)
          : up(COLD_TOP, MACHINE_Y + MACHINE_R - 6, a.claimed)
        : null;
    const widest = Math.max(qhWidth, qcWidth, a.claimed);
    return {
      qh,
      qc,
      claimed,
      w: arrowRight(MACHINE_Y, WORK_START, CX - MACHINE_R + 6, a.w),
      labelX: CX + widest / 2 + 7 + 12,
      workLabelY: MACHINE_Y - a.w / 2 - 14,
      /** Labels of the useful flow, next to its arrow. */
      usefulY: this.heating() ? [132, 160] : [350, 378],
      otherY: this.heating() ? 350 : 146,
    };
  });

  protected readonly text = computed(() => {
    const f = this.flows();
    if (!f) return null;
    return {
      useful: formatValue(f.usefulAllowed),
      other: formatValue(f.otherHeatAllowed),
      claimed: f.usefulClaimed === null ? null : formatValue(f.usefulClaimed),
    };
  });

  /** Subscript of the useful heat symbol: h when heating, c when cooling. */
  protected readonly usefulSub = computed(() => (this.heating() ? 'h' : 'c'));
  protected readonly otherSub = computed(() => (this.heating() ? 'c' : 'h'));

  /** The text alternative: the numbers drawn on the figure. */
  protected readonly alt = computed(() => {
    const f = this.flows();
    const t = this.text();
    if (!f || !t) return STRINGS.apparatusNeedTemperatures;
    const a = STRINGS.copAlt;
    const useful = this.heating() ? a.usefulHeating : a.usefulCooling;
    const other = this.heating() ? a.otherHeating : a.otherCooling;
    const base = `${a.work} ${useful} ${t.useful}, ${other} ${t.other}.`;
    if (t.claimed === null) return `${base} ${a.noClaim}`;
    return `${base} ${a.claim} ${t.claimed}. ${f.exceedsBound ? a.exceeds : a.within}`;
  });
}
