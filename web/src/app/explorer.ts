import {
  Component,
  OnDestroy,
  computed,
  effect,
  inject,
  input,
  signal,
  untracked,
} from '@angular/core';
import { AnalyzedClaim, CurveResponse, SurfaceResponse } from './analysis.models';
import { AnalysisService } from './analysis.service';
import { ApiFailure, LimitInfo } from './api.models';
import { formatValue, summarizeBounds } from './chart-geometry';
import { comparisonScale, graphGeometry, heatGeometry } from './graph-geometry';
import { WB } from './strings';
import { PLOT, rememberDirection } from './workbench-logic';

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
  /** Curve of one parameter, or heat map of two (laws with two parameters or more). */
  protected readonly view = signal<'curve' | 'map'>('curve');
  protected readonly yParam = signal('');
  protected readonly surface = signal<SurfaceResponse | null>(null);
  /** Relative uncertainty applied to the fixed parameters, in percent; 0 draws no band. */
  protected readonly uncertainty = signal(0);
  protected readonly uncertaintyChoices = [0, 1, 5, 10];
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
    return c ? graphGeometry(c, this.claim().value) : null;
  });
  protected readonly heat = computed(() => {
    const m = this.surface();
    return m ? heatGeometry(m, this.claim().value) : null;
  });
  protected readonly canMap = computed(() => this.parameters().length >= 2);
  protected readonly isEquality = computed(() => this.limit()?.direction === 'Equal');
  /** The claim against the bound on a log axis, for a law with no parameter. */
  protected readonly scale = computed(() => {
    if (this.parameters().length > 0) return null;
    const claim = this.claim();
    const bounds = summarizeBounds(claim.result?.bounds ?? null);
    const direction = this.limit()?.direction;
    if (!bounds || claim.value === null || (direction !== 'Max' && direction !== 'Min'))
      return null;
    const bound = direction === 'Max' ? bounds.high : bounds.low;
    return comparisonScale(claim.value, bound, direction);
  });
  protected readonly otherParameters = computed(() =>
    this.parameters().filter((p) => p.id !== this.free()),
  );

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
        this.surface.set(null);
        this.view.set('curve');
        this.yParam.set(params.find((p) => p.id !== this.free())?.id ?? '');
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
    this.surface.set(null);
    if (this.yParam() === id) this.yParam.set(this.otherParameters()[0]?.id ?? '');
    this.request(0);
  }

  protected chooseView(view: 'curve' | 'map'): void {
    this.view.set(view);
    this.request(0);
  }

  protected chooseY(id: string): void {
    this.yParam.set(id);
    this.surface.set(null);
    this.request(0);
  }

  protected chooseUncertainty(raw: string): void {
    this.uncertainty.set(Number(raw) || 0);
    this.request(0);
  }

  protected slide(id: string, raw: string): void {
    const v = Number(raw);
    if (!Number.isFinite(v)) return;
    this.values.update((m) => ({ ...m, [id]: v }));
    if (id !== this.free() && !(this.view() === 'map' && id === this.yParam())) this.request(150);
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
    if (this.view() === 'map') {
      this.loadSurface(claim, constraints);
      return;
    }
    const pct = this.uncertainty() / 100;
    const uncertainties =
      pct > 0
        ? Object.fromEntries(
            Object.entries(parameters).map(([id, p]) => [id, Math.abs(p.value) * pct]),
          )
        : undefined;
    this.api
      .curve({
        limit_id: claim.limit_id,
        constraints,
        parameters,
        free_parameter: free,
        uncertainties,
      })
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
  private loadSurface(claim: AnalyzedClaim, constraints: Record<string, string[]>): void {
    const x = this.free();
    const y = this.yParam();
    const parameters: Record<string, { value: number; unit: string }> = {};
    for (const [id, v] of Object.entries(this.values())) {
      if (id === x || id === y) continue;
      parameters[id] = { value: v, unit: claim.parameters[id]?.unit || 'K' };
    }
    this.api
      .surface({
        limit_id: claim.limit_id ?? '',
        constraints,
        parameters,
        x_parameter: x,
        y_parameter: y,
      })
      .subscribe({
        next: (m) => {
          rememberDirection(m.limit_id, m.direction);
          this.failure.set(null);
          this.surface.set(m);
          this.loading.set(false);
        },
        error: (f: ApiFailure) => {
          this.failure.set(f);
          this.loading.set(false);
        },
      });
  }
}
