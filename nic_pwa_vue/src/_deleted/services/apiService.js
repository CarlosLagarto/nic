import axios from "axios";

const API_URL = "http://localhost:8080/api";

export const getStatus = async () => {
  const res = await axios.get(`${API_URL}/status`);
  return res.data;
};

export const getWeather = async () => {
  const res = await axios.get(`${API_URL}/weather`);
  return res.data;
};