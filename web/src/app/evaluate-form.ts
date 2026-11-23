import { Component, DestroyRef, computed, inject, input, signal } from '@angular/core';
import { Subscription } from 'rxjs';
import {
  ApiFailure,
  AssumptionInfo,
  EvaluateRequest,
  EvaluateResponse,
  LimitInfo,
  ParameterSpec,
  ParameterValue,
} from './api.models';
import { ApiService } from './api.service';
import { CarnotApparatus } from './carnot-apparatus';
import { CopApparatus, TwinVerdicts } from './cop-apparatus';
import { CopMode } from './cop-geometry';
import { parseDecimal } from './decimal';
import { EXAMPLES, Example } from './examples';
import { EvaluateClaim, ResultPanel } from './result-panel';
import { STRINGS } from './strings';

/** Parses a finite number from a text field, with a decimal point or comma; null when empty or invalid. */
const parseNumber = parseDecimal;

/** Pause after the last change before a live re-evaluation. */
const LIVE_DEBOUNCE_MS = 300;

/** The assumption set by the Betz guided recovery. */
const REFERENCE_AREA = 'reference_area';

/** The COP assumption set by the mode control on the figure. */
const COP_MODE = 'mode';

@Component({
  selector: 'app-evaluate-form',
  imports: [ResultPanel, CarnotApparatus, CopApparatus],
  templateUrl: './evaluate-form.html',
})
export class EvaluateForm {
  readonly limits = input<LimitInfo[]>([]);

  private readonly api = inject(ApiService);
  protected readonly s = STRINGS;
  protected readonly examples = EXAMPLES;

  private readonly selectedId = signal<string | null>(null);
  protected readonly limit = computed<LimitInfo | null>(() => {
    const all = this.limits();
    return all.find((l) => l.id === this.selectedId()) ?? all[0] ?? null;
  });

  protected readonly claimValue = signal('');
  protected readonly claimUnit = computed(() => this.limit()?.claim_unit ?? '');
  protected readonly uncertainty = signal('');
  private readonly choices = signal<Record<string, string>>({});
  private readonly paramValues = signal<Record<string, string>>({});
  private readonly paramUnits = signal<Record<string, string>>({});

  protected readonly fieldError = signal<string | null>(null);
  protected readonly busy = signal(false);
  protected readonly result = signal<EvaluateResponse | null>(null);
  protected readonly failure = signal<ApiFailure | null>(null);
  protected readonly evaluatedClaim = signal<EvaluateClaim | null>(null);
  protected readonly evaluatedLimit = signal<LimitInfo | null>(null);
  protected readonly evaluatedConstraints = signal<Record<string, string[]>>({});
  /** Index of the open case in EXAMPLES, as the select value; '' when none. */
  protected readonly openCaseIndex = signal('');
  /** The apparatus drawn on the bench for the selected limit, if it has one. */
  protected readonly apparatus = computed<'carnot' | 'cop' | null>(() => {
    const id = this.limit()?.id;
    return id === 'carnot' || id === 'cop' ? id : null;
  });
  protected readonly hasApparatus = computed(() => this.apparatus() !== null);
  /** COP mode as set on the figure; '' when not stated. */
  protected readonly copMode = computed<'' | CopMode>(() => {
    const m = this.choice(COP_MODE);
    return m === 'heating' || m === 'cooling' ? m : '';
  });
  /** Engine verdicts for each COP mode, requested when the mode is not stated. */
  protected readonly twinVerdicts = signal<TwinVerdicts>({ heating: null, cooling: null });
  protected readonly twinsRequested = signal(false);
  private twinRequests: Subscription[] = [];
  /** The claim unit is shown only when the quantity has one. */
  protected readonly showClaimUnit = computed(() => {
    const unit = this.claimUnit();
    return unit !== '' && unit !== 'dimensionless';
  });
  private pending: Subscription | null = null;
  private liveTimer: ReturnType<typeof setTimeout> | null = null;
  private hasEvaluated = false;

  constructor() {
    inject(DestroyRef).onDestroy(() => {
      this.cancelLive();
      this.cancelTwins();
    });
  }

  protected choice(id: string): string {
    return this.choices()[id] ?? '';
  }

  protected paramValue(id: string): string {
    return this.paramValues()[id] ?? '';
  }

  protected paramUnit(p: ParameterSpec): string {
    return this.paramUnits()[p.id] ?? p.units[0] ?? '';
  }

  /** Unit of a declared parameter, from the metadata. */
  protected unitOf(limit: LimitInfo, id: string): string {
    const p = limit.parameters.find((q) => q.id === id);
    return p ? this.paramUnit(p) : '';
  }

  /** Assumptions shown in the settings; those set on the apparatus are left out. */
  protected settingsAssumptions(limit: LimitInfo): AssumptionInfo[] {
    return this.apparatus() === 'cop' ? limit.assumptions.filter((a) => a.id !== COP_MODE) : limit.assumptions;
  }

  /** Template handler: the mode control on the COP figure. */
  protected changeMode(mode: '' | CopMode): void {
    this.changeChoice(COP_MODE, mode);
  }

  private cancelTwins(): void {
    for (const s of this.twinRequests) s.unsubscribe();
    this.twinRequests = [];
  }

  /**
   * With the COP mode not stated, asks the engine for each mode separately,
   * so the two small figures show engine verdicts. Otherwise clears them.
   */
  private evaluateTwins(limit: LimitInfo, request: EvaluateRequest): void {
    this.cancelTwins();
    this.twinVerdicts.set({ heating: null, cooling: null });
    const wanted = limit.id === 'cop' && !request.constraints[COP_MODE];
    this.twinsRequested.set(wanted);
    if (!wanted) return;
    for (const mode of ['heating', 'cooling'] as const) {
      const twin: EvaluateRequest = { ...request, constraints: { ...request.constraints, [COP_MODE]: [mode] } };
      this.twinRequests.push(
        this.api.evaluate(twin).subscribe({
          next: (r) => this.twinVerdicts.update((v) => ({ ...v, [mode]: r.verdict })),
          error: () => this.twinVerdicts.update((v) => ({ ...v, [mode]: null })),
        }),
      );
    }
  }

  protected selectLimit(id: string): void {
    this.cancelLive();
    this.cancelTwins();
    this.twinVerdicts.set({ heating: null, cooling: null });
    this.twinsRequested.set(false);
    this.hasEvaluated = false;
    this.pending?.unsubscribe();
    this.busy.set(false);
    this.selectedId.set(id);
    this.choices.set({});
    this.paramValues.set({});
    this.paramUnits.set({});
    this.result.set(null);
    this.failure.set(null);
    this.evaluatedClaim.set(null);
    this.evaluatedLimit.set(null);
    this.evaluatedConstraints.set({});
    this.fieldError.set(null);
  }

  /** Template handler: a user change of an assumption. */
  protected changeChoice(id: string, option: string): void {
    this.setChoice(id, option);
    this.scheduleLive();
  }

  /** Template handler: a user change of the claim value. */
  protected changeClaim(value: string): void {
    this.claimValue.set(value);
    this.scheduleLive();
  }

  private setChoice(id: string, option: string): void {
    this.choices.update((c) => ({ ...c, [id]: option }));
  }

  protected setParamValue(id: string, value: string): void {
    this.paramValues.update((c) => ({ ...c, [id]: value }));
  }

  /** Template handler: a user change of a parameter, from a field or a thermometer. */
  protected changeParam(id: string, value: string): void {
    this.setParamValue(id, value);
    this.scheduleLive();
  }

  /** Template handler: the "Open a case" select. */
  protected openCase(index: string): void {
    const example = EXAMPLES[Number(index)];
    if (index === '' || !example) return;
    this.applyExample(example);
    this.openCaseIndex.set(index);
  }

  /** Template handler: a user change of the limit, which closes the open case. */
  protected changeLimit(id: string): void {
    this.openCaseIndex.set('');
    this.selectLimit(id);
  }

  protected setParamUnit(id: string, unit: string): void {
    this.paramUnits.update((c) => ({ ...c, [id]: unit }));
  }

  protected submit(event: Event): void {
    event.preventDefault();
    this.evaluate();
  }

  /** Fills the form with an example and evaluates it. */
  protected applyExample(example: Example): void {
    if (!this.limits().some((l) => l.id === example.limitId)) return;
    this.selectLimit(example.limitId);
    this.claimValue.set(example.claim);
    this.uncertainty.set('');
    this.choices.set({ ...example.constraints });
    this.paramValues.set({ ...example.parameters });
    this.evaluate();
  }

  /** Guided recovery: sets the reference area and evaluates again. */
  protected recover(option: string): void {
    this.setChoice(REFERENCE_AREA, option);
    this.evaluate();
  }

  /** Debounced re-evaluation, only once a first evaluation has been requested. */
  private scheduleLive(): void {
    if (!this.hasEvaluated) return;
    this.cancelLive();
    this.liveTimer = setTimeout(() => {
      this.liveTimer = null;
      this.evaluate(true);
    }, LIVE_DEBOUNCE_MS);
  }

  private cancelLive(): void {
    if (this.liveTimer !== null) clearTimeout(this.liveTimer);
    this.liveTimer = null;
  }

  /** quiet is a live re-evaluation: an invalid form is skipped without an error, the old result stays until the new one. */
  private evaluate(quiet = false): void {
    const limit = this.limit();
    if (!limit) return;
    const request = this.buildRequest(limit);
    if (!request) {
      if (quiet) this.fieldError.set(null);
      return;
    }
    this.cancelLive();
    this.hasEvaluated = true;
    this.pending?.unsubscribe();
    this.busy.set(true);
    if (!quiet) {
      this.result.set(null);
      this.failure.set(null);
    }
    this.pending = this.api.evaluate(request).subscribe({
      next: (r) => {
        this.evaluatedClaim.set({ value: request.claim.value, unit: request.claim.unit });
        this.evaluatedLimit.set(limit);
        this.evaluatedConstraints.set(request.constraints);
        this.failure.set(null);
        this.result.set(r);
        this.busy.set(false);
      },
      error: (f: ApiFailure) => {
        this.result.set(null);
        this.failure.set(f);
        this.busy.set(false);
      },
    });
    this.evaluateTwins(limit, request);
  }

  /** Builds the request from the form; sets `fieldError` and returns null when invalid. */
  private buildRequest(limit: LimitInfo): EvaluateRequest | null {
    const value = parseNumber(this.claimValue());
    if (value === null) {
      this.fieldError.set(
        this.claimValue().trim() === '' ? STRINGS.formClaimRequired : STRINGS.formInvalidNumber,
      );
      return null;
    }
    const uncertainty = parseNumber(this.uncertainty());
    if (this.uncertainty().trim() !== '' && uncertainty === null) {
      this.fieldError.set(STRINGS.formInvalidNumber);
      return null;
    }
    const parameters: Record<string, ParameterValue> = {};
    for (const p of limit.parameters) {
      const text = this.paramValue(p.id);
      const n = parseNumber(text);
      if (text.trim() !== '' && n === null) {
        this.fieldError.set(STRINGS.formInvalidNumber);
        return null;
      }
      if (n !== null) parameters[p.id] = { value: n, unit: this.paramUnit(p) };
    }
    const constraints: Record<string, string[]> = {};
    for (const a of limit.assumptions) {
      const chosen = this.choice(a.id);
      if (chosen !== '') constraints[a.id] = [chosen];
    }
    this.fieldError.set(null);
    return {
      limit_id: limit.id,
      claim: {
        value,
        unit: this.claimUnit(),
        ...(uncertainty !== null ? { uncertainty } : {}),
      },
      constraints,
      parameters,
    };
  }
}
