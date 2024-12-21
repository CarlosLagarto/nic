import { createStore } from 'vuex';

// Utility to manage WebSocket connection
const weatherSocketURL = 'ws://localhost/ws/weather';

export default createStore({
  state: {
    // Machine status
    machineStatus: {
      mode: 'auto',
      activeCycle: null,
      sectors: [],
    },
    // Weather data from the server
    weather: {
      temperature: [],
      humidity: [],
      rain: [],
      wind: [],
    },
    // Locally stored last 24 hours of weather data
    localWeatherData: JSON.parse(localStorage.getItem('weatherData')) || [],
  },

  mutations: {
    // Update machine status state
    UPDATE_MACHINE_STATUS(state, status) {
      state.machineStatus = status;
    },

    // Update weather data and persist to localStorage
    UPDATE_WEATHER(state, data) {
      state.weather = data;

      // Add to local weather history (keep 24-hour data)
      state.localWeatherData.push(data);
      if (state.localWeatherData.length > 24) state.localWeatherData.shift();

      // Persist local data to localStorage
      localStorage.setItem('weatherData', JSON.stringify(state.localWeatherData));
    },
  },

  actions: {
    // Simulated HTTP request to fetch machine status
    async fetchMachineStatus({ commit }) {
      try {
        // Replace this with an actual HTTP request using Axios or Fetch
        const response = await fetch('/api/machine/status'); // Mock API endpoint
        const data = await response.json();

        // Commit new status to the state
        commit('UPDATE_MACHINE_STATUS', data);
      } catch (error) {
        console.error('Failed to fetch machine status:', error);
      }
    },

    // Initialize WebSocket for weather updates
    initializeWebSocket({ commit }) {
      const socket = new WebSocket(weatherSocketURL);

      socket.onopen = () => {
        console.log('WebSocket connected for weather updates.');
      };

      socket.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          commit('UPDATE_WEATHER', data); // Update weather data
        } catch (error) {
          console.error('Error parsing WebSocket message:', error);
        }
      };

      socket.onerror = (error) => {
        console.error('WebSocket error:', error);
      };

      socket.onclose = () => {
        console.warn('WebSocket disconnected.');
        // Reconnect logic could go here if needed
      };
    },
  },

  getters: {
    // Get current machine status
    getMachineStatus: (state) => state.machineStatus,

    // Get real-time weather data
    getWeatherData: (state) => state.weather,

    // Get locally stored weather data (last 24 hours)
    getLocalWeather: (state) => state.localWeatherData,
  },
});