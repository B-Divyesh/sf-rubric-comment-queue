import { describe, expect, it } from 'vitest';
import { decryptWorkspace, emptyWorkspace, encryptWorkspace, feedbackText, parsePlainText, sampleWorkspace, toCsv } from './lib';

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

describe('encrypted backup', () => {
  it('@claim:passphrase-protection restores only with the right passphrase', async () => {
    const workspace = sampleWorkspace();
    workspace.submissions[0].excerpt = 'Private student sentence for encryption test.';
    const payload = await encryptWorkspace(workspace, 'correct horse battery staple');
    expect(payload).not.toContain('Private student sentence');
    const envelope = JSON.parse(payload);
    expect(Object.keys(envelope).sort()).toEqual(['data', 'iv', 'salt', 'v']);
    expect((await decryptWorkspace(payload, 'correct horse battery staple')).submissions[0].excerpt).toBe(workspace.submissions[0].excerpt);
    await expect(decryptWorkspace(payload, 'wrong passphrase')).rejects.toThrow('could not be opened');
  });
});

describe('export', () => {
  it('combines personal next step without changing the draft', () => {
    const item = { ...parsePlainText('Writing')[0], draft: 'Strong opening.', nextStep: 'Add one example.' };
    expect(feedbackText(item)).toBe('Strong opening.\n\nNext step: Add one example.');
    expect(toCsv([item])).toContain('"Strong opening.\n\nNext step: Add one example."');
  });

  it('starts with a populated teacher comment bank', () => expect(emptyWorkspace().comments.length).toBeGreaterThan(4));

  it('ships a realistic three-stage demo queue', () => {
    const sample = sampleWorkspace();
    expect(sample.submissions.map((item) => item.status)).toEqual(['ready', 'draft', 'new']);
    expect(sample.submissions.every((item) => item.excerpt.length > 80)).toBe(true);
    expect(sample.currentId).toBe('demo-station');
  });
});
