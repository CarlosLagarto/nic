export function connectWebSocket(url, onMessage) {
  const socket = new WebSocket(url);
  socket.onmessage = (event) => {
    onMessage(JSON.parse(event.data));
  };
  return socket;
}