import { writable } from 'svelte/store';

export const machineStatus = writable({
  mode: 'auto',
  activeCycle: null,
  sectors: [],
});

export const weatherData = writable({
  temperature: [],
  humidity: [],
  rain: [],
  wind: [],
});