<template>
  <div>
    <h1>Weather Information</h1>
    <WeatherInfo />
  </div>
</template>

<script>
import WeatherInfo from '@/components/WeatherInfo.vue';

export default {
  name: 'WeatherView',
  components: {
    WeatherInfo,
  },
};
</script>