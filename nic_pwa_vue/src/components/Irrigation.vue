<template>
    <div>
      <h1>Machine Status</h1>
      <p>Mode: {{ machineStatus.mode }}</p>
      <p>Active Cycle: {{ machineStatus.activeCycle }}</p>
  
      <h2>Sector Progress</h2>
      <div v-for="sector in machineStatus.sectors" :key="sector.id">
        <p>Sector {{ sector.id }}: {{ sector.progress }}%</p>
        <progress :value="sector.progress" max="100"></progress>
      </div>
    </div>
  </template>
  
  <script>
  import { mapGetters, mapActions } from 'vuex';
  
  export default {
    computed: {
      ...mapGetters(['getMachineStatus']),
      machineStatus() {
        return this.getMachineStatus;
      },
    },
    created() {
      this.fetchMachineStatus();
    },
    methods: {
      ...mapActions(['fetchMachineStatus']),
    },
  };
  </script>