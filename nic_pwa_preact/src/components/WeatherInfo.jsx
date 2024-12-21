import { h } from 'preact';
import LineChart from './Charts/LineChart';
import BarChart from './Charts/BarChart';
import RadialChart from './Charts/RadialChart';

const WeatherInfo = ({ weather }) => {
  const { temperature, humidity, rain, wind } = weather;

  return (
    <div>
      <h1>Weather</h1>
      <LineChart data={temperature} title="Temperature" />
      <LineChart data={humidity} title="Humidity" />
      <BarChart data={rain} title="Rainfall" />
      <RadialChart data={wind} title="Wind Direction" />
    </div>
  );
};

export default WeatherInfo;