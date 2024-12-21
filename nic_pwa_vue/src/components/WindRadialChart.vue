<template>
    <canvas ref="windCanvas"></canvas>
  </template>
  
  <script>
  import { Chart } from 'chart.js/auto';
  
  export default {
    props: ['windData'],
    mounted() {
      const colors = this.windData.map((_, i) =>
        `hsl(${120 - i * 5}, 80%, 50%)` // Color changes from green to brown
      );
  
      new Chart(this.$refs.windCanvas, {
        type: 'scatter',
        data: {
          datasets: [
            {
              label: 'Wind Direction and Speed',
              data: this.windData.map((d) => ({ x: d.degree, y: d.speed })),
              backgroundColor: colors,
            },
          ],
        },
        options: {
          scales: {
            x: { type: 'linear', position: 'bottom', title: { display: true, text: 'Direction (°)' } },
            y: { title: { display: true, text: 'Speed (m/s)' } },
          },
        },
      });
    },
  };
  </script>