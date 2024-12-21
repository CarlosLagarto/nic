import { Bar } from 'react-chartjs-2';
import {
    Chart as ChartJS,
    BarElement,
    CategoryScale,
    LinearScale,
    Tooltip,
    Legend,
} from 'chart.js';

ChartJS.register(BarElement, CategoryScale, LinearScale, Tooltip, Legend);

const BarChart = ({ data = [], title = '', unit = '', customLabels = [] }) => {
    const labels = customLabels.length > 0 ? customLabels : data.map((_, index) => `T-${24 - index}h`);

    const chartData = {
        labels, 
        title,
        datasets: [
            {
                label: title,
                data,
                backgroundColor: 'rgba(75, 192, 192, 0.5)',
                borderColor: 'rgba(75, 192, 192, 1)',
                borderWidth: 1,
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
                ticks: { maxTicksLimit: 7 }, // Limit X-axis tick labels
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

    return <Bar data={chartData} options={options} />;
};

export default BarChart;