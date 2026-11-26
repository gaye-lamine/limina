import { Component, computed, input, output } from '@angular/core';
import { arrowDown, arrowRight } from './arrows';
import { BENCH_FIGURE_STYLES, MIN_FIGURE_SCALE } from './bench-figure';
import { carnotArrows, carnotFlows } from './carnot-geometry';
import { parseDecimal } from './decimal';
import { STRINGS } from './strings';
import { Thermometer } from './thermometer';

/** Figure frame, in viewBox units. */
export const VIEW_WIDTH = 460;
export const VIEW_HEIGHT = 500;
/** The figure is never drawn narrower than this (CSS min-width), so text keeps its size. */
export const MIN_RENDER_WIDTH = Math.ceil(VIEW_WIDTH * MIN_FIGURE_SCALE);
/** Font sizes in viewBox units; at MIN_RENDER_WIDTH none renders below 12 px. */
export const FONT = { label: 17, symbol: 22, sub: 16, value: 17 } as const;

/** A share of the heat taken in (Qh = 100 %): "75 %". */
export function share(f: number): string {
  return `${Number((f * 100).toPrecision(3))} %`;
}

const CX = 190;
const HOT_BOTTOM = 84;
const ENGINE_Y = 226;
const ENGINE_R = 46;
const COLD_TOP = 420;
const WORK_END = 404;
/** Offset of the two heat-out arrows from the engine axis: minimum on the left, claimed on the right. */
const REJECT_OFFSET = 30;
/** The heat-out arrows start under the engine circle, which is drawn over them. */
const REJECT_START = ENGINE_Y + 30;
const CALLOUT_X = 262;
const CALLOUT_Y = 362;
const LINE_STEP = 18;

/**
 * The Carnot apparatus: hot reservoir, engine and cold reservoir, with the
 * thermometers for Th and Tc and the claimed efficiency in the figure frame.
 */
@Component({
  selector: 'app-carnot-apparatus',
  imports: [Thermometer],
  templateUrl: './carnot-apparatus.html',
  styles: [BENCH_FIGURE_STYLES],
})
export class CarnotApparatus {
  readonly th = input<string>('');
  readonly tc = input<string>('');
  readonly claim = input<string>('');
  readonly thUnit = input<string>('K');
  readonly tcUnit = input<string>('K');
  readonly thChange = output<string>();
  readonly tcChange = output<string>();
  readonly claimChange = output<string>();

  protected readonly s = STRINGS;
  protected readonly font = FONT;
  protected readonly minWidth = MIN_RENDER_WIDTH;
  protected readonly minHeight = Math.ceil(VIEW_HEIGHT * MIN_FIGURE_SCALE);
  protected readonly viewBox = `0 0 ${VIEW_WIDTH} ${VIEW_HEIGHT}`;
  protected readonly cx = CX;
  protected readonly engineY = ENGINE_Y;
  protected readonly engineR = ENGINE_R;
  protected readonly hotBottom = HOT_BOTTOM;
  protected readonly coldTop = COLD_TOP;
  protected readonly workStart = CX + ENGINE_R;
  protected readonly workEnd = WORK_END;
  protected readonly minX = CX - REJECT_OFFSET;
  protected readonly claimX = CX + REJECT_OFFSET;
  protected readonly rejectStart = REJECT_START;
  protected readonly calloutX = CALLOUT_X;
  protected readonly calloutY = CALLOUT_Y;
  protected readonly lineStep = LINE_STEP;

  protected readonly flows = computed(() =>
    carnotFlows(parseDecimal(this.th()), parseDecimal(this.tc()), parseDecimal(this.claim())),
  );

  protected readonly arrows = computed(() => {
    const f = this.flows();
    return f ? carnotArrows(f) : null;
  });

  protected readonly shapes = computed(() => {
    const a = this.arrows();
    if (!a) return null;
    const workWidth = Math.max(a.work, a.claimWork);
    return {
      qh: arrowDown(CX, HOT_BOTTOM, ENGINE_Y - ENGINE_R, a.qh),
      work: arrowRight(ENGINE_Y, CX + ENGINE_R, WORK_END, a.work),
      claimWork: a.claimWork > 0 ? arrowRight(ENGINE_Y, CX + ENGINE_R, WORK_END, a.claimWork) : null,
      reject: arrowDown(CX - REJECT_OFFSET, REJECT_START, COLD_TOP, a.reject),
      claimReject:
        a.claimReject > 0 ? arrowDown(CX + REJECT_OFFSET, REJECT_START, COLD_TOP, a.claimReject) : null,
      qhLabelX: CX + a.qh / 2 + 16,
      workLabelTop: ENGINE_Y - workWidth / 2 - 14,
      workLabelBottom: ENGINE_Y + workWidth / 2 + 28,
      /** Labels of the minimum arrow end left of its head; those of the claimed arrow start right of its head. */
      minLabelX: CX - REJECT_OFFSET - a.reject / 2 - 15,
      claimLabelX: CX + REJECT_OFFSET + a.claimReject / 2 + 15,
      claimEdgeX: CX + REJECT_OFFSET + a.claimReject / 2,
      dimTop: ENGINE_Y - a.work / 2,
      dimBottom: ENGINE_Y + a.work / 2,
      showDimension: a.work >= 12,
    };
  });

  protected readonly text = computed(() => {
    const f = this.flows();
    if (!f) return null;
    return {
      work: share(f.boundWork),
      reject: share(f.minReject),
      claimWork: f.claimWork === null ? null : share(f.claimWork),
      claimReject: f.claimReject === null ? null : share(f.claimReject),
    };
  });

  /** The text alternative: the numbers drawn on the figure. */
  protected readonly alt = computed(() => {
    const f = this.flows();
    const t = this.text();
    if (!f || !t) return STRINGS.apparatusNeedTemperatures;
    const a = STRINGS.apparatusAlt;
    const base = `${a.flows} ${t.work}, ${a.reject} ${t.reject}.`;
    if (t.claimWork === null) return `${base} ${a.noClaim}`;
    const verdict = f.exceedsBound ? a.exceeds : a.within;
    return `${base} ${a.claim} ${t.claimWork}, ${a.claimReject} ${t.claimReject}. ${verdict}`;
  });
}
