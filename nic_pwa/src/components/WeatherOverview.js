import React from 'react';

const formatValue = (value, unit, decimals = 0) => {
    if (value == null || value === 'N/A') {
        return 'N/A';
    }
    return `${value.toFixed(decimals)} ${unit}`;
};

const WeatherOverview = ({ weather = {} }) => {
    const {
        temperature = [],
        humidity = [],
        rain = [],
        wind = [],
        evapotranspiration = [],
        pressure = [],
    } = weather;

    const latestWind = wind.at(-1) || { direction: 'N/A', intensity: 'N/A' };

    return (
        <div className="summary-section">
            <h3>Latest</h3>
            <ul className="weather-overview">
                <li>
                    <span className="label">Wind:</span>
                    <span className="value">{`${latestWind.intensity} km/h @ ${latestWind.direction}°`}</span>
                </li>
                <li>
                    <span className="label">Pressure:</span>
                    <span className="value">{pressure.at(-1) || 'N/A'} hPa</span>
                </li>
                <li>
                    <span className="label">Temperature:</span>
                    <span className="value">{temperature.at(-1) || 'N/A'}°C</span>
                </li>
                <li>
                    <span className="label">Rain:</span>
                    <span className="value">{rain.at(-1) || 'N/A'} mm</span>
                </li>
                <li>
                    <span className="label">Humidity:</span>
                    <span className="value">{humidity.at(-1) || 'N/A'}%</span>
                </li>
                <li>
                    <span className="label">Evapotranspiration:</span>
                    <span className="value">
                        {formatValue(evapotranspiration.at(-1), 'mm/day', 1)}
                    </span>
                </li>
            </ul>
        </div>
    );
};

export default WeatherOverview;