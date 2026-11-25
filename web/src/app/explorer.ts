import { Component, OnDestroy, computed, effect, inject, input, signal, untracked } from '@angular/core';
import { AnalyzedClaim, CurveResponse } from './analysis.models';
import { AnalysisService } from './analysis.service';
import { ApiFailure, LimitInfo } from './api.models';
import { formatValue } from './chart-geometry';
import { WB } from './strings';
import { PLOT, plotGeometry, rememberDirection } from './workbench-logic';

/** One slider: a parameter of the law, its unit and its range. */
interface Slider {
  id: string;
  unit: string;
  min: number;
  max: number;
  step: number;
}

/**
 * The explorer of a law: the bound against one chosen parameter, computed
 * by the engine through POST /v1/curve, the claim as a horizontal line and
 * the region inconsistent with the law hatched. Every slider asks the
 * engine again; the browser only draws.
 */
@Component({
  selector: 'app-explorer',
  templateUrl: './explorer.html',
})
export class Explorer implements OnDestroy {
  readonly claim = input.required<AnalyzedClaim>();
  readonly limit = input<LimitInfo | null>(null);

  private readonly api = inject(AnalysisService);
  protected readonly s = WB;
  protected readonly plot = PLOT;
  protected readonly free = signal('');
  protected readonly values = signal<Record<string, number>>({});
  protected readonly curve = signal<CurveResponse | null>(null);
  protected readonly failure = signal<ApiFailure | null>(null);
  protected readonly loading = signal(false);
  private timer: ReturnType<typeof setTimeout> | null = null;

  protected readonly parameters = computed(() => this.limit()?.parameters ?? []);

  protected readonly sliders = computed<Slider[]>(() => {
    const c = this.curve();
    const v = this.values();
    return this.parameters().map((p) => {
      const unit = this.claim().parameters[p.id]?.unit || p.units[0] || '';
      const value = v[p.id];
      let [min, max] =
        c && c.free_parameter === p.id
          ? c.range
          : value !== undefined
            ? [value * 0.5, value * 1.5]
            : [0, 1];
      if (value !== undefined) {
        min = Math.min(min, value);
        max = Math.max(max, value);
      }
      return { id: p.id, unit, min, max, step: (max - min) / 200 || 1 };
    });
  });

  protected readonly geometry = computed(() => {
    const c = this.curve();
    return c ? plotGeometry(c.points, c.direction, this.claim().value) : null;
  });

  /** Position of the free parameter's own value on the axis. */
  protected readonly marker = computed(() => {
    const c = this.curve();
    const v = this.values()[this.free()];
    if (!c || v === undefined) return null;
    const [lo, hi] = c.range;
    if (v < lo || v > hi) return null;
    return PLOT.left + ((v - lo) / (hi - lo || 1)) * (PLOT.width - PLOT.left - PLOT.right);
  });

  protected readonly alt = computed(() => {
    const c = this.curve();
    if (!c) return '';
    const pts = c.points.filter((p) => p.decisive !== null);
    if (!pts.length) return '';
    const a = pts[0];
    const b = pts[pts.length - 1];
    return `${WB.explorerAlt} ${c.free_parameter}: ${formatValue(a.decisive ?? 0)} at ${formatValue(a.x)}, ${formatValue(b.decisive ?? 0)} at ${formatValue(b.x)}; ${WB.explorerClaim} ${formatValue(this.claim().value ?? 0)}.`;
  });

  constructor() {
    effect(() => {
      const claim = this.claim();
      const params = this.parameters();
      untracked(() => {
        const values: Record<string, number> = {};
        for (const p of params) {
          const v = claim.parameters[p.id]?.value;
          if (v !== null && v !== undefined) values[p.id] = v;
        }
        this.values.set(values);
        const missing = params.find((p) => values[p.id] === undefined);
        this.free.set(missing?.id ?? params[0]?.id ?? '');
        this.curve.set(null);
        this.request(0);
      });
    });
  }

  ngOnDestroy(): void {
    if (this.timer) clearTimeout(this.timer);
  }

  protected chooseFree(id: string): void {
    this.free.set(id);
    this.curve.set(null);
    this.request(0);
  }

  protected slide(id: string, raw: string): void {
    const v = Number(raw);
    if (!Number.isFinite(v)) return;
    this.values.update((m) => ({ ...m, [id]: v }));
    if (id !== this.free()) this.request(150);
  }

  protected valueOr(id: string, fallback: number): number {
    const v: number | undefined = this.values()[id];
    return v === undefined ? fallback : v;
  }

  protected label(n: number): string {
    return formatValue(n);
  }

  private request(delay: number): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => this.load(), delay);
  }

  private load(): void {
    const claim = this.claim();
    const free = this.free();
    if (!claim.limit_id || !free) return;
    const parameters: Record<string, { value: number; unit: string }> = {};
    for (const [id, v] of Object.entries(this.values())) {
      if (id === free) continue;
      parameters[id] = { value: v, unit: claim.parameters[id]?.unit || 'K' };
    }
    const constraints = Object.fromEntries(
      Object.entries(claim.constraints).map(([a, o]) => [a, [o]]),
    );
    this.loading.set(true);
    this.api
      .curve({ limit_id: claim.limit_id, constraints, parameters, free_parameter: free })
      .subscribe({
        next: (c) => {
          rememberDirection(c.limit_id, c.direction);
          this.failure.set(null);
          this.curve.set(c);
          this.loading.set(false);
        },
        error: (f: ApiFailure) => {
          this.failure.set(f);
          this.loading.set(false);
        },
      });
  }
}
