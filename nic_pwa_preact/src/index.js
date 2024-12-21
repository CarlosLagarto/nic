import { render } from 'preact';
import { StoreProvider } from './store';
import App from './App';

function renderApp() {
  const root = document.getElementById('root');
  if (root) {
    render(
      <StoreProvider>
        <App />
      </StoreProvider>,
      root
    );
  } else {
    console.error('Root element not found!');
  }
}

// Ensure DOM is fully loaded before attempting to render
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', renderApp);
} else {
  renderApp();
}

export default App; // Export App for prerendering