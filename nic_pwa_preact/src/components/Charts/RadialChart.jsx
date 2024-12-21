import { h } from 'preact';
import { useEffect, useRef } from 'preact/hooks';
import Chart from 'chart.js/auto';

const RadialChart = ({ data, title }) => {
  const chartRef = useRef();

  useEffect(() => {
    const ctx = chartRef.current.getContext('2d');
    new Chart(ctx, {
      type: 'polarArea',
      data: {
        labels: data.map((point) => `${point.direction}°`),
        datasets: [
          {
            label: title,
            data: data.map((point) => point.intensity),
            backgroundColor: data.map((_, index) =>
              `rgba(${(index * 50) % 255}, ${(index * 30) % 255}, ${(index * 20) % 255}, 0.5)`
            ),
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
      },
    });
  }, [data, title]);

  return <canvas ref={chartRef} />;
};

export default RadialChart;