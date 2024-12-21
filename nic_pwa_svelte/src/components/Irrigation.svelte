<script>
    import { machineStatus } from '../store';
</script>

<h1>Smart Irrigation</h1>

{#if $machineStatus}
<h2>Mode: {$machineStatus.mode}</h2>
<h3>Active Cycle: {$machineStatus.activeCycle}</h3>
<ul>
    {#each $machineStatus.sectors as sector}
    <li>Sector {sector.id} - {sector.progress}% Watered</li>
    {/each}
</ul>
{:else}
<p>Loading...</p>
{/if}