<template>
    <div>
      <h1>Weather Info</h1>
      <canvas id="weatherChart"></canvas>
    </div>
  </template>
  
  <script>
  import Chart from "chart.js/auto";
  
  export default {
    name: "WeatherView",
    data() {
      return {
        weatherData: [], // Populate from WebSocket or API
      };
    },
    methods: {
      createChart() {
        const ctx = document.getElementById("weatherChart").getContext("2d");
        new Chart(ctx, {
          type: "bar",
          data: {
            labels: this.weatherData.map((d) => d.time),
            datasets: [
              {
                label: "Rainfall",
                data: this.weatherData.map((d) => d.rain),
              },
              {
                label: "Evapotranspiration",
                data: this.weatherData.map((d) => d.et),
              },
            ],
          },
        });
      },
    },
    mounted() {
      this.createChart();
    },
  };
  </script>