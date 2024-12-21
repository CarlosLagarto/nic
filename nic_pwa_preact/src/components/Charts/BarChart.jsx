import { h } from 'preact';
import { useEffect, useRef } from 'preact/hooks';
import Chart from 'chart.js/auto';

const BarChart = ({ data, title }) => {
  const chartRef = useRef();

  useEffect(() => {
    const ctx = chartRef.current.getContext('2d');
    new Chart(ctx, {
      type: 'bar',
      data: {
        labels: data.map((_, index) => `Hour ${index + 1}`),
        datasets: [
          {
            label: title,
            data: data,
            backgroundColor: 'rgba(153, 102, 255, 0.5)',
            borderColor: 'rgba(153, 102, 255, 1)',
            borderWidth: 1,
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
        scales: {
          y: {
            beginAtZero: true,
          },
        },
      },
    });
  }, [data, title]);

  return <canvas ref={chartRef} />;
};

export default BarChart;