<script lang="ts">
  import { tick } from "svelte";

  type Props = {
    version: string;
    checkingForUpdate: boolean;
    onCheckForUpdates: () => void;
    onOpenHelp: () => void;
  };

  let { version, checkingForUpdate, onCheckForUpdates, onOpenHelp }: Props = $props();

  const menuId = "application-menu";
  let open = $state(false);
  let container = $state<HTMLDivElement>();
  let hamburger = $state<HTMLButtonElement>();
  let firstAction = $state<HTMLButtonElement>();

  async function toggleMenu() {
    open = !open;
    if (open) {
      await tick();
      firstAction?.focus();
    }
  }

  function handleWindowClick(event: MouseEvent) {
    if (open && !container?.contains(event.target as Node)) open = false;
  }

  async function handleWindowKeydown(event: KeyboardEvent) {
    if (!open || event.key !== "Escape") return;
    open = false;
    await tick();
    hamburger?.focus();
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleWindowKeydown} />

<div class="application-menu" bind:this={container}>
  <button
    bind:this={hamburger}
    type="button"
    aria-label="Open application menu"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls={menuId}
    onclick={toggleMenu}
  >
    ☰
  </button>

  {#if open}
    <div id={menuId} role="menu">
      <button
        bind:this={firstAction}
        type="button"
        role="menuitem"
        disabled={checkingForUpdate}
        onclick={onCheckForUpdates}
      >
        {checkingForUpdate ? "Checking for updates…" : "Check for updates"}
      </button>
      <button type="button" role="menuitem" onclick={onOpenHelp}>Help</button>
      <footer>Version {version}</footer>
    </div>
  {/if}
</div>
