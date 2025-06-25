import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      // everything that is *not* an actual static file is forwarded
      // to the tonic server on 50051
      "^/putty_interface.*": {
        target: "http://127.0.0.1:50051",
        changeOrigin: true,
      },
    },
  },
});
