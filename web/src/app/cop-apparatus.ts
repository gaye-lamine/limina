import { Component, computed, input, output } from '@angular/core';
import { Verdict } from './api.models';
import { BENCH_FIGURE_STYLES } from './bench-figure';
import { CopMode } from './cop-geometry';
import { CopFigure } from './cop-figure';
import { STRINGS } from './strings';
import { Thermometer } from './thermometer';

/** Engine verdicts of the two modes, used when the mode is not stated; null while unknown. */
export interface TwinVerdicts {
  heating: Verdict | null;
  cooling: Verdict | null;
}

/** A verdict that leaves the claim possible. */
function possible(v: Verdict): boolean {
  return v === 'COMPATIBLE' || v === 'CONDITIONAL';
}

/**
 * One sentence from the two engine verdicts, or '' while one is unknown or
 * is INSUFFICIENT_DATA. The page never computes a verdict itself.
 */
export function twinSentence(v: TwinVerdicts): string {
  const { heating, cooling } = v;
  if (heating === null || cooling === null) return '';
  if (heating === 'INSUFFICIENT_DATA' || cooling === 'INSUFFICIENT_DATA') return '';
  const s = STRINGS.copSentence;
  if (possible(heating) && possible(cooling)) return s.both;
  if (possible(heating)) return s.heatingOnly;
  if (possible(cooling)) return s.coolingOnly;
  return s.neither;
}

const VERDICT_CLASS: Record<Verdict, string> = {
  IMPOSSIBLE: 'verdict--impossible',
  CONDITIONAL: 'verdict--conditional',
  COMPATIBLE: 'verdict--compatible',
  INSUFFICIENT_DATA: 'verdict--insufficient-data',
};

/**
 * The COP apparatus: a heat pump or refrigerator between two reservoirs,
 * with the mode control, the thermometers and the claimed COP on the bench.
 * With the mode not stated, it draws the machine twice, one per mode, each
 * with the verdict the engine returned for that mode.
 */
@Component({
  selector: 'app-cop-apparatus',
  imports: [CopFigure, Thermometer],
  templateUrl: './cop-apparatus.html',
  styles: [BENCH_FIGURE_STYLES],
})
export class CopApparatus {
  readonly th = input<string>('');
  readonly tc = input<string>('');
  readonly claim = input<string>('');
  readonly thUnit = input<string>('K');
  readonly tcUnit = input<string>('K');
  /** '' when not stated. */
  readonly mode = input<'' | CopMode>('');
  readonly twinVerdicts = input<TwinVerdicts>({ heating: null, cooling: null });
  /** True once an evaluation was requested, so the twin verdicts are awaited. */
  readonly evaluated = input<boolean>(false);
  readonly thChange = output<string>();
  readonly tcChange = output<string>();
  readonly claimChange = output<string>();
  readonly modeChange = output<'' | CopMode>();

  protected readonly s = STRINGS;
  protected readonly modes: ('' | CopMode)[] = ['', 'heating', 'cooling'];
  protected readonly twinModes: CopMode[] = ['heating', 'cooling'];

  protected readonly stated = computed(() => this.mode() !== '');
  protected readonly statedMode = computed<CopMode>(() => (this.mode() === 'cooling' ? 'cooling' : 'heating'));
  protected readonly sentence = computed(() => twinSentence(this.twinVerdicts()));

  protected modeId(m: '' | CopMode): string {
    return `cop-mode-${m === '' ? 'none' : m}`;
  }

  protected verdictOf(m: CopMode): Verdict | null {
    return this.twinVerdicts()[m];
  }

  protected verdictClass(v: Verdict): string {
    return VERDICT_CLASS[v];
  }
}
