import { h } from 'preact';
import { useEffect, useRef } from 'preact/hooks';
import Chart from 'chart.js/auto';

const LineChart = ({ data, title }) => {
  const chartRef = useRef();

  useEffect(() => {
    const ctx = chartRef.current.getContext('2d');
    new Chart(ctx, {
      type: 'line',
      data: {
        labels: data.map((_, index) => `Hour ${index + 1}`),
        datasets: [
          {
            label: title,
            data: data,
            borderColor: 'rgba(75, 192, 192, 1)',
            backgroundColor: 'rgba(75, 192, 192, 0.2)',
            borderWidth: 2,
            fill: true,
          },
        ],
      },
      options: {
        responsive: true,
        plugins: {
          legend: {
            display: true,
            position: 'top',
          },
        },
      },
    });
  }, [data, title]);

  return <canvas ref={chartRef} />;
};

export default LineChart;