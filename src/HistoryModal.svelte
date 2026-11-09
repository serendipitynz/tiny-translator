<script lang="ts">
  import { tick } from "svelte";
  import {
    clearHistory,
    countHistory,
    deleteHistoryEntry,
    directionLabel,
    emptyListWords,
    entryName,
    type HistoryEntry,
    savedAtLabel,
    searchHistory,
    searchTerms,
  } from "./lib/history";
  import { announce } from "./lib/ui/announce";
  import Button from "./lib/ui/Button.svelte";
  import IconButton from "./lib/ui/IconButton.svelte";
  import Modal from "./lib/ui/Modal.svelte";
  import TextInput from "./lib/ui/TextInput.svelte";

  interface Props {
    /** A translation was picked to be put back on the screen; the caller closes the modal. */
    onpick: (entry: HistoryEntry) => void;
    onclose: () => void;
    /** Tells the result of an operation done here (snz-design doc-9 §6.5). */
    notify: (words: string) => void;
  }

  let { onpick, onclose, notify }: Props = $props();

  let query = $state("");
  /** The translations the search found, or null until the first search has answered. */
  let entries = $state<HistoryEntry[] | null>(null);
  /**
   * The search `entries` answers. The count and the empty list speak of it rather than of `query`,
   * which runs ahead of the list while the typing pauses and stays ahead when the search fails.
   */
  let shownQuery = $state("");
  /** How many translations the history holds, whatever the search; null when it could not be read. */
  let total = $state<number | null>(null);
  /** Why the list could not be read or changed, told beside it (doc-9 §5.5). */
  let failure = $state<string | null>(null);
  let searching = $state(false);
  let confirmingClear = $state(false);
  let clearing = $state(false);
  let clearFailure = $state<string | null>(null);
  /** Counts searches, so an answer that arrives after a newer one was asked for is dropped. */
  let current = 0;
  let list: HTMLElement;
  let search: TextInput;

  // Long enough that a word typed at speed is searched for once, short enough to read as immediate.
  const SEARCH_DELAY_MS = 200;

  let filtered = $derived(searchTerms(shownQuery).length > 0);
  /** Whether the search narrows a history that holds something: an empty list is then the search's doing (doc-9 §5.4). */
  let narrowing = $derived(filtered && total !== 0);

  function fail(words: string) {
    failure = words;
    announce(words);
  }

  async function load(text: string) {
    const id = ++current;
    searching = true;
    try {
      const [found, count] = await Promise.all([searchHistory(text), countHistory()]);
      if (id === current) {
        entries = found;
        shownQuery = text;
        total = count;
        failure = null;
      }
    } catch (error) {
      // doc-9 §5.5: what was listed before stays, so a failure does not read as an empty history.
      if (id === current) {
        fail(`翻訳履歴を読み込めませんでした: ${error}`);
      }
    } finally {
      if (id === current) {
        searching = false;
      }
    }
  }

  /** Whether the list was asked for yet; plain rather than state, so the effect below reruns on `query` alone. */
  let asked = false;

  $effect(() => {
    const text = query;
    // The first list is read at once; later ones wait for the typing to pause.
    if (!asked) {
      asked = true;
      void load(text);
      return;
    }
    const timer = setTimeout(() => void load(text), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  });

  async function clearSearch() {
    query = "";
    await tick();
    search.focus();
  }

  async function remove(entry: HistoryEntry) {
    let gone: boolean;
    try {
      gone = await deleteHistoryEntry(entry.id);
    } catch (error) {
      fail(`翻訳を削除できませんでした: ${error}`);
      return;
    }
    if (entries === null) {
      return;
    }
    const at = entries.findIndex((listed) => listed.id === entry.id);
    entries = entries.filter((listed) => listed.id !== entry.id);
    if (gone && total !== null) {
      total -= 1;
    }
    failure = null;
    notify("翻訳を削除しました");
    // doc-9 §5.1: the delete button that held the focus is gone, so the focus moves to the nearest one
    // left — the row that took its place, else the one above, else the search — and deleting several
    // in a row stays one key per row.
    await tick();
    const rows = list.querySelectorAll<HTMLElement>("li button:not(.entry)");
    const next = rows[Math.min(at, rows.length - 1)];
    if (next === undefined) {
      search.focus();
    } else {
      next.focus();
    }
  }

  async function clearAll() {
    clearing = true;
    clearFailure = null;
    try {
      await clearHistory();
    } catch (error) {
      // doc-9 §6.6: the confirmation stays open, so the failure is read against what was asked.
      clearing = false;
      clearFailure = `翻訳履歴を削除できませんでした: ${error}`;
      announce(clearFailure);
      return;
    }
    clearing = false;
    confirmingClear = false;
    entries = [];
    total = 0;
    failure = null;
    notify("翻訳履歴をすべて削除しました");
  }

  function cancelClear() {
    confirmingClear = false;
    clearFailure = null;
  }

  let clearBlocker = $derived(total === 0 ? "削除する翻訳がありません" : null);
  let countLabel = $derived(
    entries === null ? "" : narrowing && total !== null ? `${entries.length} 件 / 全 ${total} 件` : `${entries.length} 件`,
  );
</script>

<Modal heading="翻訳履歴" {onclose}>
  <section class="list" aria-label="翻訳履歴の一覧" aria-busy={searching ? "true" : undefined} bind:this={list}>
    <div class="list-head">
      <div class="search"><TextInput label="検索" bind:value={query} autofocus bind:this={search} /></div>
      <p class="count" aria-live="polite">{countLabel}</p>
      <Button label="全件削除" variant="danger" disabledReason={clearBlocker} onpress={() => (confirmingClear = true)} />
    </div>

    {#if failure !== null}
      <p class="failure">{failure}</p>
    {/if}

    {#if entries === null}
      {#if failure === null}
        <!-- doc-9 §5.4: waiting is not shown as empty, which would invite acting before the list is in. -->
        <p class="loading"><span class="spinner"></span>読み込み中…</p>
      {/if}
    {:else if entries.length === 0}
      <div class="empty">
        <p>{emptyListWords(narrowing ? shownQuery : "")}</p>
        {#if narrowing}
          <!-- doc-9 §5.4: an empty result names its condition and offers to lift it. -->
          <Button label="検索語をクリア" onpress={clearSearch} />
        {/if}
      </div>
    {:else}
      <div class="columns" aria-hidden="true">
        <span>日時</span><span>言語</span><span>原文</span><span>訳文</span>
      </div>
      <ul>
        {#each entries as entry (entry.id)}
          <li>
            <button type="button" class="entry" aria-label={entryName(entry)} onclick={() => onpick(entry)}>
              <span class="at">{savedAtLabel(entry.createdAt)}</span>
              <span class="direction">{directionLabel(entry)}</span>
              <span class="text">{entry.sourceText}</span>
              <span class="text">{entry.translation}</span>
            </button>
            <IconButton
              icon="trash-2"
              label={`${savedAtLabel(entry.createdAt)} の翻訳を削除`}
              onpress={() => remove(entry)}
            />
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  {#snippet actions()}
    <Button label="閉じる" onpress={onclose} />
  {/snippet}
</Modal>

{#if confirmingClear}
  <Modal
    heading="翻訳履歴をすべて削除しますか"
    closeBlocker={clearing ? "削除が終わるまで閉じられません" : null}
    onclose={cancelClear}
  >
    <!-- doc-9 §5.7: what goes and that it cannot be undone, in words. -->
    <p class="confirm">
      {total === null ? "保存しているすべての翻訳" : `保存している ${total} 件の翻訳`}を削除します。{filtered
        ? "検索で絞り込んでいるものに限らず、すべて削除します。"
        : ""}削除した翻訳は元に戻せません。
    </p>
    {#if clearFailure !== null}
      <p class="failure">{clearFailure}</p>
    {/if}
    {#snippet actions()}
      <Button
        label="キャンセル"
        autofocus
        disabledReason={clearing ? "削除が終わるまで取り消せません" : null}
        onpress={cancelClear}
      />
      <Button label="すべて削除" busyLabel="削除中…" variant="danger" busy={clearing} onpress={clearAll} />
    {/snippet}
  </Modal>
{/if}

<style lang="scss">
  @use "./styles/mixins" as *;

  // doc-9 §6.1's 行の並び. The four columns are shared by the column names and every row, so they line
  // up; the delete button takes the last track of the outer grid.
  .list {
    --history-columns: 8.5rem 9rem minmax(0, 1fr) minmax(0, 1fr);

    container: history / inline-size;
    display: grid;
    gap: var(--snz-space-sm);
  }

  .list-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--snz-space-sm) var(--snz-space-md);
  }

  .search {
    flex: 1 1 14rem;
    min-inline-size: 0;
  }

  .count {
    margin: 0;
    font-size: var(--snz-font-size-small);
    color: var(--snz-fg-muted);
  }

  .columns,
  li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--snz-size-control);
    align-items: center;
    gap: var(--snz-space-xs);
  }

  .columns {
    grid-template-columns: var(--history-columns) var(--snz-size-control);
    column-gap: var(--snz-space-md);
    padding: 0 var(--snz-space-sm);
    font-size: var(--snz-font-size-small);
    color: var(--snz-fg-muted);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    border-block-start: var(--snz-border-line) solid var(--snz-line-divider);
  }

  li {
    padding-inline-end: var(--snz-space-xs);
    border-block-end: var(--snz-border-line) solid var(--snz-line-divider);
  }

  .entry {
    display: grid;
    grid-template-columns: var(--history-columns);
    align-items: start;
    gap: var(--snz-space-xs) var(--snz-space-md);
    min-inline-size: 0;
    padding: var(--snz-space-row-y) var(--snz-space-sm);
    font: inherit;
    color: var(--snz-fg);
    text-align: start;
    background: var(--snz-surface);
    border: 0;
    border-radius: var(--snz-radius-sm);
    cursor: pointer;
    transition: background-color var(--snz-motion-state);

    &:hover {
      background: var(--snz-surface-hover);
    }

    &:active {
      background: var(--snz-surface-pressed);
    }

    &:focus-visible {
      @include focus-ring;
    }
  }

  .at,
  .direction {
    font-size: var(--snz-font-size-small);
    color: var(--snz-fg-muted);
  }

  // doc-9 §6.1: cut short to keep the rows one height; picking the row puts the whole text back. Line
  // breaks fold into spaces, so the two lines shown are text rather than a heading and a blank line.
  .text {
    display: -webkit-box;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow-wrap: anywhere;
  }

  // doc-9 §5.4: the words of an empty list are faint but readable.
  .empty {
    display: grid;
    justify-items: start;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-md) var(--snz-space-sm);

    p {
      margin: 0;
      color: var(--snz-fg-faint);
    }
  }

  .loading {
    display: flex;
    align-items: center;
    gap: var(--snz-space-xs);
    margin: 0;
    padding: var(--snz-space-md) var(--snz-space-sm);
    color: var(--snz-fg-muted);
  }

  // doc-8 §6.7, as Button draws it.
  .spinner {
    box-sizing: border-box;
    inline-size: var(--snz-icon-size-sm);
    block-size: var(--snz-icon-size-sm);
    border: var(--snz-border-focus) solid var(--snz-figure);
    border-inline-end-color: transparent;
    border-radius: 50%;
    animation: spin var(--snz-motion-spin) linear infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation-duration: var(--snz-motion-spin-reduced);
    }
  }

  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }

  // Told in words, not colour alone (doc-8 §5.8).
  .failure {
    margin: 0;
    color: var(--snz-danger);
    overflow-wrap: anywhere;
  }

  .confirm {
    margin: 0;
  }

  // doc-9 §6.1: on a narrow list the rows become cards — when and which way on one line, the source
  // and the translation under it — and the column names go with the columns.
  @container history (inline-size < 36rem) {
    .columns {
      display: none;
    }

    .entry {
      grid-template-columns: auto minmax(0, 1fr);

      .text {
        grid-column: 1 / -1;
      }
    }
  }
</style>
