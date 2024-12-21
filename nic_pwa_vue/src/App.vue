<template>
  <div id="app">
    <nav>
      <router-link to="/">Irrigation</router-link> |
      <router-link to="/weather">Weather Info</router-link>
    </nav>
    <router-view />
  </div>
</template>

<style nonce="fixed-nonce-lagarto">
nav {
  padding: 10px;
  background-color: #42b983;
  color: white;
}

nav a {
  margin-right: 15px;
  color: white;
  text-decoration: none;
}

nav a.router-link-exact-active {
  font-weight: bold;
}
</style>