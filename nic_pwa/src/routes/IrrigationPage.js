import React from 'react';
import Irrigation from '../components/Irrigation';

const IrrigationPage = () => {
  const machineStatus = {
    mode: 'auto',
    activeCycle: 'Cycle A',
    sectors: [
      { id: 1, progress: 30 },
      { id: 2, progress: 70 },
    ],
  };

  return (
    <div>
      {/* <h1>Irrigation</h1> */}
      <Irrigation machineStatus={machineStatus} />
    </div>
  );
};

export default IrrigationPage;