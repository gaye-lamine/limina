/**
 * System stress test (spec laboratory-3d, R-17): two to four laws loaded
 * together. Each law keeps its own claim, values and assumptions; its
 * stress is the claimed over the limit (a maximum) or the limit over the
 * claimed (a minimum), so above 1 means past the limit. The System load
 * slider scales every stress so that, at 100 %, the most constrained law
 * sits exactly at its limit; the claims sent to `/v1/evaluate` follow.
 * The verdict of each law at each load is the engine's.
 */
import { EvaluateRequest, LimitInfo } from '../api.models';
import { LabResult } from './lab-model';

export const CASCADE_MIN = 2;
export const CASCADE_MAX = 4;
/** The System load slider, in percent of the most constraining limit. */
export const LOAD_MIN = 20;
export const LOAD_MAX = 150;

/** One law of the stress test, with its own evaluate request at its nominal claim. */
export interface CascadeMember {
  limitId: string;
  title: string;
  direction: 'Max' | 'Min';
  /** The request at the nominal claim (base units); the load only changes `claim.value`. */
  request: EvaluateRequest;
  /** The engine's decisive limit at the nominal claim; null until known. */
  bound: number | null;
}

/** Whether a law can join: it states a maximum or a minimum (a predicted value has no limit to load). */
export function canJoin(limit: Pick<LimitInfo, 'direction'>): boolean {
  return limit.direction !== 'Equal';
}

/** Adds a member, at most CASCADE_MAX and once per law. */
export function addMember(list: CascadeMember[], m: CascadeMember): CascadeMember[] {
  if (list.length >= CASCADE_MAX || list.some((x) => x.limitId === m.limitId)) return list;
  return [...list, m];
}

export function removeMember(list: CascadeMember[], limitId: string): CascadeMember[] {
  return list.filter((m) => m.limitId !== limitId);
}

/** Stress of a claim against its limit: above 1 is past it. */
export function stressOf(claim: number, bound: number | null, direction: 'Max' | 'Min'): number | null {
  if (bound === null || !Number.isFinite(bound) || !Number.isFinite(claim) || bound <= 0 || claim <= 0) return null;
  return direction === 'Max' ? claim / bound : bound / claim;
}

/** The claim that puts a law at `stress`. */
export function claimAtStress(stress: number, bound: number, direction: 'Max' | 'Min'): number {
  return direction === 'Max' ? stress * bound : bound / stress;
}

/** The highest nominal stress of the members: the most constrained law. */
export function maxStress(members: CascadeMember[]): number | null {
  const s = members.map((m) => stressOf(m.request.claim.value, m.bound, m.direction)).filter((v): v is number => v !== null);
  return s.length ? Math.max(...s) : null;
}

/** Each member's claim at a load (percent of the most constraining limit). Members without a known limit keep their claim. */
export function claimsAtLoad(members: CascadeMember[], loadPercent: number): number[] {
  const top = maxStress(members);
  const load = loadPercent / 100;
  return members.map((m) => {
    const s0 = stressOf(m.request.claim.value, m.bound, m.direction);
    if (top === null || s0 === null || m.bound === null) return m.request.claim.value;
    return claimAtStress((s0 / top) * load, m.bound, m.direction);
  });
}

/** The requests to send at a load. */
export function requestsAtLoad(members: CascadeMember[], loadPercent: number): EvaluateRequest[] {
  const claims = claimsAtLoad(members, loadPercent);
  return members.map((m, i) => ({ ...m.request, claim: { ...m.request.claim, value: claims[i] } }));
}

/**
 * The first law to break: among those the engine finds IMPOSSIBLE at this
 * load, the one with the highest nominal stress (it crossed its limit at
 * the lowest load). Null when none is past its limit.
 */
export function firstBreaking(members: CascadeMember[], results: (LabResult | null)[]): string | null {
  let best: string | null = null;
  let top = -Infinity;
  members.forEach((m, i) => {
    if (results[i]?.kind !== 'impossible') return;
    const s = stressOf(m.request.claim.value, m.bound, m.direction) ?? 0;
    if (s > top) {
      top = s;
      best = m.limitId;
    }
  });
  return best;
}

/** The load (percent) at which each member reaches its limit, by the scaling; null without a known limit. */
export function breakingLoad(members: CascadeMember[]): (number | null)[] {
  const top = maxStress(members);
  return members.map((m) => {
    const s0 = stressOf(m.request.claim.value, m.bound, m.direction);
    return top === null || s0 === null ? null : (100 * top) / s0;
  });
}
