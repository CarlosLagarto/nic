import { Scatter } from 'react-chartjs-2';
import {
    Chart as ChartJS,
    LinearScale,
    PointElement,
    Tooltip,
} from 'chart.js';

ChartJS.register(LinearScale, PointElement, Tooltip);

const polarToCartesian = (direction, intensity) => {
    const angleInRadians = (direction * Math.PI) / 180;
    const x = intensity * Math.cos(angleInRadians);
    const y = intensity * Math.sin(angleInRadians);
    return { x, y };
};

const RadialChart = ({ data }) => {
    const maxIntensity = 70;
    const chartSize = 160; // Chart container size in pixels
    const scaleValues = [10, 20, 30, 50, 70]; // Scale values for the circles
    const scatterData = {
        datasets: [
            {
                label: 'Wind',
                data: data.map((item, index) => {
                    const { x, y } = polarToCartesian(item.direction, item.intensity);
                    return { x, y };
                }),
                backgroundColor: data.map((_, index) => {
                    const ratio = index / data.length;
                    const red = Math.floor(165 + (90 * ratio)); // From green to brown
                    const green = Math.floor(255 - (165 * ratio));
                    return `rgb(${red}, ${green}, 0)`;
                }),
                borderColor: 'white',
                pointRadius: 4,
                pointBorderWidth: 1,
            },
        ],
    };

    const options = {
        responsive: true,
        maintainAspectRatio: false,
        animation: false, // Disable animation
        plugins: {
            tooltip: {
                callbacks: {
                    label: (context) => {
                        const { x, y } = context.raw;
                        const direction = Math.atan2(y, x) * (180 / Math.PI);
                        const intensity = Math.sqrt(x ** 2 + y ** 2);
                        return `Direction: ${Math.round((direction + 360) % 360)}°, Speed: ${Math.round(intensity)} km/h`;
                    },
                },
            },
            legend: { display: false }, // Remove legend
        },
        scales: {
            x: {
                ticks: { display: false }, // Hide x-axis ticks
                grid: { display: false }, // Remove x-axis gridlines
                title: { display: false }, // Remove title
                border: { display: false }, // Remove axis line
                min: -maxIntensity,
                max: maxIntensity,
            },
            y: {
                ticks: { display: false }, // Hide y-axis ticks
                grid: { display: false }, // Remove y-axis gridlines
                title: { display: false }, // Remove title
                border: { display: false }, // Remove axis line
                min: -maxIntensity,
                max: maxIntensity,
            },
        },
    };
    return (
        <div style={{ position: 'relative', height: '160px', width: '160px' }}>
            <Scatter data={scatterData} options={options} />
            {scaleValues.map((radius, index) => {
                const size = (radius / maxIntensity) * chartSize;
                return (<div
                    key={index}
                    style={{
                        position: 'absolute',
                        border: '1px solid #ddd',
                        borderRadius: '50%',
                        width: `${size}px`,
                        height: `${size}px`,
                        top: `calc(50% - ${size / 2}px)`,
                        left: `calc(50% - ${size / 2}px)`,
                        zIndex: 1, // Place circles behind the chart
                        pointerEvents: 'none', // Prevent interaction with circles
                    }}
                />
                );
            })}
            {/* Add vertical scale with labels */}
            {Array.from({ length: 4 }, (_, i) => (
                <div
                    key={i}
                    style={{
                        position: 'absolute',
                        width: '2px',
                        height: '160px',
                        backgroundColor: '#ddd',
                        transformOrigin: 'center',
                        transform: `rotate(${i * 90}deg)`,
                        top: 0,
                        left: '50%',
                        zIndex: 1, // Place radial lines on top
                        pointerEvents: 'none', // Prevent interaction with lines
                    }}
                />
            ))}
            {/* Add scale numbers */}
            {scaleValues.map((radius, index) => {
                const size = (radius / maxIntensity) * chartSize;
                const offset = size / 2; // Offset to position numbers near the bottom of the circles
                return (
                    <div
                        key={index}
                        style={{
                            position: 'absolute',
                            left: 'calc(50% - 10px)', // Center the numbers horizontally near the vertical line
                            top: `calc(50% + ${offset}px)`, // Align with the bottom part of the circle
                            fontSize: '10px',
                            color: 'darkgray',
                            zIndex: 2, // Ensure the numbers are visible
                            pointerEvents: 'none', // Prevent interaction
                        }}
                    >
                        {radius}
                    </div>
                );
            })}
        </div>
    );
};

export default RadialChart;