<script lang="ts">
  import { tick } from "svelte";

  type Props = {
    version: string;
    checkingForUpdate: boolean;
    onCheckForUpdates: () => void;
    onOpenHelp: () => void;
  };

  let { version, checkingForUpdate, onCheckForUpdates, onOpenHelp }: Props = $props();

  const menuId = `application-menu-${crypto.randomUUID()}`;
  let open = $state(false);
  let container = $state<HTMLDivElement>();
  let hamburger = $state<HTMLButtonElement>();
  let firstAction = $state<HTMLButtonElement>();
  let helpAction = $state<HTMLButtonElement>();

  async function toggleMenu() {
    open = !open;
    if (open) {
      await tick();
      (checkingForUpdate ? helpAction : firstAction)?.focus();
    }
  }

  function handleWindowClick(event: MouseEvent) {
    if (open && !container?.contains(event.target as Node)) open = false;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (!open || event.key !== "Escape") return;
    open = false;
    hamburger?.focus();
  }

  async function runAction(callback: () => void) {
    open = false;
    await tick();
    callback();
  }

  $effect(() => {
    if (!open) return;
    window.addEventListener("click", handleWindowClick);
    window.addEventListener("keydown", handleWindowKeydown);
    return () => {
      window.removeEventListener("click", handleWindowClick);
      window.removeEventListener("keydown", handleWindowKeydown);
    };
  });
</script>

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
        onclick={() => runAction(onCheckForUpdates)}
      >
        {checkingForUpdate ? "Checking for updates…" : "Check for updates"}
      </button>
      <button
        bind:this={helpAction}
        type="button"
        role="menuitem"
        onclick={() => runAction(onOpenHelp)}>Help</button
      >
      <footer role="presentation">Version {version}</footer>
    </div>
  {/if}
</div>
