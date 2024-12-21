self.addEventListener("install", (event) => {
    console.log("Service Worker installing...");
    event.waitUntil(
      caches.open("app-static-v1").then((cache) => {
        return cache.addAll([
          "/",
          "/index.html",
          "/manifest.json",
          "/img/icons/nic_icon.png",
          "/img/icons/nic_icon.png",
          "/css/main.css",
          "/js/app.js"
        ]);
      })
    );
  });
  
  self.addEventListener("fetch", (event) => {
    event.respondWith(
      caches.match(event.request).then((response) => {
        return response || fetch(event.request);
      })
    );
  });
  
  self.addEventListener("activate", (event) => {
    console.log("Service Worker activated!");
  });