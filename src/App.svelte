<script lang="ts">
  import { onMount } from 'svelte';
  import {
    CRITERIA, DEFAULT_COMMENTS, decryptWorkspace, download, emptyWorkspace, encryptWorkspace,
    feedbackText, parsePlainText, toCsv, uid,
    type CommentBlock, type Submission, type Workspace
  } from './lib';

  const STORAGE_KEY = 'rcq_workspace:v1';
  const LICENSE_KEY = 'sb_license:rubric-comment-queue';
  const VERDICT_KEY = 'sb_license_verdict:rubric-comment-queue';
  const API_BASE = 'https://api.sociobot.in/api/v1/products/rubric-comment-queue';
  let workspace: Workspace = emptyWorkspace();
  let hydrated = false;
  let online = typeof navigator === 'undefined' ? true : navigator.onLine;
  let toast = '';
  let current: Submission | undefined;
  let readyCount = 0;
  let importText = '';
  let importError = '';
  let commentTitle = '';
  let commentBody = '';
  let commentCriterion = CRITERIA[0];
  let license = '';
  let paid = false;
  let licenseNote = '';
  let backupPassphrase = '';
  let backupBusy = false;
  let theme = 'light';
  let importDialog: HTMLDialogElement;
  let commentDialog: HTMLDialogElement;
  let backupDialog: HTMLDialogElement;

  $: current = workspace.submissions.find((item) => item.id === workspace.currentId) ?? workspace.submissions[0];
  $: readyCount = workspace.submissions.filter((item) => item.status === 'ready').length;
  $: filteredComments = current ? workspace.comments.filter((comment) => comment.criterion === current?.criterion || comment.criterion === 'Whole response') : [];

  function announce(message: string) {
    toast = '';
    setTimeout(() => toast = message, 10);
  }

  function persist(message?: string) {
    workspace.updatedAt = new Date().toISOString();
    workspace = { ...workspace, submissions: [...workspace.submissions], comments: [...workspace.comments] };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(workspace));
    if (message) announce(message);
  }

  function updateCurrent(values: Partial<Submission>) {
    if (!current) return;
    Object.assign(current, values, { updatedAt: new Date().toISOString() });
    if (current.status === 'new' && (values.draft !== undefined || values.nextStep !== undefined)) current.status = 'draft';
    persist();
  }

  function openImport() {
    importError = '';
    importDialog.showModal();
  }

  function runImport() {
    const items = parsePlainText(importText);
    if (!items.length) {
      importError = 'Paste at least one response. Separate responses with a blank line and three dashes.';
      return;
    }
    workspace.submissions.push(...items);
    workspace.currentId ||= items[0].id;
    persist(`${items.length} ${items.length === 1 ? 'response' : 'responses'} added to the queue.`);
    importText = '';
    importDialog.close();
  }

  async function readFile(event: Event) {
    const file = (event.currentTarget as HTMLInputElement).files?.[0];
    if (!file) return;
    if (file.size > 1_000_000) {
      importError = 'That file is over 1 MB. Split it into smaller plain-text files first.';
      return;
    }
    importText = await file.text();
  }

  function chooseComment(comment: CommentBlock) {
    if (!current) return;
    updateCurrent({ commentId: comment.id, draft: comment.body, status: 'draft' });
    announce(`“${comment.title}” added. Edit it to fit this response.`);
    requestAnimationFrame(() => document.querySelector<HTMLTextAreaElement>('#feedback-draft')?.focus());
  }

  function saveAndNext() {
    if (!current) return;
    if (!current.draft.trim() || !current.nextStep.trim()) {
      announce('Add feedback and one personal next step before marking this ready.');
      document.querySelector<HTMLElement>(!current.draft.trim() ? '#feedback-draft' : '#next-step')?.focus();
      return;
    }
    current.status = 'ready';
    const index = workspace.submissions.indexOf(current);
    const next = workspace.submissions.slice(index + 1).find((item) => item.status !== 'ready')
      ?? workspace.submissions.find((item) => item.status !== 'ready');
    workspace.currentId = next?.id ?? current.id;
    persist(next ? `${current.label} is ready. Next response opened.` : 'Every response is ready to export.');
  }

  function move(step: number) {
    if (!current) return;
    const index = workspace.submissions.indexOf(current);
    const nextIndex = Math.max(0, Math.min(workspace.submissions.length - 1, index + step));
    workspace.currentId = workspace.submissions[nextIndex].id;
    persist();
  }

  function removeCurrent() {
    if (!current || !confirm(`Delete “${current.label}” and its feedback? This cannot be undone.`)) return;
    const index = workspace.submissions.indexOf(current);
    workspace.submissions.splice(index, 1);
    workspace.currentId = workspace.submissions[Math.min(index, workspace.submissions.length - 1)]?.id ?? null;
    persist(`${current.label} deleted.`);
  }

  async function copyCurrent() {
    if (!current) return;
    try {
      await navigator.clipboard.writeText(feedbackText(current));
      announce('Feedback copied to the clipboard.');
    } catch {
      announce('Clipboard access was blocked. Select and copy the feedback text instead.');
    }
  }

  function exportCsv() {
    if (!workspace.submissions.length) return;
    download(`rubric-feedback-${new Date().toISOString().slice(0, 10)}.csv`, toCsv(workspace.submissions), 'text/csv;charset=utf-8');
    announce(`${workspace.submissions.length} feedback rows exported.`);
  }

  function exportLocalBackup() {
    download(`rubric-comment-queue-${new Date().toISOString().slice(0, 10)}.json`, JSON.stringify(workspace, null, 2), 'application/json');
    announce('Local backup downloaded.');
  }

  function openNewComment() {
    commentTitle = '';
    commentBody = '';
    commentCriterion = current?.criterion ?? CRITERIA[0];
    commentDialog.showModal();
  }

  function addComment() {
    if (!commentTitle.trim() || !commentBody.trim()) return;
    workspace.comments.push({ id: uid(), title: commentTitle.trim(), body: commentBody.trim(), criterion: commentCriterion, custom: true });
    persist('Comment block saved to your bank.');
    commentDialog.close();
  }

  function deleteComment(comment: CommentBlock) {
    if (!comment.custom || !confirm(`Remove “${comment.title}” from your comment bank?`)) return;
    workspace.comments = workspace.comments.filter((item) => item.id !== comment.id);
    persist('Comment block removed.');
  }

  function toggleTheme() {
    theme = theme === 'light' ? 'dark' : 'light';
    document.documentElement.dataset.theme = theme;
    localStorage.setItem('rcq_theme', theme);
  }

  async function verifyLicense(force = false) {
    if (!license || !online) return;
    const cached = JSON.parse(localStorage.getItem(VERDICT_KEY) ?? 'null') as { valid: boolean; checked: number } | null;
    if (!force && cached && Date.now() - cached.checked < 86_400_000) {
      paid = cached.valid;
      return;
    }
    try {
      const response = await fetch(`${API_BASE}/verify?license=${encodeURIComponent(license)}`);
      if (!response.ok) throw new Error();
      const verdict = await response.json() as { valid: boolean };
      paid = verdict.valid;
      localStorage.setItem(VERDICT_KEY, JSON.stringify({ valid: paid, checked: Date.now() }));
      licenseNote = paid ? 'Desk Pass active on this device.' : 'This license is no longer active.';
    } catch {
      licenseNote = 'Could not check the license. Your free workspace still works.';
    }
  }

  function restoreLicense() {
    license = license.trim();
    if (license.length < 12) {
      licenseNote = 'Paste the complete license token from your receipt.';
      return;
    }
    localStorage.setItem(LICENSE_KEY, license);
    localStorage.removeItem(VERDICT_KEY);
    paid = true;
    licenseNote = 'License saved. Checking it now…';
    verifyLicense(true);
  }

  async function cloudBackup(mode: 'save' | 'restore') {
    if (!paid || !license) return;
    backupBusy = true;
    licenseNote = '';
    try {
      if (mode === 'save') {
        const payload = await encryptWorkspace(workspace, backupPassphrase);
        const response = await fetch('/api/backup', { method: 'PUT', headers: { 'content-type': 'application/json', authorization: `Bearer ${license}` }, body: JSON.stringify({ payload }) });
        if (!response.ok) throw new Error(response.status === 401 ? 'Your license could not be verified.' : 'The backup could not be saved.');
        announce('Encrypted backup saved. Only your passphrase can open it.');
      } else {
        const response = await fetch('/api/backup', { headers: { authorization: `Bearer ${license}` } });
        if (response.status === 404) throw new Error('No cloud backup exists for this license yet.');
        if (!response.ok) throw new Error('The backup could not be downloaded.');
        const data = await response.json() as { payload: string };
        workspace = await decryptWorkspace(data.payload, backupPassphrase);
        persist('Encrypted backup restored on this device.');
      }
      backupDialog.close();
      backupPassphrase = '';
    } catch (error) {
      licenseNote = error instanceof Error ? error.message : 'The backup request failed.';
    } finally {
      backupBusy = false;
    }
  }

  async function deleteCloudBackup() {
    if (!paid || !license || !confirm('Delete the encrypted cloud backup for this license? Your local workspace will stay on this device.')) return;
    backupBusy = true;
    try {
      const response = await fetch('/api/backup', { method: 'DELETE', headers: { authorization: `Bearer ${license}` } });
      if (!response.ok) throw new Error();
      backupDialog.close();
      announce('Encrypted cloud backup deleted. Local work was not changed.');
    } catch {
      licenseNote = 'The cloud backup could not be deleted. Try again when you are online.';
    } finally {
      backupBusy = false;
    }
  }

  function keyHandler(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key === 'Enter' && current) {
      event.preventDefault();
      saveAndNext();
    }
  }

  onMount(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved) as Workspace;
        if (parsed.version === 1 && Array.isArray(parsed.submissions)) workspace = parsed;
      }
    } catch {
      announce('Saved data could not be read. A fresh local workspace was opened.');
    }
    theme = localStorage.getItem('rcq_theme') === 'dark' ? 'dark' : 'light';
    document.documentElement.dataset.theme = theme;
    const params = new URLSearchParams(location.search);
    const returned = params.get('license');
    if (returned) {
      localStorage.setItem(LICENSE_KEY, returned);
      params.delete('license');
      history.replaceState({}, '', `${location.pathname}${params.size ? `?${params}` : ''}${location.hash}`);
    }
    license = returned ?? localStorage.getItem(LICENSE_KEY) ?? '';
    const cached = JSON.parse(localStorage.getItem(VERDICT_KEY) ?? 'null') as { valid: boolean; checked: number } | null;
    paid = Boolean(license && cached?.valid);
    if (license) verifyLicense();
    const goOnline = () => { online = true; verifyLicense(); };
    const goOffline = () => online = false;
    addEventListener('online', goOnline);
    addEventListener('offline', goOffline);
    addEventListener('keydown', keyHandler);
    hydrated = true;
    fetch('/api/pageview', { method: 'POST', keepalive: true }).catch(() => undefined);
    return () => {
      removeEventListener('online', goOnline);
      removeEventListener('offline', goOffline);
      removeEventListener('keydown', keyHandler);
    };
  });
</script>

<svelte:head>
  <meta name="color-scheme" content={theme === 'dark' ? 'dark' : 'light'} />
</svelte:head>

<a class="skip-link" href="#main">Skip to review workspace</a>
<header class="topbar">
  <a class="brand" href="/" aria-label="Rubric Comment Queue home"><img src="/mark.svg" width="36" height="36" alt="" /><span>Rubric<br />Comment Queue</span></a>
  <nav aria-label="Utility navigation">
    <span class:offline={!online} class="connection"><span aria-hidden="true">●</span> {online ? 'Local autosave on' : 'Offline · local save on'}</span>
    <button class="icon-button" type="button" on:click={toggleTheme} aria-label={`Use ${theme === 'light' ? 'dark' : 'light'} theme`}>{theme === 'light' ? '◐' : '◑'}</button>
    <button class="outline small" type="button" on:click={() => backupDialog.showModal()}>{paid ? 'Encrypted backup' : 'Desk Pass'}</button>
  </nav>
</header>

{#if location.pathname === '/privacy' || location.pathname === '/terms'}
  <main id="main" class="legal-page">
    <a class="back-link" href="/">← Back to the queue</a>
    {#if location.pathname === '/privacy'}
      <h1>Privacy, in plain language</h1>
      <p class="lede">Your students’ writing belongs in your classroom, not in a training set.</p>
      <h2>What stays on this device</h2>
      <p>Response labels, excerpts, comment blocks, drafts, and next steps are saved in your browser’s local storage. They do not leave this device unless you choose encrypted backup.</p>
      <h2>Encrypted backup</h2>
      <p>Desk Pass backup is encrypted in your browser with AES-256-GCM before upload. The server stores the encrypted blob, its update time, and a one-way hash of your license. We cannot read the contents or recover a forgotten passphrase.</p>
      <h2>Minimal service data</h2>
      <p>We count an aggregate page view and retain ordinary short-lived server logs for reliability. There are no advertising cookies, analytics scripts, model training, or student profiles. Billing and license verification are handled by Sociobot/Dodo as merchant of record.</p>
      <h2>Your controls</h2>
      <p>Delete individual responses in the queue, clear site data in your browser, or overwrite/delete your encrypted backup. Contact privacy@sociobot.in for a backup deletion request tied to a license.</p>
    {:else}
      <h1>Terms of use</h1>
      <p class="lede">Rubric Comment Queue is a teacher-controlled writing feedback utility.</p>
      <h2>Your responsibility</h2>
      <p>You decide what feedback is appropriate and remain responsible for reviewing every comment before sharing it. The service does not score writing, detect plagiarism, or make educational decisions.</p>
      <h2>Appropriate data</h2>
      <p>Use minimal identifiers such as initials or roster numbers. Do not store information you are not authorized to process. Follow your school’s policies and applicable FERPA/GDPR requirements.</p>
      <h2>Desk Pass</h2>
      <p>Desk Pass is a $29 one-time license for encrypted backup on supported devices. Sociobot/Dodo is merchant of record and handles payment and refunds. A refund revokes the license. Core local review and export remain free.</p>
      <h2>Availability</h2>
      <p>The service is provided as-is without a guarantee that it will meet every institutional requirement. Export local backups regularly. These terms may be updated with the effective date shown below.</p>
      <p><strong>Effective:</strong> August 27, 2026.</p>
    {/if}
  </main>
{:else}
  <main id="main" class="app-shell" aria-busy={!hydrated}>
    <section class="intro" aria-labelledby="page-title">
      <div>
        <p class="eyebrow">Teacher-authored · teacher-approved</p>
        <h1 id="page-title">Feedback stays yours.</h1>
        <p>Move every response from excerpt to specific, sendable feedback—without outsourcing the judgment.</p>
      </div>
      {#if workspace.submissions.length}
        <div class="progress-card" aria-label={`${readyCount} of ${workspace.submissions.length} responses ready`}>
          <span><strong>{readyCount}</strong> / {workspace.submissions.length} ready</span>
          <div class="progress-track"><span style={`width:${workspace.submissions.length ? readyCount / workspace.submissions.length * 100 : 0}%`}></span></div>
        </div>
      {/if}
    </section>

    {#if workspace.submissions.length === 0}
      <section class="empty-state" aria-labelledby="empty-title">
        <picture><source media="(max-width: 640px)" srcset="/queue-desk-640.webp" /><img src="/queue-desk.webp" width="960" height="640" alt="Abstract paper excerpts passing through a blue rubric tray and orange teacher stamp into a neat feedback stack" fetchpriority="high" /></picture>
        <div class="empty-copy">
          <p class="stamp">EMPTY DESK</p>
          <h2 id="empty-title">Bring the writing. Keep the judgment.</h2>
          <p>Paste excerpts or import a plain-text file. Separate responses with a blank line and <code>---</code>. Names are optional; initials or roster numbers are safer.</p>
          <button class="primary" type="button" on:click={openImport}>Add the first responses <span aria-hidden="true">→</span></button>
          <p class="micro">Nothing is sent to an AI model. Work autosaves on this device.</p>
        </div>
      </section>
    {:else}
      <div class="workspace">
        <aside class="queue" aria-labelledby="queue-title">
          <div class="section-heading"><div><p class="kicker">Step 1</p><h2 id="queue-title">Response queue</h2></div><button class="square-button" type="button" on:click={openImport} aria-label="Add responses">+</button></div>
          <ol>
            {#each workspace.submissions as item, index (item.id)}
              <li class:active={item.id === current?.id} class:ready={item.status === 'ready'}>
                <button type="button" on:click={() => { workspace.currentId = item.id; persist(); }} aria-current={item.id === current?.id ? 'step' : undefined}>
                  <span class="queue-number">{String(index + 1).padStart(2, '0')}</span>
                  <span><strong>{item.label}</strong><small>{item.status === 'ready' ? '✓ Ready' : item.status === 'draft' ? 'In review' : 'Not started'}</small></span>
                </button>
              </li>
            {/each}
          </ol>
          <button class="outline full" type="button" on:click={exportCsv}>Export all CSV</button>
          <button class="text-button full" type="button" on:click={exportLocalBackup}>Download local backup</button>
        </aside>

        {#if current}
          <article class="review-sheet" aria-labelledby="review-title">
            <div class="sheet-topline"><span>{current.status === 'ready' ? '✓ Ready to send' : 'Now reviewing'}</span><span>Saved locally</span></div>
            <div class="sheet-heading">
              <div><p class="kicker">Step 2 · Read & respond</p><h2 id="review-title">{current.label}</h2></div>
              <button class="danger-link" type="button" on:click={removeCurrent}>Delete</button>
            </div>
            <section class="excerpt" aria-labelledby="excerpt-title"><h3 id="excerpt-title">Student excerpt</h3><blockquote>{current.excerpt}</blockquote></section>
            <div class="field-row">
              <label for="criterion">Rubric criterion</label>
              <select id="criterion" value={current.criterion} on:change={(event) => updateCurrent({ criterion: event.currentTarget.value, commentId: '' })}>
                {#each CRITERIA as criterion}<option value={criterion}>{criterion}</option>{/each}
              </select>
            </div>
            <div class="field-block">
              <label for="feedback-draft">Feedback draft <span>Edit the selected block in your own words</span></label>
              <textarea id="feedback-draft" rows="7" value={current.draft} on:input={(event) => updateCurrent({ draft: event.currentTarget.value })} placeholder="Choose a comment block or write your own feedback…"></textarea>
              <small class="count">{current.draft.length} characters</small>
            </div>
            <div class="field-block next-step">
              <label for="next-step">One personal next step <span>Required to mark ready</span></label>
              <textarea id="next-step" rows="3" value={current.nextStep} on:input={(event) => updateCurrent({ nextStep: event.currentTarget.value })} placeholder="For your next draft, try…"></textarea>
            </div>
            <div class="sheet-actions">
              <div><button class="outline" type="button" on:click={() => move(-1)} disabled={workspace.submissions.indexOf(current) === 0}>← Back</button><button class="outline" type="button" on:click={copyCurrent} disabled={!current.draft.trim()}>Copy feedback</button></div>
              <button class="primary" type="button" on:click={saveAndNext}>Save & next <span class="key-hint">⌘↵</span></button>
            </div>
          </article>

          <aside class="comment-bank" aria-labelledby="bank-title">
            <div class="section-heading"><div><p class="kicker">Step 3</p><h2 id="bank-title">Your comment bank</h2></div><button class="square-button" type="button" on:click={openNewComment} aria-label="Add a comment block">+</button></div>
            <p class="bank-help">Blocks for <strong>{current.criterion}</strong>. Choose one, then make it specific.</p>
            {#if filteredComments.length}
              <ul>
                {#each filteredComments as comment (comment.id)}
                  <li>
                    <button class:selected={current.commentId === comment.id} class="comment-block" type="button" on:click={() => chooseComment(comment)}>
                      <span class="comment-meta"><strong>{comment.title}</strong>{#if comment.custom}<em>Yours</em>{/if}</span>
                      <span>{comment.body}</span>
                    </button>
                    {#if comment.custom}<button class="remove-block" type="button" on:click={() => deleteComment(comment)} aria-label={`Remove ${comment.title}`}>×</button>{/if}
                  </li>
                {/each}
              </ul>
            {:else}
              <div class="mini-empty"><p>No blocks for this criterion yet.</p><button class="text-button" type="button" on:click={openNewComment}>Write one</button></div>
            {/if}
          </aside>
        {/if}
      </div>
    {/if}
  </main>
{/if}

<footer>
  <p><strong>Rubric Comment Queue</strong> · No auto-grading. No model training. <a href="/privacy">Privacy</a> <a href="/terms">Terms</a></p>
  <p>Illustration generated for Rubric Comment Queue.</p>
</footer>

<dialog bind:this={importDialog} aria-labelledby="import-title">
  <form method="dialog" on:submit={(event) => event.preventDefault()}>
    <div class="dialog-head"><div><p class="kicker">Add to queue</p><h2 id="import-title">Import responses</h2></div><button class="icon-button" value="cancel" aria-label="Close import dialog">×</button></div>
    <p>Paste plain text below. Put an optional <code># label</code> on the first line, and separate responses with a blank line, three dashes, then another blank line.</p>
    <label class="file-button" for="text-file">Choose .txt file</label><input id="text-file" class="visually-hidden" type="file" accept=".txt,text/plain" on:change={readFile} />
    <label for="import-text">Response text</label>
    <textarea id="import-text" bind:value={importText} rows="10" aria-describedby={importError ? 'import-error' : 'import-help'} placeholder="# Roster 12&#10;The opening paragraph…&#10;&#10;---&#10;&#10;# Roster 13&#10;In this response…"></textarea>
    <small id="import-help">Maximum file size: 1 MB.</small>
    {#if importError}<p id="import-error" class="error" role="alert">{importError}</p>{/if}
    <div class="dialog-actions"><button class="outline" value="cancel">Cancel</button><button class="primary" type="button" on:click={runImport}>Add to queue</button></div>
  </form>
</dialog>

<dialog bind:this={commentDialog} aria-labelledby="comment-title">
  <form method="dialog" on:submit={(event) => event.preventDefault()}>
    <div class="dialog-head"><div><p class="kicker">Reusable language</p><h2 id="comment-title">Write a comment block</h2></div><button class="icon-button" value="cancel" aria-label="Close comment dialog">×</button></div>
    <label for="comment-criterion">Rubric criterion</label><select id="comment-criterion" bind:value={commentCriterion}>{#each CRITERIA as criterion}<option>{criterion}</option>{/each}</select>
    <label for="comment-name">Short name</label><input id="comment-name" bind:value={commentTitle} maxlength="60" placeholder="Connect the evidence" />
    <label for="comment-body">Teacher-written block</label><textarea id="comment-body" bind:value={commentBody} rows="6" placeholder="Your evidence is relevant. Explain how…"></textarea>
    <div class="dialog-actions"><button class="outline" value="cancel">Cancel</button><button class="primary" type="button" on:click={addComment} disabled={!commentTitle.trim() || !commentBody.trim()}>Save block</button></div>
  </form>
</dialog>

<dialog bind:this={backupDialog} aria-labelledby="backup-title">
  <form method="dialog" on:submit={(event) => event.preventDefault()}>
    <div class="dialog-head"><div><p class="kicker">Optional upgrade</p><h2 id="backup-title">Encrypted desk backup</h2></div><button class="icon-button" value="cancel" aria-label="Close backup dialog">×</button></div>
    {#if paid}
      <p class="license-active">✓ Desk Pass active</p>
      <p>Choose a private passphrase. Encryption happens on this device; we cannot read the backup or recover the passphrase.</p>
      <label for="backup-passphrase">Backup passphrase</label><input id="backup-passphrase" type="password" bind:value={backupPassphrase} minlength="10" autocomplete="new-password" aria-describedby="passphrase-help" />
      <small id="passphrase-help">At least 10 characters. You need the same phrase to restore.</small>
      <div class="backup-actions"><button class="primary" type="button" on:click={() => cloudBackup('save')} disabled={backupBusy || backupPassphrase.length < 10}>{backupBusy ? 'Working…' : 'Save encrypted backup'}</button><button class="outline" type="button" on:click={() => cloudBackup('restore')} disabled={backupBusy || backupPassphrase.length < 10}>Restore backup</button><button class="danger-link" type="button" on:click={deleteCloudBackup} disabled={backupBusy}>Delete cloud backup</button></div>
    {:else}
      <p class="price"><strong>$29</strong> one time</p>
      <p>Desk Pass adds one encrypted cloud backup you can restore on another device. The complete local queue, comment bank, copy, CSV export, accessibility, and offline use stay free.</p>
      <a class="primary button-link" href={`${API_BASE}/checkout`}>Buy Desk Pass securely <span aria-hidden="true">↗</span></a>
      <hr />
      <h3>Have a license?</h3>
      <label for="license-token">Paste license token</label><input id="license-token" bind:value={license} autocomplete="off" spellcheck="false" />
      <button class="outline" type="button" on:click={restoreLicense}>Restore purchase</button>
      <p class="micro">Sociobot/Dodo is the merchant of record. Refunds are handled there and revoke the license. See <a href="/privacy">privacy</a> and <a href="/terms">terms</a>.</p>
    {/if}
    {#if licenseNote}<p class="notice" role="status">{licenseNote}</p>{/if}
  </form>
</dialog>

<div class="toast" class:show={toast} role="status" aria-live="polite">{toast}</div>
