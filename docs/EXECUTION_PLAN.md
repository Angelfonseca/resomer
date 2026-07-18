# EXECUTION_PLAN — Resomer

Guía paso a paso para ejecutar el plan de desarrollo. **Para un modelo más pequeño y rápido.**

## Reglas obligatorias

1. **Trabajar UNA subfase a la vez, en orden.** No saltar.
2. **ANTES de marcar completa** cualquier subfase, ejecutar su verificación:
   - `./scripts/dev.sh` (si dice "App lista en :5173", OK)
   - `./scripts/check.sh` (si no hay errores, OK)
   - Según subfase: pruebas específicas en "Verificación" de abajo
3. Registrar cada cambio/hallazgo/proceso con:
   ```bash
   ./scripts/log.sh <FASE> cambio "Descripción" ok
   ./scripts/log.sh <FASE> hallazgo "Problema/Insight" ok
   ./scripts/log.sh <FASE> verificacion "Qué se verificó" ok
   ```
4. **No avanzar de fase** si la verificación falla. Debuguear y registrar el fallo.
5. Respetar **SOLID/clean code**: comandos finos, lógica en services detrás de traits.

---

## Fase 0 — Andamiaje y tooling

### 0.1 — Init Tauri v2 + React + TS + Vite
**Estado:** ✓ COMPLETO (scripts, configs, estructura base creada)

**Qué se hizo:**
- Creó `Cargo.toml` (workspace + backend)
- Creó `package.json` con deps (React, Tauri, Tailwind v4)
- Inicializó `src-tauri/Cargo.toml` con dependencias backend
- Creó `src-tauri/src/main.rs` (bootstrap Tauri mínimo)
- Creó `src/main.tsx`, `src/App.tsx`, `src/index.css` (frontend mínimo)
- Creó `index.html`, `vite.config.ts`, `tsconfig.json`

**Verificación:**
- [ ] Ejecutar: `npm install` (instala deps frontend + Tauri CLI)
- [ ] Ejecutar: `cargo fetch -p resomer-backend` (descarga deps Rust)
- [ ] Registrar: `./scripts/log.sh 0 verificacion "npm install y cargo fetch OK" ok`

---

### 0.2 — Tailwind v4 configurado
**Estado:** 📋 PENDIENTE

**Qué hacer:**
1. Crear `tailwind.config.ts` en raíz
2. Crear `globals.css` con `@import "tailwindcss"` (Tailwind v4 usa CSS imports)
3. Modificar `src/index.css` para usar `globals.css`
4. Test: Usar clases Tailwind en `src/App.tsx` (ya hay `bg-slate-950`, `text-white`, etc.)

**Verificación:**
- [ ] `npm run type-check` sin errores
- [ ] `./scripts/dev.sh` abre app con bg oscuro y texto blanco
- [ ] Registrar: `./scripts/log.sh 0 verificacion "Tailwind v4 funciona" ok`

---

### 0.3 — Estructura de carpetas SOLID
**Estado:** 📋 PENDIENTE

**Qué hacer:**
```
src-tauri/src/
├── main.rs                    # bootstrap (ya existe)
├── lib.rs                     # (nuevo) exports públicos
├── error.rs                   # (nuevo) ResomerError unificado
├── domain/                    # (crear) entidades + traits (ports)
│   ├── mod.rs
│   ├── meeting.rs
│   ├── segment.rs
│   └── ports.rs               # traits: AudioRecorder, DiarizationEngine, etc.
├── commands/                  # (crear) handlers Tauri
│   ├── mod.rs
│   └── meeting.rs
├── services/                  # (crear) implementaciones
│   ├── mod.rs
│   ├── recording.rs
│   ├── diarization.rs
│   ├── transcription.rs
│   ├── summarization.rs
│   └── storage.rs
└── infra/                     # (crear) http, config, keychain, etc.
    ├── mod.rs
    ├── http_client.rs
    ├── config.rs
    └── keychain.rs

src/
├── features/                  # (crear) módulos por feature
│   ├── meetings/
│   │   ├── components/
│   │   ├── hooks/
│   │   └── services/
│   ├── recording/
│   ├── transcript/
│   ├── summary/
│   └── settings/
├── components/
│   ├── ui/                    # (crear) componentes reutilizables
│   └── mod.ts
├── stores/                    # (crear) zustand stores
└── types/                     # (crear) tipos compartidos
```

Crear los archivos `mod.rs` (vacíos o con pub use) en cada carpeta.

**Verificación:**
- [ ] `cargo build -p resomer-backend` compila (mod.rs vacío es OK)
- [ ] `npm run type-check` pasa
- [ ] Registrar: `./scripts/log.sh 0 verificacion "Estructura SOLID creada" ok`

---

### 0.4 — scripts/log.sh + docs/progress.json
**Estado:** ✓ COMPLETO (script creado)

**Verificación:**
- [ ] Ejecutar: `./scripts/log.sh 0 cambio "Test del script" ok`
- [ ] Comprobar: `cat docs/progress.json | jq .` contiene la entrada
- [ ] Registrar: `./scripts/log.sh 0 verificacion "log.sh funciona" ok`

---

### 0.5 — scripts/dev.sh y scripts/check.sh
**Estado:** ⚠️ PARCIAL (scripts creados, pero npm install pendiente)

**Qué hacer:**
1. Ejecutar: `npm install`
2. Ejecutar: `cargo fetch -p resomer-backend`

**Verificación:**
- [ ] `./scripts/check.sh` pasa (sin warnings) — puede fallar ahora si hay módulos sin uso
- [ ] `./scripts/dev.sh` levanta app y la mata limpio
- [ ] Registrar: `./scripts/log.sh 0 verificacion "dev.sh y check.sh OK" ok`

---

### 0.6 — docs/EXECUTION_PLAN.md
**Estado:** ✓ COMPLETO (este archivo)

---

### 0.7 — Config de linters y formatters
**Estado:** ✓ COMPLETO (creados)
- `rustfmt.toml`
- `.eslintrc.json`
- `.prettierrc.json`
- `tsconfig.json`

**Verificación:**
- [ ] `cargo fmt --check` no reclama cambios
- [ ] `npm run lint` pasa
- [ ] Registrar: `./scripts/log.sh 0 verificacion "Linters OK" ok`

---

### 0.8 — Verificación final: App arranca
**Estado:** 📋 PENDIENTE

**Qué hacer:**
1. Ejecutar: `npm install`
2. Ejecutar: `./scripts/dev.sh`
3. Comprobar que levanta sin errores y se cierra limpio
4. Registrar: `./scripts/log.sh 0 verificacion "App arranca sin errores" ok`

**Esperado:**
- Log: `[✓] App lista en :5173`
- Log: `[✓] App verificada y cerrada.`

---

## Fase 1 — Configuración y persistencia base

**Subfases (en orden):**
- 1.1 Config (base URL, endpoints, env)
- 1.2 Keychain (guardar/leer/borrar API key)
- 1.3 SQLite schema + migraciones
- 1.4 Trait `MeetingRepository` + impl
- 1.5 UI: Settings (guardar key, probar conexión)
- 1.6 Verificación: CRUD de Meeting persiste

**Regla:** No avanzar de subfase sin pasar su verificación.

---

## Fase 2 — Grabación de audio

**Subfases:**
- 2.1 Trait `AudioRecorder` + entidades
- 2.2 Captura micro (cpal + hound → WAV 16 kHz mono)
- 2.3 Captura audio sistema (macOS: ScreenCaptureKit o BlackHole)
- 2.4 Mezcla si se eligen ambas
- 2.5 Comandos Tauri start/stop/pause
- 2.6 UI: pantalla grabación
- 2.7 Verificación: WAV en disco, reproducible, correcta fuente

---

## Fase 3 — Diarización local (PyAnnote ONNX vía sherpa-onnx)

**Subfases:**
- 3.1 `./scripts/setup-models.sh` (descargar ONNX)
- 3.2 Trait `DiarizationEngine`
- 3.3 Wrapper sherpa-onnx
- 3.4 Ejecutar diarización → segmentos
- 3.5 Persistir segmentos
- 3.6 Verificación: audio multi-hablante → segmentos coherentes

---

## Fase 4 — Chunking + transcripción cloud (Whisper)

**Subfases:**
- 4.1 Trait `Transcriber` + cliente reqwest
- 4.2 Chunking ≤300s por turno de hablante
- 4.3 Llamada `/audio/transcriptions` con reintentos
- 4.4 Alineación transcript ↔ diarización
- 4.5 Persistir transcript
- 4.6 UI: vista transcript por hablante
- 4.7 Verificación: audio real → transcript en español, atribuido

---

## Fase 5 — Resumen con LLM

**Subfases:**
- 5.1 Trait `Summarizer` + cliente LLM (mimo-v2.5)
- 5.2 Prompt en español
- 5.3 Manejo transcripts largos
- 5.4 Persistir resumen
- 5.5 UI: vista resumen
- 5.6 Verificación: transcript → resumen correcto en español

---

## Fase 6 — Orquestación del pipeline + UX

**Subfases:**
- 6.1 Orquestador pipeline
- 6.2 Eventos de progreso
- 6.3 Lista reuniones + detalle
- 6.4 Export md/txt
- 6.5 Errores globales + reanudar
- 6.6 Verificación: E2E grabar → diarizar → transcribir → resumir

---

## Fase 7 — Pulido, empaquetado y QA

**Subfases:**
- 7.1 Tests unitarios, sin warnings
- 7.2 UI polish, español consistente
- 7.3 `tauri build` release
- 7.4 README final

---

## Cómo debuguear si falla algo

1. Leer el output del script que falló.
2. Si es Rust: `cargo clippy -p resomer-backend` para más detalles.
3. Si es Node: `npm run lint` o `npm run type-check`.
4. Registrar el fallo: `./scripts/log.sh <FASE> hallazgo "Error: ..." fallo`
5. Fixear.
6. Registrar fix: `./scripts/log.sh <FASE> cambio "Arreglado: ..." ok`

---

## Quick reference: comandos útiles

```bash
# Instalar dependencias
npm install
cargo fetch -p resomer-backend

# Desarrollar
npm run tauri dev

# Verificar calidad
./scripts/check.sh
./scripts/dev.sh

# Registrar progreso
./scripts/log.sh 0 verificacion "Descripción" ok
./scripts/log.sh 0 hallazgo "Problema encontrado" fallo

# Ver historial
cat docs/progress.json | jq .

# Formato y lint automático
cargo fmt -p resomer-backend
npm run format
npm run lint -- --fix
```

---

**Creado:** 2026-07-17
**Última actualización:** Durante Fase 0
