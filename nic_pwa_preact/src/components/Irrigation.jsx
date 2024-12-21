import { h } from 'preact';

const Irrigation = ({ machineStatus }) => {
  const { mode, activeCycle, sectors } = machineStatus;

  return (
    <div>
      <h1>Smart Irrigation</h1>
      <h2>Mode: {mode}</h2>
      <h3>Active Cycle: {activeCycle}</h3>
      <ul>
        {sectors.map((sector) => (
          <li key={sector.id}>
            Sector {sector.id} - {sector.progress}% Watered
          </li>
        ))}
      </ul>
    </div>
  );
};

export default Irrigation;