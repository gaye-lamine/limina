import { Component, DestroyRef, computed, inject, input, output, signal } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { Observable, Subject, catchError, debounceTime, forkJoin, map, of, switchMap } from 'rxjs';
import { ApiFailure, EvaluateRequest, EvaluateResponse, LimitInfo } from '../api.models';
import { ApiService } from '../api.service';
import { LAB } from '../strings';
import {
  CASCADE_MAX,
  CASCADE_MIN,
  CascadeMember,
  LOAD_MAX,
  LOAD_MIN,
  addMember,
  canJoin,
  firstBreaking,
  removeMember,
  requestsAtLoad,
} from './cascade';
import { Inspect } from './inspect';
import { LabResult, choosers, constraintsOf, decisiveBound, failureOf, fmt, resultOf } from './lab-model';
import { convert } from './lab-units';

/** The current law of the laboratory, as the stress test takes it. */
export interface CurrentLaw {
  limit: LimitInfo;
  request: EvaluateRequest;
  bound: number | null;
}

/**
 * System stress test (spec laboratory-3d, R-17): two to four laws, one
 * System load slider, the engine's verdict and ratio of each law at that
 * load, and the first law past its limit labelled "First breaking limit".
 * Choosing a law shows it in the 3D view; the stress test stays.
 */
@Component({
  selector: 'app-lab-cascade',
  templateUrl: './lab-cascade.html',
})
export class LabCascade {
  protected readonly t = LAB.cascade;
  protected readonly min = LOAD_MIN;
  protected readonly max = LOAD_MAX;
  protected readonly fmt = fmt;
  private readonly api = inject(ApiService);
  private readonly destroyRef = inject(DestroyRef);

  /** The law shown in 3D, and the way to add it. */
  readonly current = input<CurrentLaw | null>(null);
  readonly shownId = input<string>('');
  /** Asks the page to show a law in 3D with this member's values. */
  readonly openLaw = output<Inspect>();
  /** A law has just become the first to break. */
  readonly breaking = output<string>();

  protected readonly members = signal<CascadeMember[]>([]);
  protected readonly load = signal(100);
  protected readonly results = signal<(LabResult | null)[]>([]);
  private readonly changes = new Subject<void>();
  private lastFirst: string | null = null;

  protected readonly first = computed(() => firstBreaking(this.members(), this.results()));
  protected readonly active = computed(() => this.members().length >= CASCADE_MIN);
  protected readonly canAdd = computed(() => {
    const c = this.current();
    return !!c && canJoin(c.limit) && this.members().length < CASCADE_MAX && !this.members().some((m) => m.limitId === c.limit.id);
  });
  protected readonly addNote = computed(() => {
    const c = this.current();
    if (c && !canJoin(c.limit)) return this.t.equal;
    if (this.members().length >= CASCADE_MAX) return this.t.full;
    return '';
  });

  constructor() {
    this.changes
      .pipe(
        debounceTime(120),
        switchMap(() => this.ask()),
        takeUntilDestroyed(this.destroyRef),
      )
      .subscribe((r) => this.take(r));
  }

  /** Adds the current law, at its current claim, values and assumptions. */
  protected addCurrent(): void {
    const c = this.current();
    if (!c || !this.canAdd()) return;
    const direction = c.limit.direction === 'Min' ? 'Min' : 'Max';
    const member: CascadeMember = { limitId: c.limit.id, title: c.limit.title, direction, request: c.request, bound: c.bound };
    this.members.update((list) => addMember(list, member));
    if (member.bound === null) this.fetchBound(member);
    else this.changes.next();
  }

  /** Starts a stress test from analysed claims: each law at its claimed value, assumptions defaulted where unstated. */
  start(claims: Inspect[], limits: LimitInfo[]): void {
    let list: CascadeMember[] = [];
    for (const i of claims) {
      const limit = limits.find((l) => l.id === i.limit_id);
      if (!limit || !canJoin(limit)) continue;
      const value = convert(i.claim.value, i.claim.unit, limit.claim_unit || 'dimensionless');
      if (value === null) continue;
      const cs = choosers(limit).map((c) => (i.constraints[c.id] && c.options.some((o) => o.id === i.constraints[c.id]) ? { ...c, value: i.constraints[c.id] } : c));
      const request: EvaluateRequest = { limit_id: limit.id, claim: { value, unit: limit.claim_unit || 'dimensionless' }, constraints: constraintsOf(cs), parameters: i.parameters };
      list = addMember(list, { limitId: limit.id, title: limit.title, direction: limit.direction === 'Min' ? 'Min' : 'Max', request, bound: null });
    }
    this.members.set(list);
    for (const m of list) this.fetchBound(m);
  }

  /** The member's limit at its nominal claim, from the engine. */
  private fetchBound(m: CascadeMember): void {
    this.api
      .evaluate(m.request)
      .pipe(catchError(() => of(null)))
      .subscribe((r) => {
        const bound = r ? decisiveBound(r, m.direction) : null;
        this.members.update((list) => list.map((x) => (x.limitId === m.limitId ? { ...x, bound } : x)));
        this.changes.next();
      });
  }

  protected remove(limitId: string): void {
    this.members.update((list) => removeMember(list, limitId));
    this.results.set([]);
    this.changes.next();
  }

  protected clear(): void {
    this.members.set([]);
    this.results.set([]);
    this.lastFirst = null;
  }

  protected onLoad(event: Event): void {
    this.load.set(Number((event.target as HTMLInputElement).value));
    this.changes.next();
  }

  protected show(m: CascadeMember, index: number): void {
    const req = requestsAtLoad(this.members(), this.load())[index] ?? m.request;
    const constraints = Object.fromEntries(Object.entries(req.constraints).map(([k, v]) => [k, v[0]]));
    this.openLaw.emit({ limit_id: m.limitId, claim: req.claim, parameters: req.parameters, constraints });
  }

  private ask(): Observable<(LabResult | null)[] | null> {
    const list = this.members();
    if (list.length < CASCADE_MIN) return of(null);
    const requests = requestsAtLoad(list, this.load());
    return forkJoin(
      requests.map((req, i) =>
        this.api.evaluate(req).pipe(
          map((r: EvaluateResponse) => resultOf(r, req.claim.value, list[i].direction)),
          catchError((f: ApiFailure) => of(failureOf(f))),
        ),
      ),
    ).pipe(catchError(() => of(null)));
  }

  private take(results: (LabResult | null)[] | null): void {
    if (!results) return;
    this.results.set(results);
    const first = this.first();
    if (first && first !== this.lastFirst) this.breaking.emit(first);
    this.lastFirst = first;
  }
}
