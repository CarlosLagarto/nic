import { defineStore } from 'pinia';
import axios from 'axios';
import { io } from 'socket.io-client';

export const useMachineStore = defineStore('machineStore', {
  state: () => ({
    mode: 'Auto',
    status: {},
    sectors: [],
    weather: {},
    historicalWeather: [],
    websocket: null,
  }),
  actions: {
    async fetchStatus() {
      const response = await axios.get('/api/status');
      this.status = response.data;
      this.mode = response.data.mode;
      this.sectors = response.data.sectors;
    },
    async changeMode(newMode) {
      await axios.post(`/api/mode/change`, { mode: newMode });
      this.mode = newMode;
    },
    connectWebSocket() {
      this.websocket = io('wss://your-server-vpn-address');
      this.websocket.on('weatherUpdate', (data) => {
        this.weather = data;
        this.saveLocalWeather(data);
      });
    },
    saveLocalWeather(data) {
      const now = Date.now();
      this.historicalWeather.push({ ...data, timestamp: now });
      localStorage.setItem('weatherHistory', JSON.stringify(this.historicalWeather.slice(-24)));
    },
    loadLocalWeather() {
      const data = localStorage.getItem('weatherHistory');
      if (data) this.historicalWeather = JSON.parse(data);
    },
  },
});