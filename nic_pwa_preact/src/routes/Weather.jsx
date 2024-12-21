import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import WeatherInfo from '../components/WeatherInfo';

const WeatherInfoRoute = () => {
  const [weather, setWeather] = useState({
    temperature: [],
    humidity: [],
    rain: [],
    wind: [],
  });

  useEffect(() => {
    const fetchWeather = async () => {
      const response = await fetch('/api/weather'); // Adjust API endpoint as necessary
      const data = await response.json();
      setWeather(data);
    };

    fetchWeather();

    // Simulate WebSocket updates
    const socket = new WebSocket('ws://localhost/ws/weather');
    socket.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setWeather((prevWeather) => ({
        ...prevWeather,
        ...data,
      }));
    };

    return () => socket.close();
  }, []);

  return <WeatherInfo weather={weather} />;
};

export default WeatherInfoRoute;