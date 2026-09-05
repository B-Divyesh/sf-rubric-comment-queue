<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    CRITERIA, decryptWorkspace, download, emptyWorkspace, encryptWorkspace,
    feedbackText, parsePlainText, sampleWorkspace, toCsv, uid,
    type CommentBlock, type Submission, type Workspace
  } from './lib';

  const REAL_STORAGE_KEY = 'rcq_workspace:v1';
  const DEMO_STORAGE_KEY = 'demo:rcq_workspace:v1';
  const LICENSE_KEY = 'sb_license:rubric-comment-queue';
  const VERDICT_KEY = 'sb_license_verdict:rubric-comment-queue';
  const API_BASE = 'https://api.sociobot.in/api/v1/products/rubric-comment-queue';
  let route = typeof location === 'undefined' ? '/' : location.pathname;
  let demoMode = route === '/demo';
  let workspace: Workspace = emptyWorkspace();
  let hydrated = false;
  let online = typeof navigator === 'undefined' ? true : navigator.onLine;
  let toast = '';
  let routeAnnouncement = '';
  let current: Submission | undefined;
  let readyCount = 0;
  let filteredComments: CommentBlock[] = [];
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
  type CachedLicenseVerdict = { valid: boolean; checked: number };

  $: current = workspace.submissions.find((item) => item.id === workspace.currentId) ?? workspace.submissions[0];
  $: readyCount = workspace.submissions.filter((item) => item.status === 'ready').length;
  $: filteredComments = current ? workspace.comments.filter((comment) => comment.criterion === current?.criterion || comment.criterion === 'Whole response') : [];

  function titleFor(path: string): string {
    if (path === '/demo') return 'Demo — Rubric Comment Queue';
    if (path === '/privacy') return 'Privacy — Rubric Comment Queue';
    if (path === '/terms') return 'Terms — Rubric Comment Queue';
    return 'Rubric Comment Queue — review writing feedback';
  }

  function descriptionFor(path: string): string {
    if (path === '/privacy') return 'How Rubric Comment Queue stores teacher feedback and protects student writing.';
    if (path === '/terms') return 'Terms for using Rubric Comment Queue in a teacher-controlled feedback workflow.';
    if (path === '/demo') return 'Try a separate sample queue with three realistic student responses.';
    return 'Review teacher-written feedback, add a personal next step, and export the finished queue.';
  }

  function syncMetadata() {
    const canonical = `https://rubric-comment-queue.sociobot.in${route === '/' ? '/' : route}`;
    document.title = titleFor(route);
    document.querySelector<HTMLMetaElement>('meta[name="description"]')?.setAttribute('content', descriptionFor(route));
    document.querySelector<HTMLLinkElement>('link[rel="canonical"]')?.setAttribute('href', canonical);
    document.querySelector<HTMLMetaElement>('meta[property="og:title"]')?.setAttribute('content', titleFor(route));
    document.querySelector<HTMLMetaElement>('meta[property="og:description"]')?.setAttribute('content', descriptionFor(route));
    document.querySelector<HTMLMetaElement>('meta[property="og:url"]')?.setAttribute('content', canonical);
    document.querySelector<HTMLMetaElement>('meta[name="twitter:title"]')?.setAttribute('content', titleFor(route));
    document.querySelector<HTMLMetaElement>('meta[name="twitter:description"]')?.setAttribute('content', descriptionFor(route));
  }

  function announce(message: string) {
    toast = '';
    setTimeout(() => toast = message, 10);
  }

  function validWorkspace(value: unknown): value is Workspace {
    const candidate = value as Partial<Workspace> | null;
    return candidate?.version === 1 && Array.isArray(candidate.submissions) && Array.isArray(candidate.comments);
  }

  function readWorkspace(key: string, fallback: () => Workspace): Workspace {
    try {
      const saved = localStorage.getItem(key);
      if (!saved) return fallback();
      const parsed: unknown = JSON.parse(saved);
      if (validWorkspace(parsed)) return parsed;
      throw new Error('invalid workspace');
    } catch {
      localStorage.removeItem(key);
      announce('Saved data could not be read. A fresh workspace was opened.');
      return fallback();
    }
  }

  function loadRouteWorkspace() {
    demoMode = route === '/demo';
    if (demoMode) {
      workspace = readWorkspace(DEMO_STORAGE_KEY, sampleWorkspace);
      paid = false;
    }
    else if (route === '/') workspace = readWorkspace(REAL_STORAGE_KEY, emptyWorkspace);
  }

  function restoreLicenseForRealWorkspace() {
    if (demoMode) {
      paid = false;
      return;
    }
    license = localStorage.getItem(LICENSE_KEY) ?? '';
    const cached = cachedLicenseVerdict();
    paid = Boolean(license && cached?.valid);
    if (license) verifyLicense();
  }

  function persist(message?: string) {
    workspace.updatedAt = new Date().toISOString();
    workspace = { ...workspace, submissions: [...workspace.submissions], comments: [...workspace.comments] };
    try {
      localStorage.setItem(demoMode ? DEMO_STORAGE_KEY : REAL_STORAGE_KEY, JSON.stringify(workspace));
    } catch {
      announce('This browser could not save the latest change. Export a local backup, then remove unneeded responses.');
      return;
    }
    if (message) announce(message);
  }

  async function focusPageHeading() {
    await tick();
    document.querySelector<HTMLElement>('main h1')?.focus({ preventScroll: true });
    routeAnnouncement = titleFor(route);
  }

  function setRoute(path: string) {
    const leavingDemo = demoMode && path !== '/demo';
    if (leavingDemo) localStorage.removeItem(DEMO_STORAGE_KEY);
    history.pushState({}, '', path);
    route = path;
    loadRouteWorkspace();
    if (hydrated && !demoMode) restoreLicenseForRealWorkspace();
    syncMetadata();
    focusPageHeading();
  }

  function followRoute(event: MouseEvent, path: string) {
    if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    setRoute(path);
  }

  function enterDemo() {
    workspace = sampleWorkspace();
    localStorage.setItem(DEMO_STORAGE_KEY, JSON.stringify(workspace));
    demoMode = true;
    setRoute('/demo');
    announce('Sample queue loaded. Your real workspace was not changed.');
  }

  function resetDemo() {
    workspace = sampleWorkspace();
    localStorage.setItem(DEMO_STORAGE_KEY, JSON.stringify(workspace));
    announce('Sample queue reset to three responses.');
  }

  function startForReal() {
    localStorage.removeItem(DEMO_STORAGE_KEY);
    demoMode = false;
    setRoute('/');
    announce('Demo data was discarded. Your real workspace is open.');
  }

  function updateCurrent(values: Partial<Submission>) {
    if (!current) return;
    const index = workspace.submissions.indexOf(current);
    const updated = { ...current, ...values };
    updated.updatedAt = new Date().toISOString();
    if (updated.status === 'new' && (values.draft !== undefined || values.nextStep !== undefined)) updated.status = 'draft';
    workspace.submissions[index] = updated;
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
    importError = '';
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
    const next = workspace.submissions.slice(index + 1).find((item) => item.status !== 'ready') ?? workspace.submissions.find((item) => item.status !== 'ready');
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
    const label = current.label;
    const index = workspace.submissions.indexOf(current);
    workspace.submissions.splice(index, 1);
    workspace.currentId = workspace.submissions[Math.min(index, workspace.submissions.length - 1)]?.id ?? null;
    persist(`${label} deleted.`);
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

  function cachedLicenseVerdict(): CachedLicenseVerdict | null {
    try {
      const raw = localStorage.getItem(VERDICT_KEY);
      if (!raw) return null;
      const cached = JSON.parse(raw) as Partial<CachedLicenseVerdict>;
      if (typeof cached.valid === 'boolean' && typeof cached.checked === 'number' && Number.isFinite(cached.checked)) return cached as CachedLicenseVerdict;
      throw new Error('invalid cached license verdict');
    } catch {
      localStorage.removeItem(VERDICT_KEY);
      announce('Saved license status could not be read. It was removed; your free workspace is ready.');
      return null;
    }
  }

  async function verifyLicense(force = false) {
    if (!license || !online) return;
    const cached = cachedLicenseVerdict();
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

  async function cloudBackup(mode: 'save' | 'restore') {
    if (demoMode || !paid || !license) return;
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
    if (demoMode || !paid || !license || !confirm('Delete the encrypted cloud backup? Your local workspace will stay on this device.')) return;
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
    loadRouteWorkspace();
    theme = localStorage.getItem('rcq_theme') === 'dark' ? 'dark' : 'light';
    document.documentElement.dataset.theme = theme;
    syncMetadata();
    const params = new URLSearchParams(location.search);
    const returned = params.get('license');
    if (returned && !demoMode) {
      localStorage.setItem(LICENSE_KEY, returned);
      params.delete('license');
      history.replaceState({}, '', `${route}${params.size ? `?${params}` : ''}${location.hash}`);
    } else if (returned) {
      params.delete('license');
      history.replaceState({}, '', `${route}${params.size ? `?${params}` : ''}${location.hash}`);
    }
    if (!demoMode) {
      license = returned ?? localStorage.getItem(LICENSE_KEY) ?? '';
      const cached = cachedLicenseVerdict();
      paid = Boolean(license && cached?.valid);
      if (license) verifyLicense();
    }
    const goOnline = () => { online = true; verifyLicense(); };
    const goOffline = () => online = false;
    const goBack = () => {
      const leavingDemo = demoMode && location.pathname !== '/demo';
      if (leavingDemo) localStorage.removeItem(DEMO_STORAGE_KEY);
      route = location.pathname;
      loadRouteWorkspace();
      if (!demoMode) restoreLicenseForRealWorkspace();
      syncMetadata();
      focusPageHeading();
    };
    addEventListener('online', goOnline);
    addEventListener('offline', goOffline);
    addEventListener('keydown', keyHandler);
    addEventListener('popstate', goBack);
    hydrated = true;
    if (!demoMode) fetch('/api/pageview', { method: 'POST', keepalive: true }).catch(() => undefined);
    return () => {
      removeEventListener('online', goOnline);
      removeEventListener('offline', goOffline);
      removeEventListener('keydown', keyHandler);
      removeEventListener('popstate', goBack);
    };
  });
</script>

<svelte:head><meta name="color-scheme" content={theme === 'dark' ? 'dark' : 'light'} /></svelte:head>

<a class="skip-link" href="#main">Skip to main content</a>
<div class="route-status visually-hidden" aria-live="polite">{routeAnnouncement}</div>
<header class="topbar">
  <a class="brand" href="/" on:click={(event) => followRoute(event, '/')} aria-label="Rubric Comment Queue home"><img src="/mark.svg" width="36" height="36" alt="" /><span>Rubric<br />Comment Queue</span></a>
  <nav aria-label="Main navigation">
    <a href="/demo" on:click={(event) => followRoute(event, '/demo')}>Demo</a>
    <a href="/#how-it-works">How it works</a>
    <a href="/privacy" on:click={(event) => followRoute(event, '/privacy')}>Privacy</a>
    <span class:offline={!online} class="connection"><span aria-hidden="true">●</span> {online ? 'Local save on' : 'Offline · local save on'}</span>
    <button class="icon-button" type="button" on:click={toggleTheme} aria-label={`Use ${theme === 'light' ? 'dark' : 'light'} theme`}>{theme === 'light' ? '◐' : '◑'}</button>
    {#if paid && !demoMode}<button class="outline small" type="button" on:click={() => backupDialog.showModal()}>Encrypted backup</button>{/if}
  </nav>
</header>

{#if demoMode}
  <aside class="demo-banner" aria-label="Demo controls"><p><strong>Demo — sample data, nothing is saved to your real workspace</strong></p><div><button type="button" class="text-button" on:click={resetDemo}>Reset demo</button><button type="button" class="outline" on:click={startForReal}>Start for real</button></div></aside>
{/if}

{#if route === '/privacy' || route === '/terms'}
  <main id="main" class="legal-page">
    <a class="back-link" href="/" on:click={(event) => followRoute(event, '/')}>← Back to the queue</a>
    {#if route === '/privacy'}
      <h1 tabindex="-1">Privacy for your feedback queue</h1>
      <p class="lede">Student writing stays in this browser during the free review workflow.</p>
      <h2>Data on this device</h2>
      <p>Labels, excerpts, comment blocks, drafts, and next steps use this browser’s local storage.</p>
      <p>The demo uses a separate storage key. It never reads or changes your real workspace.</p>
      <h2>Service data</h2>
      <p>We count aggregate page views. We do not load analytics scripts, advertising cookies, or third-party fonts.</p>
      <p>The app does not send student writing to an AI model or create student profiles.</p>
      <h2>Your controls</h2>
      <p>Delete responses inside the queue. Clear this site’s browser data to remove the whole local workspace.</p>
      <p>Email privacy@sociobot.in if you need help with a privacy request.</p>
    {:else}
      <h1 tabindex="-1">Terms for the feedback queue</h1>
      <p class="lede">Rubric Comment Queue supports teacher-controlled writing feedback.</p>
      <h2>Your responsibility</h2>
      <p>You decide what feedback is appropriate. Review every comment before sharing it with a student.</p>
      <p>The app does not score writing, detect plagiarism, or make educational decisions.</p>
      <h2>Appropriate data</h2>
      <p>Use initials or roster numbers where possible. Follow your school’s rules and applicable privacy law.</p>
      <h2>Availability</h2>
      <p>The service comes without a guarantee for every school requirement. Download local backups regularly.</p>
      <p>This release does not sell a paid plan. Core review and export remain available without an account.</p>
      <p><strong>Effective:</strong> September 5, 2026.</p>
    {/if}
  </main>
{:else}
  <main id="main" class="app-shell" class:demo-shell={demoMode} aria-busy={!hydrated}>
    <section class="intro" aria-labelledby="page-title">
      <div class="intro-copy">
        <p class="eyebrow">Teacher-controlled writing feedback</p>
        <h1 id="page-title" tabindex="-1">Review writing feedback before you send it</h1>
        <p class="audience">For teachers handling many responses, this queue keeps every comment in a review step under your control.</p>
        <div class="hero-actions">
          {#if demoMode}
            <button class="primary" type="button" on:click={resetDemo}>Reset sample data</button><button class="outline" type="button" on:click={startForReal}>Start for real</button>
          {:else}
            <button class="primary" type="button" on:click={enterDemo}>Try it with sample data</button><span>Loads three sample responses in a separate workspace.</span><button class="outline" type="button" on:click={openImport}>Add my responses</button>
          {/if}
        </div>
        <ul class="plain-facts"><li>Student writing stays in this browser.</li><li>No account is needed.</li><li>Review, copy, and CSV export are free.</li></ul>
      </div>
      {#if workspace.submissions.length}<div class="progress-card" aria-label={`${readyCount} of ${workspace.submissions.length} responses ready`}><span><strong>{readyCount}</strong> / {workspace.submissions.length} ready</span><div class="progress-track"><span style={`width:${workspace.submissions.length ? readyCount / workspace.submissions.length * 100 : 0}%`}></span></div></div>{/if}
    </section>

    <section class="product-section" aria-labelledby="workspace-title">
      <h2 id="workspace-title" class="section-title">Review queue</h2>
      {#if workspace.submissions.length === 0}
        <div class="empty-state">
          <picture><source media="(max-width: 640px)" srcset="/queue-desk-640.webp" /><img src="/queue-desk.webp" width="960" height="640" alt="Paper excerpts move through a blue rubric tray and an orange teacher stamp into a feedback stack" fetchpriority="high" /></picture>
          <div class="empty-copy"><h3>Add responses to start reviewing</h3><p>Paste excerpts or import a plain-text file. Put three dashes on a separate line between responses.</p><p>Labels are optional. Use initials or roster numbers instead of student names.</p><button class="primary" type="button" on:click={openImport}>Add responses <span aria-hidden="true">→</span></button></div>
        </div>
      {:else}
        <div class="workspace">
          <aside class="queue" aria-labelledby="queue-title">
            <div class="section-heading"><div><p class="kicker">Step 1</p><h3 id="queue-title">Response queue</h3></div><button class="square-button" type="button" on:click={openImport} aria-label="Add responses">+</button></div>
            <ol>{#each workspace.submissions as item, index (item.id)}<li class:active={item.id === current?.id} class:ready={item.status === 'ready'}><button type="button" on:click={() => { workspace.currentId = item.id; persist(); }} aria-current={item.id === current?.id ? 'step' : undefined}><span class="queue-number">{String(index + 1).padStart(2, '0')}</span><span><strong>{item.label}</strong><small>{item.status === 'ready' ? '✓ Ready' : item.status === 'draft' ? 'In review' : 'Not started'}</small></span></button></li>{/each}</ol>
            <button class="outline full" type="button" on:click={exportCsv}>Export all CSV</button><button class="text-button full" type="button" on:click={exportLocalBackup}>Download local backup</button>
          </aside>

          {#if current}
            <article class="review-sheet" aria-labelledby="review-title">
              <div class="sheet-topline"><span>{current.status === 'ready' ? '✓ Ready to send' : 'Now reviewing'}</span><span>Saved {demoMode ? 'in demo' : 'locally'}</span></div>
              <div class="sheet-heading"><div><p class="kicker">Step 2 · Read and respond</p><h3 id="review-title">{current.label}</h3></div><button class="danger-link" type="button" on:click={removeCurrent}>Delete</button></div>
              <section class="excerpt" aria-labelledby="excerpt-title"><h4 id="excerpt-title">Student excerpt</h4><blockquote>{current.excerpt}</blockquote></section>
              <div class="field-row"><label for="criterion">Rubric criterion</label><select id="criterion" value={current.criterion} on:change={(event) => updateCurrent({ criterion: event.currentTarget.value, commentId: '' })}>{#each CRITERIA as criterion}<option value={criterion}>{criterion}</option>{/each}</select></div>
              <div class="field-block"><label for="feedback-draft">Feedback draft <span>Edit the selected block in your own words</span></label><textarea id="feedback-draft" rows="7" value={current.draft} on:input={(event) => updateCurrent({ draft: event.currentTarget.value })} placeholder="Choose a comment block or write your own feedback…"></textarea><small class="count">{current.draft.length} characters</small></div>
              <div class="field-block next-step"><label for="next-step">One personal next step <span>Required to mark ready</span></label><textarea id="next-step" rows="3" value={current.nextStep} on:input={(event) => updateCurrent({ nextStep: event.currentTarget.value })} placeholder="For your next draft, try…"></textarea></div>
              <div class="sheet-actions"><div><button class="outline" type="button" on:click={() => move(-1)} disabled={workspace.submissions.indexOf(current) === 0}>← Back</button><button class="outline" type="button" on:click={copyCurrent} disabled={!current.draft.trim()}>Copy feedback</button></div><button class="primary" type="button" on:click={saveAndNext}>Save and next <span class="key-hint">⌘↵</span></button></div>
            </article>

            <aside class="comment-bank" aria-labelledby="bank-title">
              <div class="section-heading"><div><p class="kicker">Step 3</p><h3 id="bank-title">Comment bank</h3></div><button class="square-button" type="button" on:click={openNewComment} aria-label="Add a comment block">+</button></div>
              <p class="bank-help">Blocks for <strong>{current.criterion}</strong>. Choose one, then make it specific.</p>
              {#if filteredComments.length}<ul>{#each filteredComments as comment (comment.id)}<li><button class:selected={current.commentId === comment.id} class="comment-block" type="button" on:click={() => chooseComment(comment)}><span class="comment-meta"><strong>{comment.title}</strong>{#if comment.custom}<em>Yours</em>{/if}</span><span>{comment.body}</span></button>{#if comment.custom}<button class="remove-block" type="button" on:click={() => deleteComment(comment)} aria-label={`Remove ${comment.title}`}>×</button>{/if}</li>{/each}</ul>{:else}<div class="mini-empty"><p>No blocks exist for this criterion.</p><button class="text-button" type="button" on:click={openNewComment}>Write a block</button></div>{/if}
            </aside>
          {/if}
        </div>
      {/if}
    </section>

    <section id="how-it-works" class="info-section" aria-labelledby="how-title"><h2 id="how-title">How it works</h2><ol class="steps"><li><strong>1. Add responses.</strong><span>Paste text or choose a plain-text file.</span></li><li><strong>2. Review each comment.</strong><span>Choose a criterion and edit teacher-written feedback.</span></li><li><strong>3. Export the queue.</strong><span>Add one personal next step, then copy or export CSV.</span></li></ol></section>
    <section class="info-section limits" aria-labelledby="limits-title"><h2 id="limits-title">Limits and privacy</h2><p>The app does not score essays, detect plagiarism, generate comments, or profile students.</p><p>Your queue uses browser storage. Download a local backup before clearing browser data.</p><a href="/privacy" on:click={(event) => followRoute(event, '/privacy')}>Read the privacy details</a></section>
  </main>
{/if}

<footer>
  <p><strong>Review teacher-written feedback before export.</strong></p>
  <nav aria-label="Footer navigation"><a href="/privacy" on:click={(event) => followRoute(event, '/privacy')}>Privacy</a><a href="/terms" on:click={(event) => followRoute(event, '/terms')}>Terms</a><a href="https://sociobot.in" rel="noreferrer" aria-label="Built by Param Factory (external site)">Built by Param Factory <span aria-hidden="true">↗</span></a></nav>
  <p>v1.1.1 · Illustration generated for Rubric Comment Queue.</p>
</footer>

<dialog bind:this={importDialog} aria-labelledby="import-title"><form method="dialog" on:submit={(event) => event.preventDefault()}><div class="dialog-head"><div><p class="kicker">Add to queue</p><h2 id="import-title">Import responses</h2></div><button class="icon-button" type="button" on:click={() => importDialog.close()} aria-label="Close import dialog">×</button></div><p>Paste plain text below. Put an optional <code># label</code> on the first line.</p><p>Put three dashes on a separate line between responses.</p><label class="file-button" for="text-file">Choose .txt file</label><input id="text-file" class="visually-hidden" type="file" accept=".txt,text/plain" on:change={readFile} /><label for="import-text">Response text</label><textarea id="import-text" bind:value={importText} rows="10" aria-describedby={importError ? 'import-error' : 'import-help'} placeholder="# Roster 12&#10;The opening paragraph…&#10;&#10;---&#10;&#10;# Roster 13&#10;In this response…"></textarea><small id="import-help">Maximum file size: 1 MB.</small>{#if importError}<p id="import-error" class="error" role="alert">{importError}</p>{/if}<div class="dialog-actions"><button class="outline" type="button" on:click={() => importDialog.close()}>Cancel</button><button class="primary" type="button" on:click={runImport}>Add to queue</button></div></form></dialog>

<dialog bind:this={commentDialog} aria-labelledby="comment-title"><form method="dialog" on:submit={(event) => event.preventDefault()}><div class="dialog-head"><div><p class="kicker">Reusable comment</p><h2 id="comment-title">Write a comment block</h2></div><button class="icon-button" type="button" on:click={() => commentDialog.close()} aria-label="Close comment dialog">×</button></div><label for="comment-criterion">Rubric criterion</label><select id="comment-criterion" bind:value={commentCriterion}>{#each CRITERIA as criterion}<option>{criterion}</option>{/each}</select><label for="comment-name">Short name</label><input id="comment-name" bind:value={commentTitle} maxlength="60" placeholder="Connect the evidence" /><label for="comment-body">Teacher-written block</label><textarea id="comment-body" bind:value={commentBody} rows="6" placeholder="Your evidence is relevant. Explain how…"></textarea><div class="dialog-actions"><button class="outline" type="button" on:click={() => commentDialog.close()}>Cancel</button><button class="primary" type="button" on:click={addComment} disabled={!commentTitle.trim() || !commentBody.trim()}>Save block</button></div></form></dialog>

{#if paid}<dialog bind:this={backupDialog} aria-labelledby="backup-title"><form method="dialog" on:submit={(event) => event.preventDefault()}><div class="dialog-head"><div><p class="kicker">Existing license</p><h2 id="backup-title">Encrypted backup</h2></div><button class="icon-button" type="button" on:click={() => backupDialog.close()} aria-label="Close backup dialog">×</button></div><p class="license-active">✓ Desk Pass active</p><p>Choose a private passphrase. Encryption happens on this device.</p><label for="backup-passphrase">Backup passphrase</label><input id="backup-passphrase" type="password" bind:value={backupPassphrase} minlength="10" autocomplete="new-password" aria-describedby="passphrase-help" /><small id="passphrase-help">Use at least 10 characters. The same phrase restores the backup.</small><div class="backup-actions"><button class="primary" type="button" on:click={() => cloudBackup('save')} disabled={backupBusy || backupPassphrase.length < 10}>{backupBusy ? 'Working…' : 'Save encrypted backup'}</button><button class="outline" type="button" on:click={() => cloudBackup('restore')} disabled={backupBusy || backupPassphrase.length < 10}>Restore backup</button><button class="danger-link" type="button" on:click={deleteCloudBackup} disabled={backupBusy}>Delete cloud backup</button></div>{#if licenseNote}<p class="notice" role="status">{licenseNote}</p>{/if}</form></dialog>{/if}

<div class="toast" class:show={toast} role="status" aria-live="polite">{toast}</div>
