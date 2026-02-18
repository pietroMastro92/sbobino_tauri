#!/usr/bin/env bash
set -euo pipefail

APP_ID="com.sbobino.desktop"
MODELS_DIR="$HOME/Library/Application Support/$APP_ID/models"
MODEL_FILE="ggml-base.bin"
MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$MODEL_FILE"

mkdir -p "$MODELS_DIR"

if [ ! -f "$MODELS_DIR/$MODEL_FILE" ]; then
  echo "Downloading $MODEL_FILE to $MODELS_DIR ..."
  curl -L "$MODEL_URL" -o "$MODELS_DIR/$MODEL_FILE"
else
  echo "Model already present: $MODELS_DIR/$MODEL_FILE"
fi

echo "Runtime setup complete."
