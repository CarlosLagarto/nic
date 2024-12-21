<script>
import { weatherData } from '../store';
import LineChart from './charts/LineChart.svelte';
import BarChart from './charts/BarChart.svelte';
import RadialChart from './charts/RadialChart.svelte';
</script>

<h1>Weather Information</h1>

<LineChart data={$weatherData.temperature} title="Temperature" />
<LineChart data={$weatherData.humidity} title="Humidity" />
<BarChart data={$weatherData.rain} title="Rainfall" />
<RadialChart data={$weatherData.wind} title="Wind Direction" />