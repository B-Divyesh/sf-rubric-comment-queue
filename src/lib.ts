export type Status = 'new' | 'draft' | 'ready';

export interface Submission {
  id: string;
  label: string;
  excerpt: string;
  criterion: string;
  commentId: string;
  draft: string;
  nextStep: string;
  status: Status;
  updatedAt: string;
}

export interface CommentBlock {
  id: string;
  criterion: string;
  title: string;
  body: string;
  custom?: boolean;
}

export interface Workspace {
  version: 1;
  submissions: Submission[];
  comments: CommentBlock[];
  currentId: string | null;
  updatedAt: string;
}

export const CRITERIA = ['Ideas & evidence', 'Organization', 'Style & voice', 'Language & conventions', 'Whole response'];

export const DEFAULT_COMMENTS: CommentBlock[] = [
  { id: 'evidence-specific', criterion: 'Ideas & evidence', title: 'Connect evidence', body: 'Your evidence is relevant. Explain how this detail supports your central idea so the reader can follow your reasoning.' },
  { id: 'evidence-develop', criterion: 'Ideas & evidence', title: 'Develop the idea', body: 'You have a promising idea here. Add one specific example, detail, or quotation that makes the point concrete.' },
  { id: 'organization-path', criterion: 'Organization', title: 'Clarify the path', body: 'The key parts are present. Add a transition that shows how this paragraph builds on the one before it.' },
  { id: 'organization-focus', criterion: 'Organization', title: 'Focus the paragraph', body: 'This paragraph is doing more than one job. Keep the sentences that support its main point and move the other idea to its own paragraph.' },
  { id: 'voice-precise', criterion: 'Style & voice', title: 'Choose precise words', body: 'Your voice is coming through. Replace one general phrase with language that shows exactly what you mean.' },
  { id: 'language-read', criterion: 'Language & conventions', title: 'Read for flow', body: 'Read this section aloud and mark where you naturally pause. Use those pauses to check sentence boundaries and punctuation.' },
  { id: 'whole-strength', criterion: 'Whole response', title: 'Name the strength', body: 'The response has a clear direction and gives the reader a reason to keep going. Preserve that focus as you revise.' }
];

export function uid(): string {
  return crypto.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function emptyWorkspace(): Workspace {
  return { version: 1, submissions: [], comments: DEFAULT_COMMENTS, currentId: null, updatedAt: new Date().toISOString() };
}

export function parsePlainText(input: string): Submission[] {
  const normalized = input.replace(/\r\n/g, '\n').trim();
  if (!normalized) return [];
  return normalized.split(/\n\s*---\s*\n/g).map((part, index) => {
    const lines = part.trim().split('\n');
    const named = lines[0]?.match(/^#{1,3}\s+(.+)/);
    const label = named ? named[1].trim() : `Response ${index + 1}`;
    const excerpt = (named ? lines.slice(1) : lines).join('\n').trim();
    return {
      id: uid(), label, excerpt, criterion: 'Ideas & evidence', commentId: '', draft: '', nextStep: '', status: 'new' as const, updatedAt: new Date().toISOString()
    };
  }).filter((item) => item.excerpt.length > 0);
}

export function feedbackText(submission: Submission): string {
  return [submission.draft.trim(), submission.nextStep.trim() ? `Next step: ${submission.nextStep.trim()}` : ''].filter(Boolean).join('\n\n');
}

function csvCell(value: string): string {
  return `"${value.replace(/"/g, '""')}"`;
}

export function toCsv(submissions: Submission[]): string {
  const header = ['Response', 'Criterion', 'Feedback', 'Status'];
  const rows = submissions.map((item) => [item.label, item.criterion, feedbackText(item), item.status]);
  return [header, ...rows].map((row) => row.map(csvCell).join(',')).join('\r\n');
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  bytes.forEach((byte) => binary += String.fromCharCode(byte));
  return btoa(binary);
}

function base64ToBytes(value: string): Uint8Array<ArrayBuffer> {
  const binary = atob(value);
  return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}

async function deriveKey(passphrase: string, salt: Uint8Array<ArrayBuffer>, usage: KeyUsage[]): Promise<CryptoKey> {
  const source = await crypto.subtle.importKey('raw', new TextEncoder().encode(passphrase), 'PBKDF2', false, ['deriveKey']);
  return crypto.subtle.deriveKey({ name: 'PBKDF2', salt, iterations: 210_000, hash: 'SHA-256' }, source, { name: 'AES-GCM', length: 256 }, false, usage);
}

export async function encryptWorkspace(workspace: Workspace, passphrase: string): Promise<string> {
  if (passphrase.length < 10) throw new Error('Use a passphrase with at least 10 characters.');
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const key = await deriveKey(passphrase, salt, ['encrypt']);
  const ciphertext = new Uint8Array(await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, new TextEncoder().encode(JSON.stringify(workspace))));
  return JSON.stringify({ v: 1, salt: bytesToBase64(salt), iv: bytesToBase64(iv), data: bytesToBase64(ciphertext) });
}

export async function decryptWorkspace(payload: string, passphrase: string): Promise<Workspace> {
  try {
    const parsed = JSON.parse(payload) as { v: number; salt: string; iv: string; data: string };
    if (parsed.v !== 1) throw new Error();
    const key = await deriveKey(passphrase, base64ToBytes(parsed.salt), ['decrypt']);
    const clear = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: base64ToBytes(parsed.iv) }, key, base64ToBytes(parsed.data));
    const workspace = JSON.parse(new TextDecoder().decode(clear)) as Workspace;
    if (workspace.version !== 1 || !Array.isArray(workspace.submissions)) throw new Error();
    return workspace;
  } catch {
    throw new Error('That backup could not be opened. Check your passphrase and try again.');
  }
}

export function download(name: string, content: string, type: string): void {
  const url = URL.createObjectURL(new Blob([content], { type }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = name;
  anchor.click();
  URL.revokeObjectURL(url);
}
