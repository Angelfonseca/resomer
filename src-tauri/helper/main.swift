// resomer-audio-helper
//
// Captura el audio del sistema (audio interno) usando ScreenCaptureKit y lo
// escribe a un archivo WAV (PCM 16-bit). Se controla desde Rust: se lanza con
// la ruta de salida como argumento y se detiene enviando SIGTERM/SIGINT, con lo
// que finaliza el archivo WAV de forma segura y sale con código 0.
//
// La seguridad la gestiona el permiso de "Grabación de pantalla" de macOS:
// la primera vez, el sistema pedirá autorización al usuario.

import Foundation
import ScreenCaptureKit
import AVFoundation

// MARK: - Salida de error a stderr

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

// MARK: - Grabador

@available(macOS 13.0, *)
final class SystemAudioCapture: NSObject, SCStreamOutput, SCStreamDelegate {
    private let outputURL: URL
    private var stream: SCStream?
    private var audioFile: AVAudioFile?
    private let sampleQueue = DispatchQueue(label: "com.resomer.audio.sample")
    private let fileLock = NSLock()

    init(outputURL: URL) {
        self.outputURL = outputURL
        super.init()
    }

    func start() async throws {
        // Obtener el contenido compartible dispara el prompt de permiso de
        // Grabación de pantalla si aún no se ha concedido.
        let content = try await SCShareableContent.excludingDesktopWindows(
            false, onScreenWindowsOnly: false
        )

        guard let display = content.displays.first else {
            fail("No hay pantallas disponibles para capturar")
        }

        // Filtro: la pantalla completa. Necesitamos un display aunque solo
        // queramos audio; el video se mantiene al mínimo.
        let filter = SCContentFilter(display: display, excludingWindows: [])

        let config = SCStreamConfiguration()
        config.capturesAudio = true
        config.sampleRate = 48_000
        config.channelCount = 2
        // Excluir el audio de nuestro propio proceso para evitar bucles.
        config.excludesCurrentProcessAudio = true
        // Video mínimo (SCK requiere configuración de video válida).
        config.width = 2
        config.height = 2
        config.minimumFrameInterval = CMTime(value: 1, timescale: 1)
        config.queueDepth = 6

        let stream = SCStream(filter: filter, configuration: config, delegate: self)
        try stream.addStreamOutput(self, type: .audio, sampleHandlerQueue: sampleQueue)
        self.stream = stream

        try await stream.startCapture()
    }

    func stop() {
        // Detener la captura y finalizar el archivo.
        if let stream = stream {
            let sema = DispatchSemaphore(value: 0)
            stream.stopCapture { _ in sema.signal() }
            _ = sema.wait(timeout: .now() + 2.0)
        }
        fileLock.lock()
        audioFile = nil // Al liberar AVAudioFile se finaliza el WAV en disco.
        fileLock.unlock()
    }

    // MARK: SCStreamOutput

    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .audio else { return }
        guard sampleBuffer.isValid else { return }
        guard let formatDesc = sampleBuffer.formatDescription else { return }

        fileLock.lock()
        defer { fileLock.unlock() }

        let inFormat = AVAudioFormat(cmAudioFormatDescription: formatDesc)

        // Crear el archivo de forma perezosa con el formato del primer buffer,
        // forzando PCM entero de 16 bits en disco (compatible con transcripción).
        if audioFile == nil {
            var settings = inFormat.settings
            settings[AVFormatIDKey] = kAudioFormatLinearPCM
            settings[AVLinearPCMBitDepthKey] = 16
            settings[AVLinearPCMIsFloatKey] = false
            settings[AVLinearPCMIsNonInterleaved] = false
            settings[AVLinearPCMIsBigEndianKey] = false
            do {
                audioFile = try AVAudioFile(
                    forWriting: outputURL,
                    settings: settings,
                    commonFormat: .pcmFormatFloat32,
                    interleaved: false
                )
            } catch {
                fail("No se pudo crear el archivo de audio: \(error)")
            }
        }

        guard let audioFile = audioFile else { return }

        let frames = AVAudioFrameCount(CMSampleBufferGetNumSamples(sampleBuffer))
        guard frames > 0 else { return }
        guard let pcmBuffer = AVAudioPCMBuffer(pcmFormat: inFormat, frameCapacity: frames) else { return }
        pcmBuffer.frameLength = frames

        let status = CMSampleBufferCopyPCMDataIntoAudioBufferList(
            sampleBuffer,
            at: 0,
            frameCount: Int32(frames),
            into: pcmBuffer.mutableAudioBufferList
        )
        guard status == noErr else { return }

        do {
            try audioFile.write(from: pcmBuffer)
        } catch {
            // Ignorar fallos puntuales de escritura para no matar la captura.
        }
    }

    // MARK: SCStreamDelegate

    func stream(_ stream: SCStream, didStopWithError error: Error) {
        fail("La captura se detuvo con error: \(error)")
    }
}

// MARK: - Punto de entrada

guard #available(macOS 13.0, *) else {
    fail("Se requiere macOS 13.0 o superior para capturar audio del sistema")
}

let args = CommandLine.arguments
guard args.count >= 2 else {
    fail("Uso: resomer-audio-helper <ruta-salida.wav>")
}

let outputURL = URL(fileURLWithPath: args[1])
let capture = SystemAudioCapture(outputURL: outputURL)

// Manejo de señales para detener limpiamente y finalizar el WAV.
signal(SIGTERM, SIG_IGN)
signal(SIGINT, SIG_IGN)
let sigtermSrc = DispatchSource.makeSignalSource(signal: SIGTERM, queue: .main)
let sigintSrc = DispatchSource.makeSignalSource(signal: SIGINT, queue: .main)
let onStop: () -> Void = {
    capture.stop()
    exit(0)
}
sigtermSrc.setEventHandler(handler: onStop)
sigintSrc.setEventHandler(handler: onStop)
sigtermSrc.resume()
sigintSrc.resume()

// Iniciar la captura.
Task {
    do {
        try await capture.start()
        // Señal de "listo" en stdout para que Rust sepa que arrancó.
        FileHandle.standardOutput.write(Data("READY\n".utf8))
    } catch {
        fail("No se pudo iniciar la captura: \(error)")
    }
}

// Mantener vivo el proceso para recibir callbacks de audio.
RunLoop.main.run()
