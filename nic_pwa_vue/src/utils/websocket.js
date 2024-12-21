import { io } from 'socket.io-client';

const socket = io('ws://localhost/ws/weather');

export const websocket = {
  connect(callback) {
    socket.on('message', (data) => {
      const weatherData = JSON.parse(data);
      callback(weatherData);
    });
  },
  disconnect() {
    socket.disconnect();
  },
};