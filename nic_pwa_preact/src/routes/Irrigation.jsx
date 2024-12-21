import { h } from 'preact';
import { useState, useEffect } from 'preact/hooks';
import Irrigation from '../components/Irrigation';

const IrrigationRoute = () => {
  const [machineStatus, setMachineStatus] = useState({
    mode: 'Auto',
    activeCycle: 'Cycle A',
    sectors: [
      { id: 1, progress: 30 },
      { id: 2, progress: 70 },
    ],
  });

  useEffect(() => {
    // Simulate fetching machine status data from an API or WebSocket
    const fetchData = async () => {
      const data = await fetch('/api/machine-status').then((res) => res.json());
      setMachineStatus(data);
    };
    fetchData();
  }, []);

  return <Irrigation machineStatus={machineStatus} />;
};

export default IrrigationRoute;