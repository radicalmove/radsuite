<script lang="ts">
  import { helpFaqs, helpSections } from "../lib/helpContent";

  type Props = {
    open: boolean;
    onClose: () => void;
  };

  let { open, onClose }: Props = $props();

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      onClose();
    }
  }

  $effect(() => {
    if (!open) {
      return;
    }

    document.addEventListener("keydown", handleKeydown);
    return () => document.removeEventListener("keydown", handleKeydown);
  });
</script>

{#if open}
  <div
    class="help-backdrop"
    role="presentation"
    onclick={(event) => {
      if (event.target === event.currentTarget) {
        onClose();
      }
    }}
  >
    <div
      class="help-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="help-heading"
      aria-describedby="help-intro"
      tabindex="-1"
    >
      <header class="help-dialog-header">
        <div>
          <p class="eyebrow">RADsuite</p>
          <h2 id="help-heading">Help and quick guide</h2>
          <p id="help-intro">A short guide to the local-first RADcite workflow.</p>
        </div>
        <button class="secondary-button compact-button" type="button" onclick={onClose}>
          Close
        </button>
      </header>

      <div class="help-dialog-content">
        <div class="help-guide-list">
          {#each helpSections as section (section.id)}
            <section class="help-guide-section" aria-labelledby={`${section.id}-heading`}>
              <h3 id={`${section.id}-heading`}>{section.title}</h3>
              <p>{section.summary}</p>
              <ol>
                {#each section.steps as step}
                  <li>{step}</li>
                {/each}
              </ol>
            </section>
          {/each}
        </div>

        <section class="help-faq" aria-labelledby="help-faq-heading">
          <h3 id="help-faq-heading">Frequently asked questions</h3>
          {#each helpFaqs as faq}
            <details>
              <summary>{faq.question}</summary>
              <p>{faq.answer}</p>
            </details>
          {/each}
        </section>
      </div>
    </div>
  </div>
{/if}
