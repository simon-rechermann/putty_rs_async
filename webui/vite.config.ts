import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: { "/rpc": "http://127.0.0.1:50051" }   // dev proxy to tonic server
  }
});
