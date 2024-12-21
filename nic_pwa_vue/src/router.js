import { createRouter, createWebHistory } from 'vue-router';

// Import the views
import IrrigationView from '@/views/IrrigationView.vue';
import WeatherView from '@/views/WeatherView.vue';

const routes = [
  {
    path: '/',
    name: 'Irrigation',
    component: IrrigationView, // Main irrigation view
  },
  {
    path: '/weather',
    name: 'Weather',
    component: WeatherView, // Weather page with real-time data
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;