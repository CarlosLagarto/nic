<script>
import { onMount } from 'svelte';
import { Chart } from 'chart.js/auto';

export let data = [];
export let title = '';

let canvas;

onMount(() => {
    new Chart(canvas, {
    type: 'bar',
    data: {
        labels: data.map((_, i) => `Point ${i + 1}`),
        datasets: [{ data, label: title, backgroundColor: 'green' }],
    },
    });
});
</script>

<canvas bind:this={canvas}></canvas>