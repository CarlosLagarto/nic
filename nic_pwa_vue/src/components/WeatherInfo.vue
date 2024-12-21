<template>
  <div>
    <h1>Weather Information</h1>
    <canvas id="tempChart"></canvas>
    <canvas id="humidityChart"></canvas>
    <canvas id="rainChart"></canvas>
    <wind-radial-chart :wind-data="windData"></wind-radial-chart>
  </div>
</template>

<script>
import { mapGetters } from 'vuex';
import { Chart } from 'chart.js/auto';
import WindRadialChart from './WindRadialChart.vue';

export default {
  components: { WindRadialChart },
  computed: {
    ...mapGetters(['getWeatherData', 'getLocalWeather']),
    weatherData() {
      return this.getWeatherData;
    },
    windData() {
      return this.getLocalWeather.map((data) => ({
        degree: data.wind.degrees,
        speed: data.wind.speed,
      }));
    },
  },
  mounted() {
    this.renderCharts();
  },
  methods: {
    renderCharts() {
      new Chart(document.getElementById('tempChart'), {
        type: 'line',
        data: {
          labels: this.getLocalWeather.map((_, i) => i),
          datasets: [
            { label: 'Temperature', data: this.getLocalWeather.map((d) => d.temperature) },
          ],
        },
      });

      new Chart(document.getElementById('humidityChart'), {
        type: 'line',
        data: {
          labels: this.getLocalWeather.map((_, i) => i),
          datasets: [
            { label: 'Humidity', data: this.getLocalWeather.map((d) => d.humidity) },
          ],
        },
      });

      new Chart(document.getElementById('rainChart'), {
        type: 'bar',
        data: {
          labels: this.getLocalWeather.map((_, i) => i),
          datasets: [
            { label: 'Rainfall', data: this.getLocalWeather.map((d) => d.rain) },
          ],
        },
      });
    },
  },
};
</script>