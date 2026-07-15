import CoreGraphics
import CoreMedia
import Dispatch
import Foundation
import ScreenCaptureKit

// MARK: - Captura del audio del sistema (ScreenCaptureKit) para Sesiones
// Compilado por build.rs como librería estática, igual que el puente de
// Apple Intelligence. Rust arranca/para la captura y drena un ring buffer
// de muestras Float32 a 16 kHz mono (el formato nativo del motor de
// transcripción, sin resamplear). Requiere macOS 13+ y el permiso de
// Grabación de pantalla; en sistemas anteriores todo degrada a "no soportado".

/// Tope del ring buffer: ~60 s. Si Rust deja de drenar (no debería), se
/// descarta lo más antiguo en vez de crecer sin límite.
private let maxBufferedSamples = 16_000 * 60

@available(macOS 13.0, *)
private final class SystemAudioCapture: NSObject, SCStreamOutput, @unchecked Sendable {
    static let shared = SystemAudioCapture()

    private let lock = NSLock()
    private var samples: [Float] = []
    private var stream: SCStream?

    func stream(
        _ stream: SCStream, didOutputSampleBuffer sb: CMSampleBuffer, of type: SCStreamOutputType
    ) {
        guard type == .audio, sb.isValid, sb.numSamples > 0 else { return }
        try? sb.withAudioBufferList { abl, _ in
            lock.lock()
            defer { lock.unlock() }
            for buf in abl {
                guard let data = buf.mData else { continue }
                let n = Int(buf.mDataByteSize) / MemoryLayout<Float>.size
                let ptr = data.assumingMemoryBound(to: Float.self)
                samples.append(contentsOf: UnsafeBufferPointer(start: ptr, count: n))
            }
            if samples.count > maxBufferedSamples {
                samples.removeFirst(samples.count - maxBufferedSamples)
            }
        }
    }

    /// Copia hasta `max` muestras al buffer de Rust y las retira del ring.
    func drain(into out: UnsafeMutablePointer<Float>, max: Int) -> Int {
        lock.lock()
        defer { lock.unlock() }
        let n = Swift.min(max, samples.count)
        if n > 0 {
            samples.withUnsafeBufferPointer { src in
                out.update(from: src.baseAddress!, count: n)
            }
            samples.removeFirst(n)
        }
        return n
    }

    /// Arranca el stream SCK (bloqueante: puentea el async con un semáforo,
    /// mismo patrón que el puente de Apple Intelligence).
    func start() -> Int32 {
        if stream != nil { return 0 }  // ya capturando

        final class ResultBox: @unchecked Sendable {
            var code: Int32 = 3
            var stream: SCStream?
        }
        let box = ResultBox()
        let semaphore = DispatchSemaphore(value: 0)

        Task.detached(priority: .userInitiated) {
            defer { semaphore.signal() }
            do {
                let content = try await SCShareableContent.excludingDesktopWindows(
                    false, onScreenWindowsOnly: false)
                guard let display = content.displays.first else { return }
                let filter = SCContentFilter(
                    display: display, excludingApplications: [], exceptingWindows: [])
                let config = SCStreamConfiguration()
                config.capturesAudio = true
                config.sampleRate = 16_000
                config.channelCount = 1
                // Que Escriba no se escuche a sí misma (sonidos de la app, TTS).
                config.excludesCurrentProcessAudio = true
                // SCK exige un stream de vídeo aunque solo queramos el audio:
                // se pide el mínimo posible (2x2 px, 1 frame por segundo).
                config.width = 2
                config.height = 2
                config.minimumFrameInterval = CMTime(value: 1, timescale: 1)

                let stream = SCStream(filter: filter, configuration: config, delegate: nil)
                try stream.addStreamOutput(
                    SystemAudioCapture.shared, type: .audio,
                    sampleHandlerQueue: DispatchQueue(label: "escriba.sysaudio"))
                try await stream.startCapture()
                box.stream = stream
                box.code = 0
            } catch {
                // Sin permiso de Grabación de pantalla, SCShareableContent lanza
                // "user declined". Se distingue para que la UI guíe al usuario.
                let desc = error.localizedDescription.lowercased()
                box.code = desc.contains("declined") ? 2 : 3
                FileHandle.standardError.write(
                    "escriba sysaudio start error: \(error)\n".data(using: .utf8)!)
            }
        }
        semaphore.wait()

        if box.code == 0 {
            lock.lock()
            samples.removeAll()
            lock.unlock()
            stream = box.stream
        }
        return box.code
    }

    func stop() {
        guard let s = stream else { return }
        stream = nil
        let semaphore = DispatchSemaphore(value: 0)
        Task.detached {
            defer { semaphore.signal() }
            try? await s.stopCapture()
        }
        semaphore.wait()
        lock.lock()
        samples.removeAll()
        lock.unlock()
    }
}

// MARK: - C ABI para Rust

/// ¿Este equipo puede capturar audio del sistema? (macOS 13+ con SCK)
@_cdecl("escriba_sysaudio_supported")
public func escribaSysaudioSupported() -> Int32 {
    if #available(macOS 13.0, *) { return 1 }
    return 0
}

/// ¿Está concedido el permiso de Grabación de pantalla?
@_cdecl("escriba_sysaudio_permission_granted")
public func escribaSysaudioPermissionGranted() -> Int32 {
    return CGPreflightScreenCaptureAccess() ? 1 : 0
}

/// Pide el permiso (muestra el diálogo del sistema la primera vez; después
/// el usuario debe activarlo a mano en Ajustes del Sistema).
@_cdecl("escriba_sysaudio_request_permission")
public func escribaSysaudioRequestPermission() -> Int32 {
    return CGRequestScreenCaptureAccess() ? 1 : 0
}

/// Arranca la captura. 0 = ok · 1 = no soportado · 2 = sin permiso · 3 = error.
@_cdecl("escriba_sysaudio_start")
public func escribaSysaudioStart() -> Int32 {
    guard #available(macOS 13.0, *) else { return 1 }
    return SystemAudioCapture.shared.start()
}

/// Detiene la captura y vacía el ring buffer.
@_cdecl("escriba_sysaudio_stop")
public func escribaSysaudioStop() {
    guard #available(macOS 13.0, *) else { return }
    SystemAudioCapture.shared.stop()
}

/// Drena hasta `max` muestras (Float32, 16 kHz mono) al buffer de Rust.
/// Devuelve cuántas escribió.
@_cdecl("escriba_sysaudio_read")
public func escribaSysaudioRead(_ out: UnsafeMutablePointer<Float>, _ max: Int32) -> Int32 {
    guard #available(macOS 13.0, *) else { return 0 }
    return Int32(SystemAudioCapture.shared.drain(into: out, max: Int(max)))
}
