import { parseDecimal } from './decimal';

describe('parseDecimal', () => {
  it('reads a decimal point and a decimal comma alike', () => {
    expect(parseDecimal('0.60')).toBe(0.6);
    expect(parseDecimal('0,60')).toBe(0.6);
    expect(parseDecimal(' 14,5 ')).toBe(14.5);
    expect(parseDecimal('600')).toBe(600);
    expect(parseDecimal('-0,5')).toBe(-0.5);
    expect(parseDecimal('1e3')).toBe(1000);
    expect(parseDecimal('.5')).toBe(0.5);
  });

  it('rejects empty, ambiguous or non-numeric text', () => {
    expect(parseDecimal('')).toBeNull();
    expect(parseDecimal('   ')).toBeNull();
    expect(parseDecimal('abc')).toBeNull();
    expect(parseDecimal('1,000.5')).toBeNull();
    expect(parseDecimal('1,2,3')).toBeNull();
    expect(parseDecimal('0.6.0')).toBeNull();
    expect(parseDecimal('Infinity')).toBeNull();
    expect(parseDecimal('0x10')).toBeNull();
  });
});
