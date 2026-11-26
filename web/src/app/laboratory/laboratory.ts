import {
  AfterViewInit,
  Component,
  DestroyRef,
  ElementRef,
  NgZone,
  OnDestroy,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { NgTemplateOutlet } from '@angular/common';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { Router } from '@angular/router';
import { Observable, Subject, catchError, debounceTime, forkJoin, map, of, switchMap } from 'rxjs';
import { CurveResponse, SurfaceResponse } from '../analysis.models';
import { AnalysisService } from '../analysis.service';
import { ApiFailure, EvaluateResponse, LimitInfo } from '../api.models';
import { ApiService } from '../api.service';
import { LAB } from '../strings';
import {
  Chooser,
  LabResult,
  Slider,
  STEPS,
  applyInspect,
  choosers,
  claimSlider,
  constraintsOf,
  evaluateRequest,
  failureOf,
  fmt,
  fromPosition,
  inBase,
  parameterSlider,
  parametersOf,
  resultOf,
  toPosition,
  unitText,
  withRange,
  withValue,
} from './lab-model';
import { readInspect } from './inspect';
import { convert } from './lab-units';
import { Observatory, webglAvailable } from './scene';
import { skinFor } from './skin';
import { readTheme } from './theme';
import { canSolve, topologyFor } from './topology';
import { Drawn, LawView, ViewData } from './views';
import { apertureParameter, apparatusFor, discreteParameter, massParameter, spectralParameter, toNanometres } from './apparatus';
import { ApparatusData, ApparatusView, BenchState } from './apparatus-view';

/** What one live update brings back. */
interface Answer {
  result: LabResult;
  curve: CurveResponse | null;
  surface: SurfaceResponse | null;
}

/** A pinned probe: the values and the engine's result at the time it was pinned. */
interface Pin {
  limitId: string;
  params: Slider[];
  claim: number;
  result: LabResult | null;
}

/**
 * The 3D laboratory (spec laboratory-3d), loaded lazily on `/laboratory`.
 * Every law, control and default comes from `GET /v1/limits`; every
 * verdict, bound and shape from the engine's endpoints.
 */
@Component({
  selector: 'app-laboratory',
  templateUrl: './laboratory.html',
  styleUrl: './laboratory.css',
  imports: [NgTemplateOutlet],
})
export class Laboratory implements AfterViewInit, OnDestroy {
  protected readonly t = LAB;
  protected readonly steps = STEPS;
  protected readonly fmt = fmt;
  protected readonly unitText = unitText;
  protected readonly toPosition = toPosition;
  private readonly api = inject(ApiService);
  private readonly analysis = inject(AnalysisService);
  private readonly router = inject(Router);
  private readonly zone = inject(NgZone);
  private readonly destroyRef = inject(DestroyRef);
  private readonly host = viewChild.required<ElementRef<HTMLElement>>('stage');
  private observatory: Observatory | null = null;
  private view: LawView | null = null;
  private bench: ApparatusView | null = null;
  private readonly reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  private readonly changes = new Subject<void>();
  private loadSeq = 0;
  private animation = 0;
  private inspect = readInspect(history.state);

  protected readonly limits = signal<LimitInfo[] | null>(null);
  protected readonly failed = signal(false);
  protected readonly viewReady = signal(false);
  protected readonly noView = signal(false);
  protected readonly selectedId = signal('');
  protected readonly params = signal<Slider[]>([]);
  protected readonly claim = signal<Slider | null>(null);
  protected readonly options = signal<Chooser[]>([]);
  /** The unit each slider is shown and typed in. */
  protected readonly shown = signal<Record<string, string>>({});
  protected readonly xAxis = signal('');
  protected readonly zAxis = signal('');
  protected readonly result = signal<LabResult | null>(null);
  protected readonly busy = signal(false);
  protected readonly solving = signal(false);
  protected readonly solveNote = signal('');
  protected readonly pin = signal<Pin | null>(null);
  /** What the last draw put in the scene (exposed as data attributes for the end-to-end checks). */
  protected readonly drawn = signal<Drawn>({ hatchedCells: 0, zones: 0 });
  /** Physical apparatus (the default) or mathematical boundary. */
  protected readonly mode = signal<'apparatus' | 'boundary'>('apparatus');
  /** What the physical apparatus shows (exposed as a data attribute for the end-to-end checks). */
  protected readonly benchState = signal<BenchState | null>(null);
  protected readonly benchKind = computed(() => {
    const l = this.limit();
    return l ? apparatusFor(l) : null;
  });
  private readonly curve = signal<CurveResponse | null>(null);
  private readonly surface = signal<SurfaceResponse | null>(null);

  protected readonly builtIn = computed(() => (this.limits() ?? []).filter((l) => l.status !== 'draft').length);
  protected readonly drafts = computed(() => (this.limits() ?? []).filter((l) => l.status === 'draft').length);
  protected readonly groups = computed(() => {
    const all = this.limits() ?? [];
    const byTitle = (a: LimitInfo, b: LimitInfo) => a.title.localeCompare(b.title);
    const built = all.filter((l) => l.status !== 'draft');
    return [
      { label: LAB.groupMax, laws: built.filter((l) => l.direction === 'Max').sort(byTitle) },
      { label: LAB.groupMin, laws: built.filter((l) => l.direction === 'Min').sort(byTitle) },
      { label: LAB.groupEqual, laws: built.filter((l) => l.direction === 'Equal').sort(byTitle) },
      { label: LAB.draftGroup, laws: all.filter((l) => l.status === 'draft').sort(byTitle) },
    ].filter((g) => g.laws.length > 0);
  });
  protected readonly limit = computed(() => (this.limits() ?? []).find((l) => l.id === this.selectedId()) ?? null);
  protected readonly isDraft = computed(() => this.limit()?.status === 'draft');
  protected readonly topology = computed(() => {
    const l = this.limit();
    return l ? topologyFor(l) : 'chamber';
  });
  protected readonly solvable = computed(() => {
    const l = this.limit();
    return !!l && canSolve(l);
  });
  /** The parameter "Find breaking point" moves: the width axis of a surface, the corridor's parameter. */
  protected readonly freeParam = computed(() => {
    const ps = this.params();
    return this.topology() === 'surface' ? (ps.find((p) => p.id === this.xAxis()) ?? null) : (ps[0] ?? null);
  });
  protected readonly sceneText = computed(() => {
    const ps = this.params();
    const label = (id: string) => ps.find((p) => p.id === id)?.label ?? id;
    const r = this.result();
    const shape =
      this.topology() === 'surface'
        ? LAB.views.surface(label(this.xAxis()), label(this.zAxis()))
        : this.topology() === 'corridor'
          ? LAB.views.corridor(ps[0]?.label ?? '')
          : LAB.views.chamber;
    const zones = this.topology() === 'corridor' && (this.curve()?.validity_zones?.length ?? 0) > 0 ? ` ${LAB.views.outsideZones}` : '';
    const what = this.mode() === 'apparatus' ? (LAB.benches[this.benchKind() ?? 'quantum'] ?? '') : `${shape}${zones}`;
    return `${LAB.regionLabel}. ${this.limit()?.title ?? ''}. ${what}${r ? ` ${r.label}.` : ''}`;
  });

  constructor() {
    this.api.getLimits().subscribe({
      next: (r) => {
        this.limits.set(r.limits);
        const wanted = this.inspect && r.limits.some((l) => l.id === this.inspect?.limit_id) ? this.inspect.limit_id : null;
        const first = wanted ?? this.groups()[0]?.laws[0]?.id;
        if (first) this.select(first);
      },
      error: () => this.failed.set(true),
    });
    this.changes
      .pipe(
        debounceTime(150),
        switchMap(() => this.ask()),
        takeUntilDestroyed(this.destroyRef),
      )
      .subscribe((a) => this.take(a));
  }

  ngAfterViewInit(): void {
    if (!webglAvailable()) {
      this.noView.set(true);
      return;
    }
    // The render loop runs outside Angular: no change detection per frame.
    this.zone.runOutsideAngular(() => {
      const theme = readTheme();
      this.observatory = new Observatory(this.host().nativeElement, theme, this.reduced);
      this.view = new LawView(this.observatory.stage, theme, this.reduced);
      this.bench = new ApparatusView(this.observatory.stage, theme, this.reduced);
      this.observatory.onFrame = (s) => {
        this.view?.tick(s);
        this.bench?.tick(s);
      };
    });
    this.viewReady.set(true);
    this.redraw();
  }

  // ---------- choosing a law ----------

  protected onLaw(event: Event): void {
    this.select((event.target as HTMLSelectElement).value);
  }

  /** Builds the controls of a law from its metadata, then asks the engine for ranges and the bound. */
  private select(id: string): void {
    const limit = (this.limits() ?? []).find((l) => l.id === id);
    if (!limit) return;
    const seq = ++this.loadSeq;
    cancelAnimationFrame(this.animation);
    this.selectedId.set(id);
    this.pin.set(null);
    this.solveNote.set('');
    this.result.set(null);
    this.curve.set(null);
    this.surface.set(null);
    let params = limit.parameters.map((p) => parameterSlider(limit, p));
    let cs = choosers(limit);
    this.xAxis.set(params[0]?.id ?? '');
    this.zAxis.set(params[1]?.id ?? '');
    this.shown.set(Object.fromEntries([...params.map((p) => [p.id, p.base]), ['claim', limit.claim_unit || 'dimensionless']]));
    this.params.set(params);
    this.options.set(cs);
    this.claim.set(claimSlider(limit, null));
    const inspect = this.inspect?.limit_id === id ? this.inspect : null;
    this.inspect = null;
    this.ranges(limit, params, cs).subscribe((ranged) => {
      if (seq !== this.loadSeq) return;
      params = ranged;
      if (inspect) ({ params, choosers: cs } = applyInspect(inspect, params, cs));
      this.params.set(params);
      this.options.set(cs);
      const claimed = inspect ? inBase(inspect.claim, limit.claim_unit) : null;
      const probe = claimSlider(limit, null, claimed ?? 1);
      this.api
        .evaluate(evaluateRequest(limit, probe, params, cs))
        .pipe(catchError(() => of(null)))
        .subscribe((r) => {
          if (seq !== this.loadSeq) return;
          const bound = r ? resultOf(r, probe.value, limit.direction).bound : null;
          this.claim.set(claimSlider(limit, bound, claimed ?? undefined));
          this.changes.next();
        });
    });
  }

  /** The engine's range of each parameter, from a two-point curve; the fallback range when it gives none. */
  private ranges(limit: LimitInfo, params: Slider[], cs: Chooser[]): Observable<Slider[]> {
    if (params.length === 0) return of(params);
    const constraints = constraintsOf(cs);
    return forkJoin(
      params.map((p) =>
        this.analysis
          .curve({ limit_id: limit.id, constraints, parameters: parametersOf(params, [p.id]), free_parameter: p.id, points: 2 })
          .pipe(
            map((c) => withRange(p, c.range)),
            catchError(() => of(p)),
          ),
      ),
    );
  }

  // ---------- controls ----------

  protected onTrack(id: string, event: Event): void {
    const position = Number((event.target as HTMLInputElement).value);
    this.update(id, (s) => ({ ...s, value: fromPosition(s, position) }));
  }

  protected onTyped(id: string, event: Event): void {
    const input = event.target as HTMLInputElement;
    const typed = Number(input.value);
    if (input.value.trim() === '' || !Number.isFinite(typed)) return;
    this.update(id, (s) => {
      const v = convert(typed, this.unitOf(s), s.base);
      return v === null ? s : withValue(s, v);
    });
  }

  protected onUnit(id: string, event: Event): void {
    const unit = (event.target as HTMLSelectElement).value;
    this.shown.update((u) => ({ ...u, [id]: unit }));
    this.redraw();
  }

  protected onChoice(id: string, event: Event): void {
    const value = (event.target as HTMLSelectElement).value;
    this.options.update((cs) => cs.map((c) => (c.id === id ? { ...c, value } : c)));
    this.changes.next();
  }

  protected onAxis(which: 'x' | 'z', event: Event): void {
    const id = (event.target as HTMLSelectElement).value;
    const other = which === 'x' ? this.zAxis : this.xAxis;
    const mine = which === 'x' ? this.xAxis : this.zAxis;
    if (other() === id) other.set(mine());
    mine.set(id);
    this.changes.next();
  }

  /** The slider's unit on screen. */
  protected unitOf(s: Slider): string {
    return this.shown()[s.id] ?? s.base;
  }

  /** The slider's value in its unit on screen. */
  protected shownValue(s: Slider): string {
    const v = convert(s.value, s.base, this.unitOf(s));
    return v === null ? fmt(s.value) : fmt(v);
  }

  /** A value of the claim's unit, written in the claim's unit on screen. */
  protected claimText(v: number | null): string {
    const c = this.claim();
    if (v === null || !c) return '—';
    const unit = this.unitOf(c);
    const shown = convert(v, c.base, unit) ?? v;
    return `${fmt(shown)} ${unitText(unit)}`.trim();
  }

  private update(id: string, f: (s: Slider) => Slider): void {
    if (id === 'claim') {
      const c = this.claim();
      if (c) this.claim.set(f(c));
    } else {
      this.params.update((ps) => ps.map((s) => (s.id === id ? f(s) : s)));
    }
    this.redraw();
    this.changes.next();
  }

  // ---------- live evaluation ----------

  /** One request to the evaluator and one to the shape of the law, from the current values. */
  private ask(): Observable<Answer | null> {
    const limit = this.limit();
    const claim = this.claim();
    if (!limit || !claim) return of(null);
    const params = this.params();
    const cs = this.options();
    this.busy.set(true);
    const evaluated = this.api.evaluate(evaluateRequest(limit, claim, params, cs)).pipe(
      map((r: EvaluateResponse) => resultOf(r, claim.value, limit.direction)),
      catchError((f: ApiFailure) => of(failureOf(f))),
    );
    return forkJoin({ result: evaluated, ...this.shape(limit, params, cs) }).pipe(catchError(() => of(null)));
  }

  private shape(limit: LimitInfo, params: Slider[], cs: Chooser[]): { curve: Observable<CurveResponse | null>; surface: Observable<SurfaceResponse | null> } {
    const constraints = constraintsOf(cs);
    const none = { curve: of(null), surface: of(null) };
    const topology = topologyFor(limit);
    if (topology === 'surface') {
      const x = params.find((p) => p.id === this.xAxis());
      const z = params.find((p) => p.id === this.zAxis());
      if (!x || !z) return none;
      return {
        curve: of(null),
        surface: this.analysis
          .surface({
            limit_id: limit.id,
            constraints,
            parameters: parametersOf(params, [x.id, z.id]),
            x_parameter: x.id,
            y_parameter: z.id,
            x_range: [x.min, x.max],
            y_range: [z.min, z.max],
            samples: 30,
          })
          .pipe(catchError(() => of(null))),
      };
    }
    if (topology === 'corridor') {
      const p = params[0];
      return {
        surface: of(null),
        curve: this.analysis
          .curve({ limit_id: limit.id, constraints, parameters: {}, free_parameter: p.id, range: [p.min, p.max], points: 101, spacing: 'auto' })
          .pipe(catchError(() => of(null))),
      };
    }
    return none;
  }

  private take(a: Answer | null): void {
    this.busy.set(false);
    if (!a) return;
    this.result.set(a.result);
    this.curve.set(a.curve);
    this.surface.set(a.surface);
    // Keep the engine's bound inside the claim's slider when the parameters move it far.
    const c = this.claim();
    const b = a.result.bound;
    if (c && b !== null && b > 0 && (b > c.max || b < c.min)) this.claim.set({ ...withValue({ ...c, value: b * 3 }, b / 100), value: c.value });
    this.redraw();
  }

  private redraw(): void {
    const limit = this.limit();
    const claim = this.claim();
    if (!this.view || !this.bench || !limit || !claim) return;
    if (this.mode() === 'apparatus') {
      const data = this.benchData(limit, claim);
      const bench = this.bench;
      this.zone.runOutsideAngular(() => this.view?.clear());
      this.benchState.set(this.zone.runOutsideAngular(() => bench.draw(data)));
      this.drawn.set({ hatchedCells: 0, zones: 0 });
      return;
    }
    this.zone.runOutsideAngular(() => this.bench?.clear());
    this.benchState.set(null);
    this.drawBoundary(limit, claim, this.view);
  }

  /** The values of the physical apparatus: the sliders' places, the level, the wavelength, the pinned probe. */
  private benchData(limit: LimitInfo, claim: Slider): ApparatusData {
    const ps = this.params();
    const place = (s: Slider | undefined) => (s ? toPosition(s) / STEPS : null);
    const byId = (id: string | null) => (id ? ps.find((p) => p.id === id) : undefined);
    const spectral = spectralParameter(limit);
    const sp = byId(spectral?.id ?? null);
    const pin = this.pin();
    return {
      kind: apparatusFor(limit),
      direction: limit.direction,
      claim: claim.value,
      claimUnit: claim.base,
      result: this.result(),
      position: place(byId(apertureParameter(limit)) ?? ps[0]) ?? 0.5,
      strength: place(byId(massParameter(limit))),
      level: byId(discreteParameter(limit))?.value ?? null,
      nanometres: sp && spectral ? toNanometres(sp.value, spectral.as) : null,
      pinned: pin && pin.limitId === limit.id ? { ratio: pin.result?.ratio ?? null, impossible: pin.result?.kind === 'impossible' } : null,
    };
  }

  private drawBoundary(limit: LimitInfo, claim: Slider, view: LawView): void {
    const ps = this.params();
    const axis = (id: string) => {
      const s = ps.find((p) => p.id === id);
      if (!s) return null;
      const text = (v: number) => `${fmt(convert(v, s.base, this.unitOf(s)) ?? v)} ${unitText(this.unitOf(s))}`.trim();
      return { lo: s.min, hi: s.max, log: s.log, value: s.value, name: s.label, loText: text(s.min), hiText: text(s.max) };
    };
    const topology = topologyFor(limit);
    const xId = topology === 'surface' ? this.xAxis() : (ps[0]?.id ?? '');
    const zId = topology === 'surface' ? this.zAxis() : '';
    const pin = this.pin();
    const pinValue = (id: string) => pin?.params.find((p) => p.id === id)?.value ?? null;
    const data: ViewData = {
      topology,
      skin: skinFor(limit),
      direction: limit.direction,
      claim: claim.value,
      result: this.result(),
      x: axis(xId),
      z: axis(zId),
      curve: this.curve(),
      surface: this.surface(),
      yName: LAB.claimLabel,
      yText: (v) => this.claimText(v),
      pinned: pin && pin.limitId === limit.id ? { x: pinValue(xId), z: pinValue(zId), claim: pin.claim } : null,
    };
    this.drawn.set(this.zone.runOutsideAngular(() => view.draw(data)));
  }

  // ---------- find breaking point ----------

  protected findBreakingPoint(): void {
    const limit = this.limit();
    const claim = this.claim();
    const free = this.freeParam();
    if (!limit || !claim || !free) return;
    this.solving.set(true);
    this.solveNote.set('');
    this.analysis
      .solve({
        limit_id: limit.id,
        claim: { value: claim.value, unit: claim.base },
        constraints: constraintsOf(this.options()),
        parameters: parametersOf(this.params(), [free.id]),
        free_parameter: free.id,
      })
      .subscribe({
        next: (r) => {
          this.solving.set(false);
          this.solveNote.set(r.sentence);
          this.glide(free.id, convert(r.threshold, r.unit, free.base) ?? r.threshold);
        },
        error: (f: ApiFailure) => {
          this.solving.set(false);
          this.solveNote.set(LAB.solveFailed[f.code] ?? LAB.solveFailed['other']);
        },
      });
  }

  /** Moves a slider and the probe smoothly to `target` (at once under reduced motion). */
  private glide(id: string, target: number): void {
    cancelAnimationFrame(this.animation);
    const start = this.params().find((p) => p.id === id)?.value;
    if (start === undefined || !Number.isFinite(target)) return;
    this.params.update((ps) => ps.map((s) => (s.id === id ? { ...withValue(s, target), value: start } : s)));
    if (this.reduced) {
      this.update(id, (s) => ({ ...s, value: target }));
      return;
    }
    const t0 = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - t0) / 700);
      const eased = 1 - (1 - t) ** 3;
      this.update(id, (s) => ({ ...s, value: start + (target - start) * eased }));
      if (t < 1) this.animation = requestAnimationFrame(step);
    };
    this.animation = requestAnimationFrame(step);
  }

  // ---------- dual probe ----------

  protected pinProbe(): void {
    const limit = this.limit();
    const claim = this.claim();
    if (!limit || !claim) return;
    this.pin.set({ limitId: limit.id, params: this.params(), claim: claim.value, result: this.result() });
    this.redraw();
  }

  protected clearPin(): void {
    this.pin.set(null);
    this.redraw();
  }

  // ---------- view ----------

  protected setMode(mode: 'apparatus' | 'boundary'): void {
    if (this.mode() === mode) return;
    this.mode.set(mode);
    this.redraw();
  }

  /** Arrow keys move between the two views of the segmented choice (radio group pattern). */
  protected onModeKey(event: KeyboardEvent): void {
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) return;
    event.preventDefault();
    const next = this.mode() === 'apparatus' ? 'boundary' : 'apparatus';
    this.setMode(next);
    const host = (event.currentTarget as HTMLElement).closest('.lab-segments')?.querySelector<HTMLElement>(`[data-mode='${next}']`);
    host?.focus();
  }

  protected onKey(event: KeyboardEvent): void {
    if (this.observatory?.onKey(event.key)) event.preventDefault();
  }

  protected resetView(): void {
    this.observatory?.resetView();
  }

  protected back(): void {
    void this.router.navigateByUrl('/');
  }

  ngOnDestroy(): void {
    cancelAnimationFrame(this.animation);
    this.view?.dispose();
    this.view = null;
    this.bench?.dispose();
    this.bench = null;
    this.observatory?.dispose();
    this.observatory = null;
  }
}

