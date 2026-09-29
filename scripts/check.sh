#!/bin/bash
set -euo pipefail

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

echo "[i] Tests Rust..."
cargo test -p resomer-backend || {
  echo "[!] los tests de Rust fallaron"
  exit 1
}

echo "[i] Build Rust..."
cargo build -p resomer-backend --release || {
  echo "[!] el build de Rust falló"
  exit 1
}

echo "[i] TypeScript check..."
pnpm run type-check || {
  echo "[!] TypeScript tiene errores"
  exit 1
}

echo "[i] ESLint..."
pnpm run lint || {
  echo "[!] ESLint encontró problemas"
  exit 1
}

echo "[i] Tests frontend (vitest)..."
pnpm run test || {
  echo "[!] los tests del frontend fallaron"
  exit 1
}

echo "[✓] Todos los checks pasaron."
