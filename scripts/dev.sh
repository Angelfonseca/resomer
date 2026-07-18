#!/bin/bash
set -e

echo "[i] Levantando app Resomer..."

# Terminar procesos anteriores si existen
pkill -f "tauri dev" || true
sleep 1

# Lanzar en background
npm run tauri dev &
DEV_PID=$!

# Esperar 10s a que levante Vite en puerto 5173
echo "[i] Esperando que Vite levante en :5173..."
for i in {1..30}; do
  if nc -z 127.0.0.1 5173 2>/dev/null; then
    echo "[✓] App lista en :5173"
    break
  fi
  if [ $i -eq 30 ]; then
    echo "[✗] Timeout esperando app"
    kill $DEV_PID 2>/dev/null || true
    exit 1
  fi
  sleep 1
done

echo "[✓] Verificación OK. Mata proceso en 5s..."
sleep 5

echo "[i] Matando proceso..."
kill $DEV_PID 2>/dev/null || true
wait $DEV_PID 2>/dev/null || true

echo "[✓] App verificada y cerrada."
