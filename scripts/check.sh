#!/bin/bash
set -e

echo "[i] Corriendo checks..."

echo "[i] Formato Rust (rustfmt)..."
cargo fmt --check || {
  echo "[!] rustfmt encontró problemas. Ejecuta: cargo fmt"
  exit 1
}

echo "[i] Linter Rust (clippy)..."
cargo clippy -p resomer-backend -- -D warnings || {
  echo "[!] clippy encontró warnings"
  exit 1
}

echo "[i] Build Rust..."
cargo build -p resomer-backend --release 2>&1 | tail -20

echo "[i] TypeScript check..."
npm run type-check || {
  echo "[!] TypeScript tiene errores"
  exit 1
}

echo "[i] ESLint..."
npm run lint || {
  echo "[!] ESLint encontró problemas"
  exit 1
}

echo "[✓] Todos los checks pasaron."
