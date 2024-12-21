<template>
    <div class="container mx-auto p-6 bg-white shadow-lg rounded-lg">
      <h1 class="text-2xl font-bold mb-6">Weather Historical Data</h1>
  
      <!-- Line Chart: Temperature -->
      <div class="mb-6">
        <h2 class="text-lg font-semibold mb-2">Temperature (°C)</h2>
        <LineChart :chart-data="temperatureData" :options="chartOptions" />
      </div>
  
      <!-- Line Chart: Humidity -->
      <div class="mb-6">
        <h2 class="text-lg font-semibold mb-2">Humidity (%)</h2>
        <LineChart :chart-data="humidityData" :options="chartOptions" />
      </div>
  
      <!-- Bar Chart: Rainfall and ET -->
      <div class="mb-6">
        <h2 class="text-lg font-semibold mb-2">Rainfall & Evapotranspiration</h2>
        <BarChart :chart-data="rainEtData" :options="chartOptions" />
      </div>
  
      <!-- Radial Chart: Wind -->
      <div>
        <h2 class="text-lg font-semibold mb-2">Wind Direction and Intensity</h2>
        <canvas id="windChart"></canvas>
      </div>
    </div>
  </template>
  
  <script>
  import {
    Chart as ChartJS,
    LineElement,
    BarElement,
    CategoryScale,
    LinearScale,
    Title,
    Tooltip,
    Legend,
    PointElement,
  } from "chart.js";

  
  ChartJS.register(
    LineElement,
    BarElement,
    CategoryScale,
    LinearScale,
    PointElement,
    Title,
    Tooltip,
    Legend
  );
  
  export default {
    components: {
      LineChart: Line,
      BarChart: Bar,
    },
    data() {
      return {
        temperatureData: null,
        humidityData: null,
        rainEtData: null,
        windData: [
                { direction: 10, intensity: 5 },
                { direction: 45, intensity: 8 },
                { direction: 90, intensity: 3 },
                { direction: 180, intensity: 12 },
                { direction: 270, intensity: 7 },
                { direction: 330, intensity: 6 },
            ],
        chartOptions: {
          responsive: true,
          maintainAspectRatio: false,
        },

      };
    },
    methods: {
      async fetchWeatherData() {
        const response = await fetch("/weather/history");
        const data = await response.json();
  
        // Prepare Temperature and Humidity Line Charts
        const labels = data.map((entry) => entry.time);
        this.temperatureData = {
          labels,
          datasets: [
            {
              label: "Temperature (°C)",
              data: data.map((entry) => entry.temperature),
              borderColor: "rgb(255, 99, 132)",
              fill: false,
            },
          ],
        };
  
        this.humidityData = {
          labels,
          datasets: [
            {
              label: "Humidity (%)",
              data: data.map((entry) => entry.humidity),
              borderColor: "rgb(54, 162, 235)",
              fill: false,
            },
          ],
        };
  
        // Prepare Rain and ET Bar Chart
        this.rainEtData = {
          labels,
          datasets: [
            {
              label: "Rainfall (mm)",
              data: data.map((entry) => entry.rain),
              backgroundColor: "rgb(75, 192, 192)",
            },
            {
              label: "Evapotranspiration (cm)",
              data: data.map((entry) => entry.et),
              backgroundColor: "rgb(153, 102, 255)",
            },
          ],
        };
  
        // Prepare Wind Polar Area Chart
        this.windData = {
          labels: data.map((entry) => entry.wind_direction),
          datasets: [
            {
              label: "Wind Intensity (km/h)",
              data: data.map((entry) => entry.wind_speed),
              backgroundColor: [
                "rgba(255, 99, 132, 0.6)",
                "rgba(54, 162, 235, 0.6)",
                "rgba(255, 206, 86, 0.6)",
                "rgba(75, 192, 192, 0.6)",
                "rgba(153, 102, 255, 0.6)",
                "rgba(255, 159, 64, 0.6)",
              ],
            },
          ],
        };
      },
    },
    mounted() {
      this.fetchWeatherData();
      this.createWindChart();
    },
  };
  </script>
  
  <style nonce="fixed-nonce-lagarto" scoped>
  .container {
    max-width: 800px;
    margin: auto;
    height: 100%;
  }
  </style>