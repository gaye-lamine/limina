import { Component, computed, input } from '@angular/core';
import { Margin } from './api.models';
import { axisTicks, boundScale, formatValue, ratioText, withUnit } from './chart-geometry';
import { STRINGS } from './strings';

/** Chart frame, in viewBox units. */
const VIEW_WIDTH = 640;
const PLOT_LEFT = 24;
const PLOT_WIDTH = 592;
/** Labels closer than this to an edge are anchored on that edge. */
const EDGE = 96;

/** Text anchor that keeps a label centred on `x` inside the frame. */
function anchorAt(x: number): 'start' | 'middle' | 'end' {
  if (x < EDGE) return 'start';
  if (x > VIEW_WIDTH - EDGE) return 'end';
  return 'middle';
}

/** Bound-scale chart: inline SVG, black, white and grey only. */
@Component({
  selector: 'app-bound-chart',
  templateUrl: './bound-chart.html',
})
export class BoundChart {
  readonly claim = input.required<number>();
  readonly boundMin = input.required<number>();
  readonly boundMax = input.required<number>();
  /** Name shown with the bound, for example "Betz limit". */
  readonly boundName = input<string>(STRINGS.chartBoundLabel);
  /** Unit of claim and bound; not shown when dimensionless. */
  readonly unit = input<string>('');
  /** Claim-to-bound ratio from the API, when computed. */
  readonly margin = input<Margin | null>(null);
  /** Figure number in the caption; 0 shows no number. */
  readonly figure = input<number>(0);

  protected readonly figureLabel = computed(() =>
    this.figure() > 0
      ? `${STRINGS.figurePrefix} ${this.figure()}: ${STRINGS.chartFigure}`
      : STRINGS.chartFigure,
  );

  protected readonly s = STRINGS;

  protected readonly g = computed(() =>
    boundScale({
      claim: this.claim(),
      boundMin: this.boundMin(),
      boundMax: this.boundMax(),
      plotLeft: PLOT_LEFT,
      plotWidth: PLOT_WIDTH,
    }),
  );

  protected readonly ticks = computed(() => axisTicks(this.g()));

  protected readonly claimText = computed(() => withUnit(formatValue(this.claim()), this.unit()));

  protected readonly boundValue = computed(() =>
    withUnit(
      this.g().isRange
        ? `${formatValue(this.boundMin())} ${STRINGS.rangeTo} ${formatValue(this.boundMax())}`
        : formatValue(this.boundMax()),
      this.unit(),
    ),
  );

  protected readonly claimLabel = computed(() => `${STRINGS.chartClaimLabel} ${this.claimText()}`);
  protected readonly boundLabel = computed(() => `${this.boundName()} ${this.boundValue()}`);
  protected readonly claimAnchor = computed(() => anchorAt(this.g().claimX));
  protected readonly boundAnchor = computed(() => anchorAt(this.g().boundX));

  /** The text alternative: the same information as the drawing. */
  protected readonly alt = computed(() => {
    const g = this.g();
    const position = g.claimInHatch ? STRINGS.chartAbove : STRINGS.chartBelow;
    const range = g.isRange ? ` ${STRINGS.chartRangeNote}` : '';
    const margin = this.margin();
    const ratio = margin ? ` ${STRINGS.chartRatio} ${ratioText(margin)}.` : '';
    return (
      `${this.claimLabel()}; ${this.boundLabel()}. ${position}${range}${ratio} ` +
      `${STRINGS.chartScale} ${STRINGS.chartAxisStart} ${STRINGS.rangeTo} ${formatValue(g.axisMax)}.`
    );
  });
}
