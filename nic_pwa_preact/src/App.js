import { h } from 'preact';
import { Router } from 'preact-router';
import Irrigation from './routes/Irrigation';
import Weather from './routes/Weather';

const App = () => (
  <div id="app">
    <Router>
      <Irrigation path="/" />
      <Weather path="/weather" />
    </Router>
  </div>
);

export default App;