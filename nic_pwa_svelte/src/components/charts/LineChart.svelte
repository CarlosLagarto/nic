<script>
import { onMount } from 'svelte';
import { Chart } from 'chart.js/auto';

export let data = [];
export let title = '';

let canvas;

onMount(() => {
    new Chart(canvas, {
    type: 'line',
    data: {
        labels: data.map((_, i) => `Point ${i + 1}`),
        datasets: [{ data, label: title, borderColor: 'blue', fill: false }],
    },
    });
});
</script>

<canvas bind:this={canvas}></canvas>