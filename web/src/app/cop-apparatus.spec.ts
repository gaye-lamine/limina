import { ComponentFixture, TestBed } from '@angular/core/testing';
import { CopApparatus, TwinVerdicts, twinSentence } from './cop-apparatus';

async function render(
  mode: '' | 'heating' | 'cooling',
  twins: TwinVerdicts = { heating: null, cooling: null },
): Promise<ComponentFixture<CopApparatus>> {
  await TestBed.configureTestingModule({ imports: [CopApparatus] }).compileComponents();
  const fixture = TestBed.createComponent(CopApparatus);
  fixture.componentRef.setInput('th', '300');
  fixture.componentRef.setInput('tc', '280');
  fixture.componentRef.setInput('claim', '14.5');
  fixture.componentRef.setInput('mode', mode);
  fixture.componentRef.setInput('twinVerdicts', twins);
  fixture.componentRef.setInput('evaluated', true);
  await fixture.whenStable();
  return fixture;
}

describe('CopApparatus', () => {
  it('offers the mode as three labelled native radio inputs on the figure', async () => {
    const el = (await render('')).nativeElement as HTMLElement;
    const radios = Array.from(el.querySelectorAll('input[type="radio"]')) as HTMLInputElement[];
    expect(radios.map((r) => r.value)).toEqual(['', 'heating', 'cooling']);
    expect(radios.map((r) => el.querySelector(`label[for="${r.id}"]`)?.textContent?.trim())).toEqual([
      'not stated',
      'heating',
      'cooling',
    ]);
    expect(new Set(radios.map((r) => r.name)).size).toBe(1);
    expect(radios[0].checked).toBe(true);
    expect(el.querySelector('fieldset.segmented legend')?.textContent?.trim()).toBe('Mode');
  });

  it('emits the chosen mode', async () => {
    const fixture = await render('');
    const got: string[] = [];
    fixture.componentInstance.modeChange.subscribe((m) => got.push(m));
    const cooling = (fixture.nativeElement as HTMLElement).querySelector('#cop-mode-cooling') as HTMLInputElement;
    cooling.checked = true;
    cooling.dispatchEvent(new Event('change'));
    expect(got).toEqual(['cooling']);
  });

  it('with a stated mode, draws one full-size figure', async () => {
    const el = (await render('cooling')).nativeElement as HTMLElement;
    expect(el.querySelectorAll('app-cop-figure')).toHaveLength(1);
    expect(el.querySelector('.twins')).toBeNull();
    expect(el.querySelector('#cop-mode-cooling')).toHaveProperty('checked', true);
  });

  it('with the mode not stated, draws the machine twice, captioned, each with the engine verdict', async () => {
    const el = (await render('', { heating: 'COMPATIBLE', cooling: 'IMPOSSIBLE' })).nativeElement as HTMLElement;
    const twins = Array.from(el.querySelectorAll('.twin'));
    expect(twins).toHaveLength(2);
    expect(twins.map((t) => t.querySelector('.twin-caption')?.textContent?.trim())).toEqual(['If heating', 'If cooling']);
    expect(twins.map((t) => t.querySelector('app-cop-figure svg')?.classList.contains('compact'))).toEqual([true, true]);
    const words = twins.map((t) => t.querySelector('.twin-verdict'));
    expect(words.map((w) => w?.textContent?.trim())).toEqual(['COMPATIBLE', 'IMPOSSIBLE']);
    expect(words[0]?.classList.contains('verdict--compatible')).toBe(true);
    expect(words[1]?.classList.contains('verdict--impossible')).toBe(true);
    expect(el.querySelector('.twins-sentence')?.textContent?.trim()).toBe('Possible only in heating mode.');
  });

  it('shows no verdict word and no sentence before the engine answered', async () => {
    const el = (await render('')).nativeElement as HTMLElement;
    expect(Array.from(el.querySelectorAll('.twin-verdict')).map((w) => w.textContent?.trim())).toEqual([
      'evaluating',
      'evaluating',
    ]);
    expect(el.querySelector('.twins-sentence')).toBeNull();
  });

  it('gives the twin figures their own SVG ids', async () => {
    const el = (await render('')).nativeElement as HTMLElement;
    const ids = Array.from(el.querySelectorAll('[id]')).map((n) => n.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('puts the thermometers and the claimed COP in one row under the figure', async () => {
    const el = (await render('heating')).nativeElement as HTMLElement;
    const controls = el.querySelector('.apparatus-controls') as HTMLElement;
    expect(controls.previousElementSibling?.classList.contains('apparatus-scroll')).toBe(true);
    expect(controls.querySelector('#param-Th-value')).not.toBeNull();
    expect(controls.querySelector('#param-Tc-value')).not.toBeNull();
    expect(controls.querySelector('label[for="claim-value"]')?.textContent?.trim()).toBe('Claimed COP');
  });

  it('says widths are proportional and W = 1 in the caption', async () => {
    const el = (await render('heating')).nativeElement as HTMLElement;
    const caption = el.querySelector('.cop-apparatus > .fig-caption')?.textContent ?? '';
    expect(caption).toMatch(/^Fig\. 1:/);
    expect(caption).toContain('proportional to energy');
    expect(caption).toContain('W = 1');
  });
});

describe('twinSentence', () => {
  it('builds one sentence from the two engine verdicts', () => {
    expect(twinSentence({ heating: 'COMPATIBLE', cooling: 'IMPOSSIBLE' })).toBe('Possible only in heating mode.');
    expect(twinSentence({ heating: 'IMPOSSIBLE', cooling: 'COMPATIBLE' })).toBe('Possible only in cooling mode.');
    expect(twinSentence({ heating: 'COMPATIBLE', cooling: 'CONDITIONAL' })).toBe('Possible in both modes.');
    expect(twinSentence({ heating: 'IMPOSSIBLE', cooling: 'IMPOSSIBLE' })).toBe('Not possible in either mode.');
  });

  it('says nothing while a verdict is unknown or data is missing', () => {
    expect(twinSentence({ heating: null, cooling: 'IMPOSSIBLE' })).toBe('');
    expect(twinSentence({ heating: 'INSUFFICIENT_DATA', cooling: 'IMPOSSIBLE' })).toBe('');
  });
});
