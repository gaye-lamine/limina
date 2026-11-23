import { Component, computed, input, output } from '@angular/core';
import { BoundChart } from './bound-chart';
import { Gauge } from './gauge';
import { formatValue, ratioText, summarizeBounds, withUnit } from './chart-geometry';
import { ApiFailure, Configuration, EvaluateResponse, LimitInfo, Verdict } from './api.models';
import { STRINGS } from './strings';

/** The claim that was evaluated, in the unit of the limit. */
export interface EvaluateClaim {
  value: number;
  unit: string;
}

/** One line of the "Why this bound" block: an assumption and its state. */
export interface ActiveAssumption {
  id: string;
  value: string;
}

const VERDICT_CLASS: Record<Verdict, string> = {
  IMPOSSIBLE: 'verdict--impossible',
  CONDITIONAL: 'verdict--conditional',
  COMPATIBLE: 'verdict--compatible',
  INSUFFICIENT_DATA: 'verdict--insufficient-data',
};

const LEGEND: Verdict[] = ['IMPOSSIBLE', 'CONDITIONAL', 'COMPATIBLE', 'INSUFFICIENT_DATA'];

function range(low: number, high: number): string {
  return low === high
    ? formatValue(low)
    : `${formatValue(low)} ${STRINGS.rangeTo} ${formatValue(high)}`;
}

/**
 * Assumptions as evaluated: the chosen option, the only option when the domain
 * has one, otherwise "not specified".
 */
export function activeAssumptions(
  limit: LimitInfo,
  constraints: Record<string, string[]>,
): ActiveAssumption[] {
  return limit.assumptions.map((a) => {
    const chosen = constraints[a.id];
    if (chosen && chosen.length > 0) return { id: a.id, value: chosen.join(', ') };
    if (a.options.length === 1) return { id: a.id, value: a.options[0] };
    return { id: a.id, value: STRINGS.whyNotSpecified };
  });
}

@Component({
  selector: 'app-result-panel',
  imports: [BoundChart, Gauge],
  templateUrl: './result-panel.html',
  host: { class: 'result-sticky' },
})
export class ResultPanel {
  readonly result = input<EvaluateResponse | null>(null);
  readonly failure = input<ApiFailure | null>(null);
  readonly claim = input<EvaluateClaim | null>(null);
  readonly limitId = input<string | null>(null);
  /** Metadata of the evaluated limit, for the chart label and "Why this bound". */
  readonly limitInfo = input<LimitInfo | null>(null);
  /** Constraints sent with the evaluated request. */
  readonly constraints = input<Record<string, string[]>>({});
  /** Whether to draw the linear bound chart; off when the bench shows an apparatus. */
  readonly showChart = input<boolean>(true);
  /** Number of figures already on the bench, so captions number on from there. */
  readonly figureOffset = input<number>(0);
  /** Emits the option id chosen in the guided recovery. */
  readonly recover = output<string>();

  protected readonly recoveryUnshrouded = 'unshrouded_rotor';
  protected readonly recoveryShrouded = 'shrouded_total_capture_area';

  protected readonly s = STRINGS;
  protected readonly legend = LEGEND;

  protected readonly summary = computed(() => summarizeBounds(this.result()?.bounds ?? null));

  protected readonly unit = computed(() => this.claim()?.unit ?? '');

  /** Distinct configuration bounds for the gauge, or null without bounds or claim. */
  protected readonly gaugeBounds = computed(() => {
    const b = this.result()?.bounds;
    if (!b || !this.claim()) return null;
    return [b.permissive.b_min, b.permissive.b_max, b.restrictive.b_min, b.restrictive.b_max];
  });

  protected readonly sheet = computed(() => {
    const claim = this.claim();
    if (!claim) return null;
    const sum = this.summary();
    return {
      bound: sum ? withUnit(range(sum.low, sum.high), claim.unit) : STRINGS.resultNoMargin,
      claim: withUnit(formatValue(claim.value), claim.unit),
      ratio: sum?.margin ? ratioText(sum.margin) : STRINGS.resultNoMargin,
    };
  });

  protected readonly boundName = computed(
    () => this.limitInfo()?.title || STRINGS.chartBoundLabel,
  );

  protected readonly why = computed(() => {
    const limit = this.limitInfo();
    if (!limit || !this.result()) return null;
    return { limit, assumptions: activeAssumptions(limit, this.constraints()) };
  });

  protected readonly missingIds = computed(() =>
    (this.result()?.missing_parameters ?? []).map((m) => m.id).join(', '),
  );

  protected readonly verdictClass = computed(() => {
    const r = this.result();
    return r ? VERDICT_CLASS[r.verdict] : '';
  });

  protected readonly verdictLabel = computed(() => {
    const r = this.result();
    return r ? STRINGS.verdictLabel[r.verdict] : '';
  });

  protected readonly needsReferenceArea = computed(() => this.failure()?.code === 'NotApplicable');
  protected readonly offersRecovery = computed(() => this.needsReferenceArea() && this.limitId() === 'betz');

  protected classOf(v: Verdict): string {
    return VERDICT_CLASS[v];
  }

  protected assumptionText(c: Configuration): string {
    return Object.entries(c.assumptions)
      .map(([a, o]) => `${a} = ${o}`)
      .join(', ');
  }
}
