import { defineConfig } from "@playwright/test";
export default defineConfig({testMatch:["hta-browser.spec.js","noir-browser.spec.js","studio-browser.spec.js"],use:{baseURL:"http://127.0.0.1:4173"},webServer:{command:"python3 -m http.server 4173 --bind 127.0.0.1 --directory ../..",port:4173,reuseExistingServer:true}});
