<script>
import { onMount } from 'svelte';
import { Chart } from 'chart.js/auto';

export let data = [];
export let title = '';

let canvas;

onMount(() => {
    new Chart(canvas, {
    type: 'radar',
    data: {
        labels: data.map((_, i) => `Direction ${i}`),
        datasets: [{ data, label: title, backgroundColor: 'rgba(255, 0, 0, 0.5)' }],
    },
    });
});
</script>

<canvas bind:this={canvas}></canvas>