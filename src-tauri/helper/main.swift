// resomer-audio-helper
//
// Captura audio del sistema (audio interno) con ScreenCaptureKit y,
// opcionalmente, mezcla el micrófono en la misma pista. Escribe un WAV
// PCM 16-bit. Se controla desde Rust: se lanza con la ruta de salida y,
// para mezclar el micro, con la bandera "--mic". Se detiene enviando
// SIGTERM/SIGINT, con lo que finaliza el WAV de forma segura y sale con 0.
//
// Uso:
//   resomer-audio-helper <salida.wav>          → solo audio del sistema
//   resomer-audio-helper <salida.wav> --mic    → sistema + micrófono (mezcla)
//
// La seguridad la gestionan los permisos de macOS: "Grabación de pantalla"
// (para el audio del sistema) y "Micrófono" (si se usa --mic).

import Foundation
import ScreenCaptureKit
import AVFoundation

// MARK: - Utilidad de error

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

// MARK: - Grabador

@available(macOS 13.0, *)
final class SystemAudioCapture: NSObject, SCStreamOutput, SCStreamDelegate {
    private let outputURL: URL
    private let captureMic: Bool

    private var stream: SCStream?
    private var audioFile: AVAudioFile?
    private let fileLock = NSLock()

    private let sysQueue = DispatchQueue(label: "com.resomer.audio.sys")
    private let micQueue = DispatchQueue(label: "com.resomer.audio.mic")

    // Parámetros de salida comunes.
    private let sampleRate: Double = 48_000
    private let channels: AVAudioChannelCount = 2

    // Buffer FIFO del micrófono (float intercalado estéreo @48k) para mezclar
    // con cada bloque de audio del sistema.
    private var micRing: [Float] = []
    private let micLock = NSLock()
    private var micConverter: AVAudioConverter?
    private var micOutFormat: AVAudioFormat!
    // Límite del FIFO (~2 s) para evitar crecimiento sin límite si hay deriva.
    private lazy var micRingCap = Int(sampleRate) * Int(channels) * 2

    init(outputURL: URL, captureMic: Bool) {
        self.outputURL = outputURL
        self.captureMic = captureMic
        super.init()
    }

    func start() async throws {
        // Obtener el contenido compartible dispara el permiso de Grabación de
        // pantalla si aún no se concedió.
        let content = try await SCShareableContent.excludingDesktopWindows(
            false, onScreenWindowsOnly: false
        )

        guard let display = content.displays.first else {
            fail("No hay pantallas disponibles para capturar")
        }

        let filter = SCContentFilter(display: display, excludingWindows: [])

        let config = SCStreamConfiguration()
        config.capturesAudio = true
        config.sampleRate = Int(sampleRate)
        config.channelCount = Int(channels)
        config.excludesCurrentProcessAudio = true
        // Video mínimo (SCK exige configuración de video válida).
        config.width = 2
        config.height = 2
        config.minimumFrameInterval = CMTime(value: 1, timescale: 1)
        config.queueDepth = 6

        if captureMic {
            if #available(macOS 15.0, *) {
                config.captureMicrophone = true
            } else {
                fail("La mezcla de micrófono requiere macOS 15.0 o superior")
            }
        }

        micOutFormat = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: sampleRate,
            channels: channels,
            interleaved: true
        )

        let stream = SCStream(filter: filter, configuration: config, delegate: self)
        try stream.addStreamOutput(self, type: .audio, sampleHandlerQueue: sysQueue)
        if captureMic, #available(macOS 15.0, *) {
            try stream.addStreamOutput(self, type: .microphone, sampleHandlerQueue: micQueue)
        }
        self.stream = stream

        try await stream.startCapture()
    }

    func stop() {
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
        guard sampleBuffer.isValid else { return }
        switch type {
        case .audio:
            handleSystem(sampleBuffer)
        case .microphone:
            handleMic(sampleBuffer)
        default:
            break
        }
    }

    // MARK: Micrófono → FIFO

    private func handleMic(_ sampleBuffer: CMSampleBuffer) {
        guard captureMic, let formatDesc = sampleBuffer.formatDescription else { return }
        let inFormat = AVAudioFormat(cmAudioFormatDescription: formatDesc)
        let frames = AVAudioFrameCount(CMSampleBufferGetNumSamples(sampleBuffer))
        guard frames > 0 else { return }
        guard let inBuf = AVAudioPCMBuffer(pcmFormat: inFormat, frameCapacity: frames) else { return }
        inBuf.frameLength = frames

        let status = CMSampleBufferCopyPCMDataIntoAudioBufferList(
            sampleBuffer, at: 0, frameCount: Int32(frames), into: inBuf.mutableAudioBufferList
        )
        guard status == noErr else { return }

        // Convertir a 48k estéreo intercalado (si hace falta).
        if micConverter == nil {
            micConverter = AVAudioConverter(from: inFormat, to: micOutFormat)
        }
        guard let converter = micConverter else { return }

        let ratio = micOutFormat.sampleRate / inFormat.sampleRate
        let outCapacity = AVAudioFrameCount(Double(frames) * ratio) + 1024
        guard let outBuf = AVAudioPCMBuffer(pcmFormat: micOutFormat, frameCapacity: outCapacity) else { return }

        var consumed = false
        var convError: NSError?
        converter.convert(to: outBuf, error: &convError) { _, statusPtr in
            if consumed {
                statusPtr.pointee = .noDataNow
                return nil
            }
            consumed = true
            statusPtr.pointee = .haveData
            return inBuf
        }
        if convError != nil { return }

        let outFrames = Int(outBuf.frameLength)
        guard outFrames > 0, let ptr = outBuf.floatChannelData else { return }
        // Formato intercalado → un solo canal de datos con 2 muestras por frame.
        let interleaved = ptr[0]
        let count = outFrames * Int(channels)

        micLock.lock()
        micRing.append(contentsOf: UnsafeBufferPointer(start: interleaved, count: count))
        if micRing.count > micRingCap {
            micRing.removeFirst(micRing.count - micRingCap)
        }
        micLock.unlock()
    }

    /// Extrae `frames` frames estéreo del FIFO del micro (rellena con silencio
    /// si no hay suficientes). Devuelve floats intercalados [L,R,L,R,...].
    private func popMic(frames: Int) -> [Float] {
        let needed = frames * Int(channels)
        var out = [Float](repeating: 0, count: needed)
        micLock.lock()
        let available = min(needed, micRing.count)
        if available > 0 {
            for i in 0..<available { out[i] = micRing[i] }
            micRing.removeFirst(available)
        }
        micLock.unlock()
        return out
    }

    // MARK: Sistema → escribir (mezclando micro si aplica)

    private func handleSystem(_ sampleBuffer: CMSampleBuffer) {
        guard let formatDesc = sampleBuffer.formatDescription else { return }
        let inFormat = AVAudioFormat(cmAudioFormatDescription: formatDesc)

        fileLock.lock()
        defer { fileLock.unlock() }

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
            sampleBuffer, at: 0, frameCount: Int32(frames), into: pcmBuffer.mutableAudioBufferList
        )
        guard status == noErr else { return }

        // Mezclar micrófono si está activo y el formato del sistema es float
        // no-intercalado con 2 canales (lo habitual en ScreenCaptureKit).
        if captureMic, let sysData = pcmBuffer.floatChannelData, inFormat.channelCount >= 2 {
            let n = Int(frames)
            let mic = popMic(frames: n)
            let ch0 = sysData[0]
            let ch1 = sysData[1]
            for i in 0..<n {
                ch0[i] = max(-1.0, min(1.0, ch0[i] + mic[i * 2]))
                ch1[i] = max(-1.0, min(1.0, ch1[i] + mic[i * 2 + 1]))
            }
        }

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
    fail("Uso: resomer-audio-helper <ruta-salida.wav> [--mic]")
}

let outputURL = URL(fileURLWithPath: args[1])
let captureMic = args.contains("--mic")
let capture = SystemAudioCapture(outputURL: outputURL, captureMic: captureMic)

// Señales para detener limpiamente y finalizar el WAV.
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

Task {
    do {
        try await capture.start()
        FileHandle.standardOutput.write(Data("READY\n".utf8))
    } catch {
        fail("No se pudo iniciar la captura: \(error)")
    }
}

RunLoop.main.run()
