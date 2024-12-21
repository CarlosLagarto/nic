import React, { useState, useEffect } from 'react';
import { BrowserRouter as Router, Route, Routes, Link } from 'react-router-dom';
import IrrigationPage from './routes/IrrigationPage';
import WeatherPage from './routes/WeatherPage';

const App = () => {
  const [weatherData, setWeatherData] = useState({
    temperature: [],
    humidity: [],
    rain: [],
    wind: [],
    evapotranspiration: null,
  });

  useEffect(() => {
    // Timer to simulate weather data updates
    const interval = setInterval(() => {
      setWeatherData((prevData) => {
        const newTemperature = Math.random() * 40; // Random temperature between 0 and 40°C
        const newHumidity = Math.random() * 100; // Random humidity between 0 and 100%
        const newRain = Math.random() * 10; // Random rainfall between 0 and 10 mm
        const newWind = Math.random() * 20; // Random wind speed between 0 and 20 km/h
        const newEvapotranspiration = (Math.random() * 5).toFixed(2); // Random eT between 0 and 5

        return {
          temperature: [...prevData.temperature.slice(-23), newTemperature],
          humidity: [...prevData.humidity.slice(-23), newHumidity],
          rain: [...prevData.rain.slice(-23), newRain],
          wind: [...prevData.wind.slice(-23), newWind],
          evapotranspiration: newEvapotranspiration,
        };
      });
    }, 1000); // Update every 5 seconds

    return () => clearInterval(interval); // Cleanup on component unmount
  }, []);
  return (
    <Router>
      <nav>
        <ul>
          <li>
            <Link to="/">Irrigation</Link>
          </li>
          <li>
            <Link to="/weather">Weather</Link>
          </li>
        </ul>
      </nav>
      <Routes>
        <Route path="/" element={<IrrigationPage />} />
        <Route path="/weather" element={<WeatherPage weatherData={weatherData} />} />
      </Routes>
    </Router>
  );
};

export default App;