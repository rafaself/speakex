<script lang="ts">
  import { onMount, tick } from "svelte";
  import { fade } from "svelte/transition";

  export let label: string;
  export let value: string;
  export let options: Array<{ value: string; label: string; disabled?: boolean }> = [];
  export let onSelect: (value: string) => void;

  let isOpen = false;
  let rootElement: HTMLDivElement | null = null;
  let triggerElement: HTMLButtonElement | null = null;
  let optionElements: Array<HTMLButtonElement | null> = [];
  let highlightedIndex = -1;
  let selectedIndex = -1;
  let selectedOption: { value: string; label: string; disabled?: boolean } | null = null;

  $: selectedIndex = options.findIndex((option) => option.value === value);
  $: selectedOption = selectedIndex >= 0 ? options[selectedIndex] : null;
  $: listboxId = `${label.toLowerCase().replace(/\s+/g, "-")}-menu-listbox`;

  onMount(() => {
    const handlePointerDown = (event: PointerEvent) => {
      if (!isOpen || !(event.target instanceof Node)) {
        return;
      }

      if (!rootElement?.contains(event.target)) {
        closeMenu(false);
      }
    };

    document.addEventListener("pointerdown", handlePointerDown);

    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
    };
  });

  function getNextEnabledIndex(startIndex: number, direction: 1 | -1): number {
    if (options.length === 0) {
      return -1;
    }

    let index = startIndex;

    for (let step = 0; step < options.length; step += 1) {
      index = (index + direction + options.length) % options.length;

      if (!options[index]?.disabled) {
        return index;
      }
    }

    return -1;
  }

  function getFirstEnabledIndex(): number {
    return options.findIndex((option) => !option.disabled);
  }

  async function openMenu(preferredIndex = selectedIndex >= 0 ? selectedIndex : getFirstEnabledIndex()) {
    if (isOpen) {
      return;
    }

    highlightedIndex = preferredIndex >= 0 && !options[preferredIndex]?.disabled ? preferredIndex : getFirstEnabledIndex();
    isOpen = true;
    await tick();
    optionElements[highlightedIndex]?.focus();
  }

  function closeMenu(restoreFocus = true) {
    isOpen = false;
    highlightedIndex = -1;

    if (restoreFocus) {
      triggerElement?.focus();
    }
  }

  function toggleMenu() {
    if (isOpen) {
      closeMenu();
      return;
    }

    void openMenu();
  }

  function selectOption(nextValue: string) {
    onSelect(nextValue);
    closeMenu();
  }

  function handleTriggerKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      void openMenu(selectedIndex >= 0 ? selectedIndex : getFirstEnabledIndex());
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      void openMenu(selectedIndex >= 0 ? selectedIndex : getFirstEnabledIndex());
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      void openMenu();
    }
  }

  function handleOptionKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      const nextIndex = getNextEnabledIndex(index, 1);
      highlightedIndex = nextIndex;
      optionElements[nextIndex]?.focus();
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      const nextIndex = getNextEnabledIndex(index, -1);
      highlightedIndex = nextIndex;
      optionElements[nextIndex]?.focus();
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      const nextIndex = getFirstEnabledIndex();
      highlightedIndex = nextIndex;
      optionElements[nextIndex]?.focus();
      return;
    }

    if (event.key === "End") {
      event.preventDefault();

      for (let nextIndex = options.length - 1; nextIndex >= 0; nextIndex -= 1) {
        if (!options[nextIndex]?.disabled) {
          highlightedIndex = nextIndex;
          optionElements[nextIndex]?.focus();
          break;
        }
      }

      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      closeMenu();
      return;
    }

    if (event.key === "Tab") {
      closeMenu(false);
    }
  }
</script>

<div class="menu-list">
  <div class="menu-root" bind:this={rootElement}>
    <button
      bind:this={triggerElement}
      class="menu-trigger"
      type="button"
      aria-label={label}
      aria-haspopup="listbox"
      aria-expanded={isOpen}
      aria-controls={listboxId}
      on:click={toggleMenu}
      on:keydown={handleTriggerKeydown}
    >
      <span class="menu-trigger-label">{selectedOption?.label ?? label}</span>
      <svg class:is-open={isOpen} width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>

    {#if isOpen}
      <div
        id={listboxId}
        class="menu-panel"
        role="listbox"
        aria-label={label}
        transition:fade={{ duration: 140 }}
      >
        {#each options as option, index}
          <button
            bind:this={optionElements[index]}
            class:selected={value === option.value}
            class:highlighted={highlightedIndex === index}
            class="menu-option"
            type="button"
            role="option"
            aria-selected={value === option.value}
            disabled={option.disabled}
            on:click={() => selectOption(option.value)}
            on:focus={() => (highlightedIndex = index)}
            on:keydown={(event) => handleOptionKeydown(event, index)}
          >
            <span>{option.label}</span>
            {#if value === option.value}
              <svg class="menu-option-check" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                <path d="M20 6 9 17l-5-5" />
              </svg>
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .menu-list {
    width: 100%;
  }

  .menu-root {
    position: relative;
    width: 100%;
  }

  .menu-trigger {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    background: #2f2f2f;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #fff;
    border-radius: 12px;
    padding: 0.75rem 0.9rem;
    text-align: left;
  }

  .menu-trigger svg {
    color: #b4b4b4;
    flex-shrink: 0;
    transition: transform 0.2s ease;
  }

  .menu-trigger svg.is-open {
    transform: rotate(180deg);
  }

  .menu-trigger-label {
    min-width: 0;
  }

  .menu-panel {
    position: absolute;
    top: calc(100% + 0.5rem);
    left: 0;
    right: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.35rem;
    background: #2a2a2a;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.35);
  }

  .menu-option {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.7rem 0.75rem;
    border-radius: 8px;
    text-align: left;
    color: #ececf1;
  }

  .menu-option.highlighted,
  .menu-option:hover:not(:disabled),
  .menu-option:focus-visible {
    background: rgba(255, 255, 255, 0.08);
    outline: none;
  }

  .menu-option.selected {
    color: #fff;
  }

  .menu-option:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .menu-option-check {
    color: #72e17b;
    flex-shrink: 0;
  }
</style>
