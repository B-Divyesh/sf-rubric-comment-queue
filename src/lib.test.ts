import { describe, expect, it } from 'vitest';
import { emptyWorkspace, feedbackText, parsePlainText, toCsv } from './lib';

describe('plain-text import', () => {
  it('splits blank-line separated excerpts and respects heading labels', () => {
    const items = parsePlainText('# Ada\nFirst response.\n\n---\n\n# Sam\nSecond response.');
    expect(items).toHaveLength(2);
    expect(items[0].label).toBe('Ada');
    expect(items[1].excerpt).toBe('Second response.');
  });

  it('ignores empty input', () => expect(parsePlainText(' \n ')).toEqual([]));

  it('keeps paragraph breaks inside one response', () => {
    const items = parsePlainText('# Roster 7\nFirst paragraph.\n\nSecond paragraph.');
    expect(items).toHaveLength(1);
    expect(items[0].excerpt).toContain('First paragraph.\n\nSecond paragraph.');
  });
});

describe('export', () => {
  it('combines personal next step without changing the draft', () => {
    const item = { ...parsePlainText('Writing')[0], draft: 'Strong opening.', nextStep: 'Add one example.' };
    expect(feedbackText(item)).toBe('Strong opening.\n\nNext step: Add one example.');
    expect(toCsv([item])).toContain('"Strong opening.\n\nNext step: Add one example."');
  });

  it('starts with a populated teacher comment bank', () => expect(emptyWorkspace().comments.length).toBeGreaterThan(4));
});
