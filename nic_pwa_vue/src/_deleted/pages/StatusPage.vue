<template>
    <div>
      <h1>Machine Status</h1>
      <p>Mode: {{ status.mode }}</p>
      <p>Active Sector: {{ status.active_sector }}</p>
      <p>Progress: {{ status.progress }}%</p>
      <button @click="changeMode">Switch to Manual</button>
    </div>
  </template>
  
  <script>
  import axios from "axios";
  
  export default {
    name: "StatusView",
    data() {
      return {
        status: {},
      };
    },
    methods: {
      async fetchStatus() {
        const response = await axios.get("/api/status");
        this.status = response.data;
      },
      async changeMode() {
        await axios.post("/api/change_mode");
        this.fetchStatus();
      },
    },
    mounted() {
      this.fetchStatus();
    },
  };
  </script>