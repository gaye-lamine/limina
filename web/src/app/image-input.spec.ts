import { IMAGE_TYPES, MAX_BASE64, base64Of, fitWithin, imageProblem } from './image-input';

describe('image input', () => {
  it('accepts PNG, JPEG and WebP images only, and not empty files', () => {
    expect(IMAGE_TYPES).toEqual(['image/png', 'image/jpeg', 'image/webp']);
    expect(imageProblem({ type: 'image/png', size: 10 })).toBeNull();
    expect(imageProblem({ type: 'image/gif', size: 10 })).toBe('type');
    expect(imageProblem({ type: 'application/pdf', size: 10 })).toBe('type');
    expect(imageProblem({ type: 'image/jpeg', size: 0 })).toBe('empty');
  });

  it('fits a large image within the longest side, keeping its proportions', () => {
    expect(fitWithin(4000, 3000, 2000)).toEqual({ width: 2000, height: 1500 });
    expect(fitWithin(1200, 4800, 2000)).toEqual({ width: 500, height: 2000 });
    expect(fitWithin(800, 600, 2000)).toEqual({ width: 800, height: 600 });
  });

  it('strips the data URL prefix and stays under the request size the server accepts', () => {
    expect(base64Of('data:image/png;base64,AAAA')).toBe('AAAA');
    // Under 2 MB of request body, with room for the JSON around it.
    expect(MAX_BASE64).toBeLessThan(2 * 1024 * 1024 - 1024);
  });
});
