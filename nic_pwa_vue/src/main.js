
import { createApp } from 'vue';
import App from './App.vue';
import router from './router';
import store from './store';

import './registerServiceWorker'; // For PWA
import './assets/main.css'; // Main styling

const app = createApp(App);
app.use(router);
app.use(store);
app.mount('#app');

store.dispatch('initializeWebSocket');
store.dispatch('fetchMachineStatus'); // Optionally fetch the initial machine status