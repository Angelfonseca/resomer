use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    build_audio_helper();
    tauri_build::build()
}

/// Compila el helper de captura de audio del sistema (Swift + ScreenCaptureKit)
/// solo en macOS. El binario se coloca en OUT_DIR y su ruta se expone al código
/// Rust mediante la variable de entorno RESOMER_AUDIO_HELPER en tiempo de compilación.
fn build_audio_helper() {
    // Este helper solo aplica a macOS.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR no definido");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR no definido");

    let swift_src = Path::new(&manifest_dir).join("helper").join("main.swift");
    let helper_bin = Path::new(&out_dir).join("resomer-audio-helper");

    // Recompilar si cambia el fuente Swift.
    println!("cargo:rerun-if-changed={}", swift_src.display());

    // Exponer la ruta del binario al crate en tiempo de compilación (dev).
    println!(
        "cargo:rustc-env=RESOMER_AUDIO_HELPER={}",
        helper_bin.display()
    );

    let status = Command::new("swiftc")
        .arg("-O")
        .arg("-framework")
        .arg("ScreenCaptureKit")
        .arg("-framework")
        .arg("AVFoundation")
        .arg("-framework")
        .arg("CoreMedia")
        .arg("-o")
        .arg(&helper_bin)
        .arg(&swift_src)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("swiftc falló al compilar el helper de audio (código {s})"),
        Err(e) => panic!("no se pudo ejecutar swiftc: {e}"),
    }

    // Copiar el helper a binaries/<name>-<target-triple> para que Tauri lo
    // empaquete como sidecar (externalBin) dentro de la app. Así el binario
    // queda junto al ejecutable principal y el fallback current_exe lo encuentra.
    let target = env::var("TARGET").unwrap_or_default();
    if !target.is_empty() {
        let binaries_dir = Path::new(&manifest_dir).join("binaries");
        let _ = std::fs::create_dir_all(&binaries_dir);
        let sidecar = binaries_dir.join(format!("resomer-audio-helper-{}", target));
        if let Err(e) = std::fs::copy(&helper_bin, &sidecar) {
            println!(
                "cargo:warning=No se pudo copiar el helper a {}: {}",
                sidecar.display(),
                e
            );
        }
    }
}
