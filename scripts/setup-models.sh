#!/bin/bash
set -e

MODELS_DIR="src-tauri/models"
mkdir -p "$MODELS_DIR"

SEGMENTATION_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2"
SEGMENTATION_FINAL="$MODELS_DIR/pyannote-segmentation-3.0.onnx"

EMBEDDING_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_en_voxceleb_16k.onnx"
EMBEDDING_FINAL="$MODELS_DIR/speaker-embedding-campplus-en.onnx"

download_segmentation() {
  if [ -f "$SEGMENTATION_FINAL" ]; then
    echo "[✓] Modelo de segmentación ya presente: $SEGMENTATION_FINAL"
    return
  fi

  echo "[i] Descargando modelo de segmentación (pyannote-segmentation-3.0, ~6.6MB)..."
  local tmp_dir
  tmp_dir=$(mktemp -d)
  curl -sSL "$SEGMENTATION_URL" -o "$tmp_dir/seg.tar.bz2"
  tar xjf "$tmp_dir/seg.tar.bz2" -C "$tmp_dir"
  mv "$tmp_dir/sherpa-onnx-pyannote-segmentation-3-0/model.int8.onnx" "$SEGMENTATION_FINAL"
  rm -rf "$tmp_dir"
  echo "[✓] Modelo de segmentación listo: $SEGMENTATION_FINAL"
}

download_embedding() {
  if [ -f "$EMBEDDING_FINAL" ]; then
    echo "[✓] Modelo de embedding ya presente: $EMBEDDING_FINAL"
    return
  fi

  echo "[i] Descargando modelo de embedding de hablante (CAM++ / 3D-Speaker VoxCeleb, ~28MB)..."
  curl -sSL "$EMBEDDING_URL" -o "$EMBEDDING_FINAL"
  echo "[✓] Modelo de embedding listo: $EMBEDDING_FINAL"
}

echo "[i] Configurando modelos de diarización (sherpa-onnx)..."
download_segmentation
download_embedding

echo "[✓] Modelos listos en $MODELS_DIR/"
echo "    - $SEGMENTATION_FINAL"
echo "    - $EMBEDDING_FINAL"
