<script lang="ts">
  // The Suisha mark: a water wheel. `spinning` turns it while work runs.
  let { size = 28, spinning = false }: { size?: number; spinning?: boolean } = $props();
  const paddles = Array.from({ length: 16 }, (_, i) => i * 22.5);
  const spokes = Array.from({ length: 8 }, (_, i) => i * 45 + 22.5);
</script>

<svg class:spinning width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
  <g fill="currentColor">
    {#each paddles as angle (angle)}
      <rect x="30" y="8.5" width="4" height="10.5" rx="0.6" transform="rotate({angle} 32 32)" />
    {/each}
  </g>
  <g fill="none" stroke="currentColor" stroke-linecap="round">
    <circle cx="32" cy="32" r="18" stroke-width="2.6" />
    <circle cx="32" cy="32" r="14" stroke-width="2.2" />
    {#each spokes as angle (angle)}
      <line x1="32" y1="32" x2="32" y2="18" stroke-width="2" transform="rotate({angle} 32 32)" />
    {/each}
  </g>
  <circle cx="32" cy="32" r="4" fill="currentColor" />
</svg>

<style>
  svg {
    display: block;
    color: var(--accent);
  }

  .spinning {
    animation: turn 6s linear infinite;
  }

  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinning {
      animation: none;
    }
  }
</style>
