<script lang="ts">
  import type { RadciteArchiveItem, RadciteArchiveItemKind } from "../types";

  type Props = {
    items: RadciteArchiveItem[];
    loading: boolean;
    error: string | null;
    onRefresh: () => void | Promise<void>;
    onRestore: (item: RadciteArchiveItem) => void | Promise<void>;
  };

  let { items, loading, error, onRefresh, onRestore }: Props = $props();
  let restoringId = $state<string | null>(null);

  const sections: Array<{ kind: RadciteArchiveItemKind; label: string }> = [
    { kind: "document", label: "Documents" },
    { kind: "module", label: "Modules" },
    { kind: "course_reference", label: "Course references" },
    { kind: "module_reading", label: "Module readings" },
  ];

  function sectionItems(kind: RadciteArchiveItemKind): RadciteArchiveItem[] {
    return items.filter((item) => item.kind === kind);
  }

  function formattedDate(value: string): string {
    const parsed = new Date(value);
    return Number.isNaN(parsed.valueOf())
      ? value
      : new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(
          parsed,
        );
  }

  async function restore(item: RadciteArchiveItem) {
    if (restoringId) {
      return;
    }

    restoringId = item.id;
    try {
      await onRestore(item);
    } finally {
      restoringId = null;
    }
  }
</script>

<section class="archive-workspace" aria-labelledby="archive-heading">
  <div class="workspace-header">
    <div>
      <p class="eyebrow">RADcite</p>
      <h2 id="archive-heading">Archive</h2>
      <p class="workspace-subtitle">Restore documents, modules, readings, and references from this project.</p>
    </div>
    <button class="secondary-button compact-button" type="button" disabled={loading} onclick={() => void onRefresh()}>
      Refresh
    </button>
  </div>

  {#if error}
    <div class="notice archive-notice">{error}</div>
  {:else if loading}
    <div class="archive-empty">Loading archived items</div>
  {:else if items.length === 0}
    <div class="archive-empty">Nothing is currently archived in this project.</div>
  {:else}
    <div class="archive-groups">
      {#each sections as section (section.kind)}
        {@const sectionItemsList = sectionItems(section.kind)}
        {#if sectionItemsList.length}
          <section class="archive-group" aria-labelledby={`${section.kind}-archive-heading`}>
            <div class="archive-group-heading">
              <h3 id={`${section.kind}-archive-heading`}>{section.label}</h3>
              <span>{sectionItemsList.length}</span>
            </div>
            <div class="archive-list">
              {#each sectionItemsList as item (item.id)}
                <article class="archive-row">
                  <div class="archive-row-copy">
                    <strong>{item.label}</strong>
                    {#if item.detail}<span>{item.detail}</span>{/if}
                    <small>Archived {formattedDate(item.archived_at)}</small>
                  </div>
                  <button
                    class="secondary-button compact-button"
                    type="button"
                    disabled={restoringId !== null}
                    onclick={() => void restore(item)}
                  >
                    {restoringId === item.id ? "Restoring" : "Restore"}
                  </button>
                </article>
              {/each}
            </div>
          </section>
        {/if}
      {/each}
    </div>
  {/if}
</section>
