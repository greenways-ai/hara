import { defineConfig } from "@playwright/test";
const executablePath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH;
const port = Number(process.env.PLAYWRIGHT_PORT ?? 4173);
export default defineConfig({testMatch:["hta-browser.spec.js","noir-browser.spec.js","studio-browser.spec.js","www-browser.spec.js","hara-amp-browser.spec.js"],use:{baseURL:`http://127.0.0.1:${port}`,launchOptions:executablePath?{executablePath}:{}},webServer:{command:`python3 -m http.server ${port} --bind 127.0.0.1 --directory ../..`,port,reuseExistingServer:true}});
