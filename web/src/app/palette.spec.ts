import { TestBed } from '@angular/core/testing';
import { Command, Palette, matchCommands } from './palette';

const COMMANDS: Command[] = [
  { id: 'claim:0', group: 'claims', label: '1. speed of a massive body · 3.60e+8 m/s', hint: 'IMPOSSIBLE', kind: 'impossible' },
  { id: 'claim:1', group: 'claims', label: '2. resolution · 100 nm', hint: 'CONDITIONAL', kind: 'conditional' },
  { id: 'library', group: 'actions', label: 'Open Library' },
];

describe('command palette', () => {
  it('matches every word of the query in the label or the hint', () => {
    expect(matchCommands(COMMANDS, '').map((c) => c.id)).toEqual(['claim:0', 'claim:1', 'library']);
    expect(matchCommands(COMMANDS, 'impossible').map((c) => c.id)).toEqual(['claim:0']);
    expect(matchCommands(COMMANDS, 'open lib').map((c) => c.id)).toEqual(['library']);
    expect(matchCommands(COMMANDS, 'nothing')).toEqual([]);
  });

  it('lists groups, moves with the arrow keys and runs the active command with Enter', async () => {
    await TestBed.configureTestingModule({ imports: [Palette] }).compileComponents();
    const f = TestBed.createComponent(Palette);
    f.componentRef.setInput('commands', COMMANDS);
    const ran: string[] = [];
    f.componentInstance.run.subscribe((id) => ran.push(id));
    let closed = false;
    f.componentInstance.closed.subscribe(() => (closed = true));
    f.detectChanges();
    const el = f.nativeElement as HTMLElement;
    expect(Array.from(el.querySelectorAll('.palette-group')).map((g) => g.textContent?.trim())).toEqual(['Claims', 'Actions']);
    expect(el.querySelector('.palette-hint--impossible')?.textContent).toBe('IMPOSSIBLE');
    const input = el.querySelector('.palette-input') as HTMLInputElement;
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown' }));
    f.detectChanges();
    expect(el.querySelector('[aria-selected="true"]')?.textContent).toContain('resolution');
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    expect(ran).toEqual(['claim:1']);
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(closed).toBe(true);
  });
});
