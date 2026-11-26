import { A11yModule } from '@angular/cdk/a11y';
import { Component, ElementRef, HostListener, computed, inject, signal, viewChild } from '@angular/core';
import { LucideAngularModule, ChevronDown, ChevronLeft, ChevronRight, Copy, Download, ImageUp, X } from 'lucide-angular';
import { Observable, firstValueFrom } from 'rxjs';
import { AnalyzeResponse, AnalyzedClaim } from './analysis.models';
import { AnalysisService } from './analysis.service';
import { ApiFailure, EvaluateResponse, LimitInfo } from './api.models';
import { ApiService } from './api.service';
import { CarnotApparatus } from './carnot-apparatus';
import { CopApparatus, TwinVerdicts } from './cop-apparatus';
import { CountUp } from './count-up';
import { formatValue } from './chart-geometry';
import { Explorer } from './explorer';
import { Gauge } from './gauge';
import {
  Entry,
  NoVerdict,
  ReadingSentence,
  VerdictKind,
  noVerdict,
  plainStep,
  readingSentences,
  shortLabel,
  boundText,
  claimValueText,
  entryTitle,
  filtered,
  plainSentence,
  ratioText,
  reviewMarkdown,
  step,
  summary,
  verdictKind,
  verdictText,
} from './review-logic';
import { SAMPLES, Sample, evaluateRequest, sampleClaims, sampleOf } from './samples';
import { StatusService } from './status.service';
import { EXPLAIN, STRINGS, UI, WB } from './strings';
import { ToastService } from './toast.service';
import { Paragraph, badge, explain, paragraphs, rememberDirection, shiftClaim, solveSentence } from './workbench-logic';

export { ILLUSTRATIVE_TEXT } from './samples';
export type { Entry } from './review-logic';

/** Answers tried per paragraph when the model answer is unusable. */
export const ATTEMPTS = 3;

/** A line of the run log, shown in the trace drawer. */
export interface LogLine {
  text: string;
  status: 'running' | 'done' | 'failed';
}

/** A paragraph of the reading panel, cut into sentences. */
interface ReadingParagraph {
  sentences: ReadingSentence[];
}

/** State of one step of the stepper, set only by pipeline events. */
export type StepState = 'waiting' | 'active' | 'done';

/**
 * The analysis screen. Input: a hero with the text card and samples.
 * Running: a compact bar, a stepper and claim cards streaming in.
 * Reviewed: a summary strip, a claim list, the text with each claim
 * highlighted and the instrument card of the selected claim. The verdicts
 * and numbers come from the API; this component only lays them out.
 */
@Component({
  selector: 'app-workbench',
  imports: [Explorer, Gauge, CarnotApparatus, CopApparatus, CountUp, LucideAngularModule, A11yModule],
  templateUrl: './workbench.html',
  host: { '[attr.data-state]': 'state()' },
})
export class Workbench {
  private readonly analysis = inject(AnalysisService);
  private readonly api = inject(ApiService);
  private readonly toast = inject(ToastService);
  protected readonly status = inject(StatusService);
  protected readonly s = WB;
  protected readonly u = UI;
  protected readonly samples = SAMPLES;
  protected readonly icons = { ChevronDown, ChevronLeft, ChevronRight, Copy, Download, ImageUp, X };

  readonly text = signal(SAMPLES[0].text);
  readonly running = signal(false);
  readonly analysed = signal(false);
  readonly entries = signal<Entry[]>([]);
  readonly selected = signal(0);
  readonly howOpen = signal(false);
  protected readonly log = signal<LogLine[]>([]);
  protected readonly parts = signal<Paragraph[]>([]);
  protected readonly answered = signal(0);
  /** Reading, Identifying laws, Researching sources, Computing: each set by a real event. */
  readonly steps = signal<StepState[]>(['waiting', 'waiting', 'waiting', 'waiting']);
  /** Values typed for the missing quantities of the selected claim. */
  protected readonly supplied = signal<Record<string, string>>({});
  protected readonly reevaluating = signal(false);
  protected readonly filter = signal<VerdictKind | null>(null);
  protected readonly sourceOpen = signal(false);
  protected readonly analysedAt = signal('');
  protected readonly limits = signal<LimitInfo[]>([]);
  protected readonly solved = signal<Record<number, string>>({});
  protected readonly twins = signal<Record<number, TwinVerdicts>>({});
  private readonly howDrawer = viewChild<ElementRef<HTMLElement>>('how');

  readonly state = computed<'input' | 'running' | 'reviewed'>(() =>
    this.running() ? 'running' : this.analysed() ? 'reviewed' : 'input',
  );
  protected readonly entry = computed(() => this.entries()[this.selected()] ?? null);
  protected readonly claim = computed(() => {
    const e = this.entry();
    return e?.kind === 'claim' ? e.claim : null;
  });
  protected readonly limit = computed(() => {
    const id = this.claim()?.limit_id;
    return this.limits().find((l) => l.id === id) ?? null;
  });
  protected readonly explanation = computed(() => {
    const c = this.claim();
    return c ? explain(c).filter((l) => l === EXPLAIN.aiDraft) : [];
  });
  protected readonly board = computed(() => summary(this.entries(), this.parts().length));
  protected readonly visible = computed(() => filtered(this.entries(), this.filter()));
  protected readonly reading = computed<ReadingParagraph[]>(() => {
    const claims = this.entries().flatMap((e, index) => (e.kind === 'claim' ? [{ index, claim: e.claim }] : []));
    return this.parts().map((p) => ({
      sentences: readingSentences(p.text, claims.map((c) => ({ ...c, claim: shiftClaim(c.claim, -p.start) }))),
    }));
  });
  /** What the instrument draws: the gauge (a Max law with no parameter), the explorer, or nothing. */
  protected readonly visual = computed<'apparatus' | 'gauge' | 'explorer' | 'none'>(() => {
    const c = this.claim();
    const l = this.limit();
    if (!c || !l || !c.limit_id || !c.result || c.result.verdict === 'INSUFFICIENT_DATA') return 'none';
    if (c.limit_id === 'carnot' || c.limit_id === 'cop') return 'apparatus';
    if (l.parameters.length === 0) return l.direction === 'Max' && c.value !== null && c.result.bounds ? 'gauge' : 'none';
    return 'explorer';
  });
  protected readonly gaugeBounds = computed(() => {
    const b = this.claim()?.result?.bounds;
    return b ? Array.from(new Set([b.permissive.b_max, b.restrictive.b_max])) : [];
  });
  protected readonly charCount = computed(() => Array.from(this.text()).length);

  constructor() {
    this.loadLimits();
  }

  private loadLimits(): Promise<void> {
    return new Promise((resolve) => {
      this.api.getLimits().subscribe({
        next: (r) => {
          for (const l of r.limits) rememberDirection(l.id, l.direction === 'Min' ? 'Min' : 'Max');
          this.limits.set(r.limits);
          resolve();
        },
        error: () => resolve(),
      });
    });
  }

  private direction(e: Entry): 'Max' | 'Min' | 'Equal' {
    if (e.kind !== 'claim') return 'Max';
    return this.limits().find((l) => l.id === e.claim.limit_id)?.direction ?? 'Max';
  }

  protected kindOf(i: number): VerdictKind {
    const e = this.entries()[i];
    return e ? verdictKind(e) : 'needs';
  }
  protected labelOf(i: number): string {
    const e = this.entries()[i];
    return e ? verdictText(e) : '';
  }
  protected sentenceOf(i: number): string {
    const e = this.entries()[i];
    return e ? plainSentence(e, this.direction(e)) : '';
  }
  protected stepLine(t: { step: string; status: string }): string {
    return plainStep(t);
  }
  protected titleOf(e: Entry): string {
    return entryTitle(e);
  }
  protected quantityOf(e: Entry): string {
    return e.kind === 'claim' ? e.claim.quantity : `${WB.progressParagraph} ${e.paragraph}`;
  }
  protected sci(v: number): string {
    return formatValue(v);
  }
  protected percent(f: number): string {
    return `${formatValue(f * 100)} %`;
  }
  protected chipOf(e: Entry): string {
    return shortLabel(e);
  }
  protected isVisible(i: number): boolean {
    return this.visible().includes(i);
  }
  protected claimText(c: AnalyzedClaim): string {
    return claimValueText(c);
  }
  protected bound(c: AnalyzedClaim): string {
    return boundText(c);
  }
  protected ratio(c: AnalyzedClaim): string {
    return ratioText(c) ?? UI.metricNone;
  }
  protected badgeOf(c: AnalyzedClaim): string {
    return badge(c);
  }
  protected classOf(c: AnalyzedClaim): string {
    const id = this.limits().find((l) => l.id === c.limit_id)?.classification;
    return id ? (STRINGS.classification[id] ?? id) : '';
  }
  protected number(v: number | null | undefined): string {
    return v === null || v === undefined ? '' : String(v);
  }
  protected paramText(c: AnalyzedClaim, id: string): string {
    const v = c.parameters[id]?.value;
    return v === null || v === undefined ? '' : String(v);
  }
  protected twinOf(i: number): TwinVerdicts {
    const t: TwinVerdicts | undefined = this.twins()[i];
    return t ?? { heating: null, cooling: null };
  }
  protected stepState(i: number): StepState {
    return this.steps()[i] ?? 'waiting';
  }

  private setStep(i: number, s: StepState): void {
    this.steps.update((all) => all.map((x, k) => (k === i ? s : x)));
  }

  /** The card of a claim without a verdict; null for a claim with one. */
  protected readonly card = computed<NoVerdict | null>(() => {
    const e = this.entry();
    return e ? noVerdict(e, this.limits()) : null;
  });

  protected supply(id: string, raw: string): void {
    this.supplied.update((m) => ({ ...m, [id]: raw }));
  }

  protected canReevaluate(card: NoVerdict): boolean {
    const s = this.supplied();
    return card.missing.length > 0 && card.missing.every((m) => Number.isFinite(Number(s[m.id])) && (s[m.id] ?? '').trim() !== '');
  }

  /** POST /v1/evaluate with the traced parameters and the values the user typed; the engine decides. */
  protected async reevaluate(card: NoVerdict): Promise<void> {
    const i = this.selected();
    const e = this.entries()[i];
    if (e?.kind !== 'claim' || !e.claim.limit_id || e.claim.value === null || !this.canReevaluate(card)) return;
    const c = e.claim;
    const limitId = c.limit_id;
    const value = c.value;
    if (limitId === null || value === null) return;
    const s = this.supplied();
    const parameters: Record<string, { value: number; unit: string }> = {};
    for (const [id, p] of Object.entries(c.parameters)) if (p.value !== null) parameters[id] = { value: p.value, unit: p.unit };
    for (const m of card.missing) parameters[m.id] = { value: Number(s[m.id]), unit: m.unit };
    this.reevaluating.set(true);
    try {
      const result = await firstValueFrom(
        this.api.evaluate({
          limit_id: limitId,
          claim: { value, unit: c.unit },
          constraints: Object.fromEntries(Object.entries(c.constraints).map(([a, o]) => [a, [o]])),
          parameters,
        }),
      );
      const added = Object.fromEntries(
        card.missing.map((m) => [m.id, { value: Number(s[m.id]), raw_value: Number(s[m.id]), unit: m.unit, evidence: null, flags: [UI.supplied] }]),
      );
      const claim: AnalyzedClaim = {
        ...c,
        parameters: { ...c.parameters, ...added },
        status: 'evaluated',
        result,
        reason: null,
        trace: [...c.trace, { step: 'evaluation', status: 'done', detail: `${UI.suppliedTrace(card.missing.map((m) => m.id).join(', '))}; engine verdict ${result.verdict}` }],
      };
      this.entries.update((all) => all.map((x, k) => (k === i ? { ...e, claim } : x)));
      this.solved.update((m) => {
        const n = { ...m };
        delete n[i];
        return n;
      });
      this.supplied.set({});
      void this.enrich(i);
    } catch (err) {
      const f = err as ApiFailure;
      this.toast.show(f.code === 'NetworkError' ? UI.toastNetwork : UI.checkFailed, 'error');
    } finally {
      this.reevaluating.set(false);
    }
  }
  protected pending(): number[] {
    const n = Math.min(3, Math.max(0, this.parts().length - this.answered()));
    return Array.from({ length: n }, (_, i) => i);
  }

  select(i: number): void {
    if (i < 0 || i >= this.entries().length) return;
    this.selected.set(i);
    void this.enrich(i);
  }

  protected move(by: 1 | -1): void {
    this.select(step(this.visible(), this.selected(), by));
  }

  protected setFilter(kind: VerdictKind | null): void {
    this.filter.set(this.filter() === kind ? null : kind);
    const v = this.visible();
    if (v.length && !v.includes(this.selected())) this.select(v[0]);
  }

  protected useSample(s: Sample): void {
    this.text.set(s.text);
  }

  toggleHow(): void {
    if (this.howOpen()) this.closeHow();
    else this.openHow();
  }
  protected openHow(): void {
    this.howOpen.set(true);
    setTimeout(() => this.howDrawer()?.nativeElement.focus());
  }
  protected closeHow(): void {
    this.howOpen.set(false);
  }

  newAnalysis(): void {
    this.analysed.set(false);
    this.howOpen.set(false);
    this.filter.set(null);
  }

  @HostListener('document:keydown.escape')
  protected onEscape(): void {
    this.closeHow();
  }

  /** J / K and the arrow keys move between claims, outside form controls. */
  @HostListener('document:keydown', ['$event'])
  protected onKey(event: KeyboardEvent): void {
    if (this.state() !== 'reviewed' || this.howOpen() || event.ctrlKey || event.metaKey || event.altKey) return;
    const t = event.target as HTMLElement | null;
    if (t && (/^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName) || t.isContentEditable)) return;
    if (t instanceof Element && t.closest('.cdk-overlay-container')) return;
    const k = event.key;
    if (k === 'j' || k === 'J' || k === 'ArrowDown' || k === 'ArrowRight') this.move(1);
    else if (k === 'k' || k === 'K' || k === 'ArrowUp' || k === 'ArrowLeft') this.move(-1);
    else return;
    event.preventDefault();
  }

  /** The review as Markdown, built and downloaded in the browser. */
  download(): void {
    const md = reviewMarkdown(this.entries(), this.parts().length, this.limits(), this.analysedAt());
    const url = URL.createObjectURL(new Blob([md], { type: 'text/markdown;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = UI.reportFile;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url));
    this.toast.show(UI.downloaded);
  }

  protected copyQuote(text: string): void {
    void navigator.clipboard?.writeText(text).then(
      () => this.toast.show(UI.copied),
      () => undefined,
    );
  }

  private push(line: LogLine): number {
    this.log.update((l) => [...l, line]);
    return this.log().length - 1;
  }

  private settle(index: number, status: LogLine['status'], text?: string): void {
    this.log.update((l) => l.map((x, i) => (i === index ? { text: text ?? x.text, status } : x)));
  }

  /** Reads an image dropped or pasted on the input, through the vision model. */
  protected async takeImage(file: File | null | undefined, event: Event): Promise<void> {
    if (!file || !file.type.startsWith('image/')) return;
    event.preventDefault();
    if (!this.status.vision()) {
      this.toast.show(WB.inputNoImage, 'error');
      return;
    }
    const base64 = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result).replace(/^data:[^,]*,/, ''));
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
    const line = this.push({ text: WB.imageRead, status: 'running' });
    try {
      const r = await firstValueFrom(this.analysis.transcribe(file.type, base64));
      this.text.set(r.text);
      this.settle(line, 'done', WB.imageRead);
    } catch {
      this.settle(line, 'failed', WB.imageFailed);
      this.toast.show(UI.toastImage, 'error');
    }
  }

  protected onDrop(event: DragEvent): void {
    void this.takeImage(event.dataTransfer?.files?.[0], event);
  }

  protected onPaste(event: ClipboardEvent): void {
    const item = Array.from(event.clipboardData?.items ?? []).find((i) => i.type.startsWith('image/'));
    if (item) void this.takeImage(item.getAsFile(), event);
  }

  private async analyzeOnce(text: string): Promise<AnalyzeResponse> {
    return firstValueFrom(this.analysis.analyze(text) as Observable<AnalyzeResponse>);
  }

  private paragraphOf(start: number): number {
    const parts = this.parts();
    let k = 0;
    parts.forEach((p, i) => {
      if (p.start <= start) k = i;
    });
    return k + 1;
  }

  /** Runs the pipeline: each paragraph through POST /v1/analyze, or a built-in sample through the engine. */
  async run(): Promise<void> {
    if (this.running() || !this.text().trim()) return;
    this.running.set(true);
    this.entries.set([]);
    this.solved.set({});
    this.twins.set({});
    this.selected.set(0);
    this.filter.set(null);
    this.sourceOpen.set(false);
    this.answered.set(0);
    this.supplied.set({});
    this.setStep(0, 'active');
    this.setStep(1, 'waiting');
    this.setStep(2, 'waiting');
    this.setStep(3, 'waiting');
    this.log.set(this.log().filter((l) => l.text.startsWith(WB.imageRead)));
    const text = this.text();
    this.parts.set(paragraphs(text));
    const sample = sampleOf(text);
    if (sample && !this.status.extraction()) await this.runSample(sample);
    else await this.runModel();
    // Every paragraph is answered: the server has read the text and identified each claim's law.
    this.setStep(0, 'done');
    this.setStep(1, 'done');
    this.setStep(2, 'active');
    await this.loadLimits();
    this.setStep(2, 'done');
    this.setStep(3, 'active');
    if (this.entries().length) await this.enrich(0);
    this.setStep(3, 'done');
    this.analysedAt.set(new Date().toISOString().slice(0, 16).replace('T', ' ') + ' UTC');
    this.running.set(false);
    this.analysed.set(true);
  }

  private async runModel(): Promise<void> {
    const parts = this.parts();
    for (const [k, p] of parts.entries()) {
      const head = `${WB.progressParagraph} ${k + 1} ${WB.progressOf} ${parts.length}`;
      const line = this.push({ text: head, status: 'running' });
      let attempt = 0;
      for (;;) {
        attempt += 1;
        try {
          const r = await this.analyzeOnce(p.text);
          const claims = r.claims.map((c) => shiftClaim(c, p.start));
          // An answer with claims carries the server's law identification (its 'limit' step).
          if (claims.some((c) => c.trace.some((t) => t.step === 'limit'))) this.setStep(1, 'active');
          this.entries.update((e) => [...e, ...claims.map((claim) => ({ kind: 'claim' as const, claim, paragraph: k + 1 }))]);
          this.settle(line, 'done', `${head}: ${UI.paragraphRead(claims.length)}`);
          break;
        } catch (e) {
          const f = e as ApiFailure;
          if (f.code === 'ExtractionFailed' && attempt < ATTEMPTS) {
            this.settle(line, 'running', `${head}: ${WB.progressRetry} (${attempt})`);
            continue;
          }
          if (f.code === 'NetworkError') this.status.markOffline();
          this.entries.update((x) => [...x, { kind: 'failure', paragraph: k + 1, text: p.text, failure: f, attempts: attempt }]);
          this.settle(line, 'failed', `${head}: ${UI.paragraphNotRead}`);
          this.toast.show(UI.toastParagraph(k + 1), 'error');
          break;
        }
      }
      this.answered.set(k + 1);
    }
  }

  /** A built-in sample: its quantities go straight to POST /v1/evaluate, no language model. */
  private async runSample(sample: Sample): Promise<void> {
    const line = this.push({ text: UI.sampleTrace, status: 'running' });
    const results: (EvaluateResponse | null)[] = [];
    for (const c of sample.claims) {
      const req = evaluateRequest(c);
      let r: EvaluateResponse | null = null;
      if (req) {
        try {
          r = await firstValueFrom(this.api.evaluate(req));
        } catch (e) {
          const f = e as ApiFailure;
          if (f.code === 'NetworkError') this.status.markOffline();
          this.toast.show(`${c.quantity}: ${UI.checkFailed}`, 'error');
        }
      }
      results.push(r);
      const claims = sampleClaims(sample, results);
      const claim = claims[results.length - 1];
      this.entries.update((e) => [...e, { kind: 'claim', claim, paragraph: this.paragraphOf(claim.evidence?.start ?? 0) }]);
      this.answered.set(this.paragraphOf(claim.evidence?.start ?? 0));
    }
    this.answered.set(this.parts().length);
    this.settle(line, 'done');
  }

  /** The solver sentence and, for COP without a mode, the verdict of each mode. */
  private async enrich(i: number): Promise<void> {
    const e = this.entries()[i];
    if (e?.kind !== 'claim') return;
    const c = e.claim;
    const verdict = c.result?.verdict;
    if (!c.limit_id || c.value === null || this.solved()[i] !== undefined) return;
    this.solved.update((m) => ({ ...m, [i]: '' }));
    const known = Object.entries(c.parameters).filter(([, p]) => p.value !== null);
    const constraints = Object.fromEntries(Object.entries(c.constraints).map(([a, o]) => [a, [o]]));
    if (verdict === 'IMPOSSIBLE' || verdict === 'CONDITIONAL') {
      for (const [free] of known) {
        const parameters = Object.fromEntries(
          known.filter(([id]) => id !== free).map(([id, p]) => [id, { value: p.value as number, unit: p.unit }]),
        );
        try {
          const r = await firstValueFrom(
            this.analysis.solve({ limit_id: c.limit_id, claim: { value: c.value, unit: c.unit }, constraints, parameters, free_parameter: free }),
          );
          this.solved.update((m) => ({ ...m, [i]: solveSentence(r.sentence) }));
          break;
        } catch {
          continue;
        }
      }
    }
    if (c.limit_id === 'cop' && !c.constraints['mode']) {
      const parameters = Object.fromEntries(known.map(([id, p]) => [id, { value: p.value as number, unit: p.unit }]));
      const twin: TwinVerdicts = { heating: null, cooling: null };
      for (const mode of ['heating', 'cooling'] as const) {
        try {
          const r = await firstValueFrom(
            this.api.evaluate({ limit_id: 'cop', claim: { value: c.value, unit: c.unit }, constraints: { mode: [mode] }, parameters }),
          );
          twin[mode] = r.verdict;
        } catch {
          twin[mode] = null;
        }
      }
      this.twins.update((m) => ({ ...m, [i]: twin }));
    }
  }
}
