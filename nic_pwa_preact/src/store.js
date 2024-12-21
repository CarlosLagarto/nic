import { createContext } from 'preact';
import { useReducer } from 'preact/hooks';

const initialState = {
  machineStatus: { mode: 'auto', activeCycle: null, sectors: [] },
  weather: { temperature: [], humidity: [], rain: [], wind: [] },
};

function reducer(state, action) {
  switch (action.type) {
    case 'UPDATE_MACHINE_STATUS':
      return { ...state, machineStatus: action.payload };
    case 'UPDATE_WEATHER':
      return { ...state, weather: action.payload };
    default:
      return state;
  }
}

export const StoreContext = createContext();

export const StoreProvider = ({ children }) => {
  const [state, dispatch] = useReducer(reducer, initialState);

  return (
    <StoreContext.Provider value={{ state, dispatch }}>
      {children}
    </StoreContext.Provider>
  );
};