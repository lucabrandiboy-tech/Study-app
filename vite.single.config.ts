import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { viteSingleFile } from 'vite-plugin-singlefile';

// Builds the whole app into one self-contained HTML file (dist-single/index.html).
export default defineConfig({
  plugins: [react(), viteSingleFile()],
  define: { 'import.meta.env.VITE_SINGLEFILE': 'true' },
  build: { outDir: 'dist-single' },
});
