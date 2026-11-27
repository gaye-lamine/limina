/**
 * Forge a new law (spec laboratory-3d, R-16): the laboratory sends a
 * sentence to `POST /v1/limits/author`, which writes a draft, checks its
 * units and samples it on a grid, then registers it. This file holds the
 * plain-language reading of that answer; the page reloads the library and
 * opens the draft like any law. Nothing here decides a verdict.
 */
import { AuthorResponse } from '../analysis.models';
import { ApiFailure } from '../api.models';
import { LAB } from '../strings';

/** Longest sentence the route accepts, in characters (MAX_AUTHOR_TEXT_CHARS of the API). */
export const FORGE_MAX = 2000;

/** One line of the progress list. */
export interface ForgeStep {
  text: string;
  state: 'running' | 'done' | 'failed';
}

/** Whether a sentence can be sent: not blank, not too long. */
export function canForge(text: string): boolean {
  const n = [...text].length;
  return text.trim() !== '' && n <= FORGE_MAX;
}

/** The progress while the request runs: the sentence is being written into a law and checked. */
export function forgeRunning(): ForgeStep[] {
  return [
    { text: LAB.forge.stepWrite, state: 'running' },
    { text: LAB.forge.stepUnits, state: 'running' },
    { text: LAB.forge.stepGrid(null), state: 'running' },
  ];
}

/** Sample points the checks of the new law used, summed over its configurations. */
export function samplesOf(r: AuthorResponse): number {
  return r.report.formulas.reduce((n, f) => n + (Number.isFinite(f.samples) ? f.samples : 0), 0);
}

/** The progress once the law is registered, from the checks the answer reports. */
export function forgeDone(r: AuthorResponse): ForgeStep[] {
  const units = r.report.dimension === 'passed';
  const finite = r.report.formulas.every((f) => f.finite !== false);
  const failedConditions = r.report.validity.reduce((n, v) => n + v.failed, 0);
  const steps: ForgeStep[] = [
    { text: LAB.forge.stepWritten(r.attempts), state: 'done' },
    { text: units ? LAB.forge.stepUnitsOk : LAB.forge.stepUnitsNo, state: units ? 'done' : 'failed' },
    { text: LAB.forge.stepGrid(samplesOf(r)), state: finite ? 'done' : 'failed' },
  ];
  if (failedConditions > 0) steps.push({ text: LAB.forge.stepConditions(failedConditions), state: 'done' });
  return steps;
}

/** How many answers were tried, when the refusal says it ("after 3 attempts"). */
export function attemptsIn(message: string): number | null {
  const m = /after (\d+) attempts?/i.exec(message);
  return m ? Number(m[1]) : null;
}

/** The refusal in plain words; the sentence stays in the box so it can be reworded. */
export function forgeFailure(f: ApiFailure): string {
  switch (f.code) {
    case 'LimitNotAuthored':
      return LAB.forge.notAuthored(attemptsIn(f.message));
    case 'ExtractionUnavailable':
      return LAB.forge.unavailable;
    case 'RateLimited':
    case 'ExtractionFailed':
    case 'ExtractionProviderError':
      return LAB.forge.busy;
    case 'EmptyText':
      return LAB.forge.empty;
    case 'TextTooLong':
      return LAB.forge.tooLong(FORGE_MAX);
    case 'NetworkError':
      return LAB.forge.offline;
    default:
      return LAB.forge.failed;
  }
}
