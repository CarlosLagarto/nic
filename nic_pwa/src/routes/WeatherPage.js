import { useEffect, useState } from 'react';
import React from 'react';
import WeatherOverview from '../components/WeatherOverview';
import LineChart from '../components/charts/LineChart';
import BarChart from '../components/charts/BarChart';
import RadialChart from '../components/charts/RadialChart';

// const generateInitialData = (size) => Array(size).fill(null); // Helper function for null-filled data

const generateInitialData = (size, isObject = false) =>
    isObject ? Array(size).fill({ direction: null, intensity: null }) : Array(size).fill(null);


const generateRainLabels = () => {
    return Array.from({ length: 24 }, (_, index) => `T-${24 - index}h`);
};

const generateETLabels = () => {
    const labels = [];
    const now = new Date();
    const monthNames = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

    for (let i = 19; i >= 0; i--) {
        const date = new Date();
        date.setDate(now.getDate() - i);
        const day = date.getDate();
        const month = monthNames[date.getMonth()]; // Get month name

        // Include month change or the most recent data point
        if ((i === 19) || (i < 19 && date.getMonth() !== new Date(now.getFullYear(), now.getMonth(), now.getDate() - i + 1).getMonth())) {
            labels.push(`${day}/${month}`);
        } else {
            labels.push(`${day}`);
        }
    }
    return labels;
};

const WeatherPage = () => {
    const initialWeatherData = {
        temperature: generateInitialData(24),
        humidity: generateInitialData(24),
        rain: generateInitialData(24),
        wind: generateInitialData(4, true),
        evapotranspiration: generateInitialData(20),
        pressure: generateInitialData(24),
    };

    const [weatherData, setWeatherData] = useState(initialWeatherData);

    useEffect(() => {
        // Adjust the .charts-grid max height dynamically
        const adjustChartsGridHeight = () => {
            const nav = document.querySelector('nav');
            const summaryContainer = document.querySelector('.summary-container');
            const chartsGrid = document.querySelector('.charts-grid');

            if (nav && summaryContainer && chartsGrid) {
                const navHeight = nav.offsetHeight || 0;
                const summaryHeight = summaryContainer.offsetHeight || 0;

                // Adjust the max-height dynamically with extra margin consideration
                const extraMargin = 20; // Account for additional spacing/padding
                // chartsGrid.style.maxHeight = `calc(100vh - ${navHeight + summaryHeight + extraMargin}px)`;
                setTimeout(() => {
                    chartsGrid.style.maxHeight = `calc(100vh - ${navHeight + summaryHeight + extraMargin}px)`;
                }, 50); // Slight delay to ensure layout stabilizes
            }
        };

        window.addEventListener('resize', adjustChartsGridHeight);
        adjustChartsGridHeight();

        return () => window.removeEventListener('resize', adjustChartsGridHeight);
    }, []);

    useEffect(() => {
        const interval = setInterval(() => {
            // console.log('Simulating weather data update');
            const newTemperature = Math.floor(Math.random() * 35);
            const newHumidity = Math.floor(Math.random() * 100);
            const newRainfall = Math.floor(Math.random() * 10);
            const newET = Math.random() * 10;
            const newWind = {
                direction: Math.floor(Math.random() * 360),
                intensity: Math.floor(Math.random() * 70) + 10,
            };
            const newPressure = Math.floor(Math.random() * 20) + 1000;

            setWeatherData((prev) => {
                const updatedData = {
                    temperature: [...prev.temperature.slice(-23), newTemperature],
                    humidity: [...prev.humidity.slice(-23), newHumidity],
                    rain: [...prev.rain.slice(-23), newRainfall],
                    wind: [...prev.wind.slice(-3), newWind],
                    evapotranspiration: [...prev.evapotranspiration.slice(-19), newET],
                    pressure: [...prev.pressure.slice(-23), newPressure],
                };
                // console.log('Updated Weather Data:', updatedData); // Log updated data
                return updatedData;
            });
        }, 1000);

        return () => clearInterval(interval);
    }, []);



    return (
        <div style={{ padding: '10px', marginBottom: '60px' }}> {/* Add margin to account for navbar */}
            {/* Summary and Wind Chart Section */}
            <div className="summary-container">
                <div className="summary-section">
                    <WeatherOverview weather={weatherData} />
                </div>
                <div className="wind-chart-container">
                    <RadialChart data={weatherData.wind} />
                </div>
            </div>

            {/* Section for Charts */}
            {/* <div className="charts-grid"> */}
            <div className="charts-grid hidden-scroll">
                <h4 className="chart-title">Atmospheric Pressure (Last 24 Hours)</h4>
                <div>
                    <LineChart data={weatherData.pressure} title="Pressure (hPa)" unit="hPa" />
                </div>
                <h4 className="chart-title">Rainfall (Last 24 Hours)</h4>
                <div>
                    <BarChart data={weatherData.rain} title="Rainfall (mm)" unit='mm' labels={generateRainLabels()} />
                </div>
                <h4 className="chart-title">Temperature (Last 24 Hours)</h4>
                <div >
                    <LineChart data={weatherData.temperature} title="Temperature (°C)" unit='°C' />
                </div>
                <h4 className="chart-title">Humidity (Last 24 Hours)</h4>
                <div >
                    <LineChart data={weatherData.humidity} title="Humidity (%)" unit='%' />
                </div>
                <h4 className="chart-title">Evapotranspiration (Last 20 Days)</h4>
                <div >
                    <BarChart
                        data={weatherData.evapotranspiration}
                        title="ET (mm/day)"
                        unit="mm"
                        customLabels={generateETLabels()} // Pass custom labels for ET
                    />
                </div>
            </div>
        </div>
    );
};

export default WeatherPage;