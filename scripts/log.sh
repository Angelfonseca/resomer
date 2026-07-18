#!/bin/bash
set -e

# Uso: ./scripts/log.sh <fase> <tipo> "<descripcion>" [estado]
# tipo: cambio|proceso|hallazgo|verificacion
# estado: ok|fallo|en_progreso (default: en_progreso)

if [ $# -lt 3 ]; then
  echo "Uso: $0 <fase> <tipo> \"<descripcion>\" [estado]"
  exit 1
fi

FASE="$1"
TIPO="$2"
DESC="$3"
ESTADO="${4:-en_progreso}"

# Crear archivo si no existe
PROGRESS_FILE="docs/progress.json"
mkdir -p docs
if [ ! -f "$PROGRESS_FILE" ]; then
  echo "[]" > "$PROGRESS_FILE"
fi

# Añadir entrada con jq
jq --arg ts "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
   --arg fase "$FASE" \
   --arg tipo "$TIPO" \
   --arg desc "$DESC" \
   --arg estado "$ESTADO" \
   '. += [{timestamp: $ts, fase: $fase, tipo: $tipo, descripcion: $desc, estado: $estado}]' \
   "$PROGRESS_FILE" > "$PROGRESS_FILE.tmp" && mv "$PROGRESS_FILE.tmp" "$PROGRESS_FILE"

echo "[✓] $TIPO registrado: $DESC ($ESTADO)"
