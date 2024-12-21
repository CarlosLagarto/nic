import { Line } from 'react-chartjs-2';
import {
    Chart as ChartJS,
    LineElement,
    CategoryScale,
    LinearScale,
    PointElement,
    Filler,
    Tooltip,
    Legend,
} from 'chart.js';

ChartJS.register(LineElement, CategoryScale, LinearScale, PointElement, Filler, Tooltip, Legend);

const LineChart = ({ data = [], title = '', unit = '' }) => {
    const chartData = {
        labels: data.map((_, index) => `T-${24 - index}h`),
        datasets: [
            {
                label: title,
                data,
                backgroundColor: 'rgba(75,192,192,0.4)',
                borderColor: 'rgba(75,192,192,1)',
                fill: true,
                tension: 0.4,
            },
        ],
    };

    const options = {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
            legend: { display: false }, 
        },
        scales: {
            x: {
                ticks: { maxTicksLimit: 5 }, // Limit X-axis tick labels
                grid: { display: false }, // Remove gridlines for a cleaner look
            },
            y: {
                ticks: {
                    callback: (value) => `${value} ${unit}`, // Add units to labels
                },
                grid: { color: '#ddd' }, // Customize gridline color
            },
        },
    };

    return <Line data={chartData} options={options} />;
};

export default LineChart;