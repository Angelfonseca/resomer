import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    // Puerto propio y fijo: `strictPort` hace que Vite falle en vez de saltar
    // a otro puerto si está ocupado (p. ej. otro proyecto en 5173), lo que
    // dejaba a la ventana de Tauri cargando el frontend equivocado.
    port: 1420,
    strictPort: true,
  },
})
