package com.vibecody.vibecli

import java.io.File
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * Microphone capture via SoX's `rec`.
 *
 * The same strategy `VoiceDispatcher::listen` uses in the CLI
 * (`vibecli/vibecli-cli/src/voice.rs`) and the VS Code extension uses in
 * `voice-capture.ts` — one capture path for every non-browser client, and one
 * documented dependency rather than three platform-specific audio backends.
 *
 * `javax.sound.sampled` was the alternative. It would avoid the dependency but
 * needs its own device enumeration, WAV muxing and per-platform mixer quirks,
 * and it silently returns a null mixer on several JDK builds — which is the
 * failure mode this whole feature exists to eliminate.
 */
object VoiceRecorder {

    /** Advice printed whenever `rec` is missing. Mirrors voice.rs verbatim. */
    const val SOX_INSTALL_HINT: String =
        "Voice input needs SoX. Install it:\n" +
            "  macOS:   brew install sox\n" +
            "  Linux:   sudo apt install sox\n" +
            "  Windows: choco install sox"

    /** A recording below this is a bare WAV header plus silence, not speech. */
    private const val MIN_USEFUL_BYTES = 2048

    /** True when `rec` is on PATH. */
    fun isAvailable(): Boolean = try {
        val probe = ProcessBuilder("rec", "--version")
            .redirectErrorStream(true)
            .start()
        probe.waitFor(3, TimeUnit.SECONDS)
        probe.destroy()
        true
    } catch (_: IOException) {
        false
    }

    /**
     * An in-flight recording. Call [stop] to finish and read the WAV, or
     * [cancel] to discard it.
     */
    class Session internal constructor(
        private val process: Process,
        private val target: File,
    ) {
        /**
         * Stop recording and return the captured WAV bytes.
         *
         * @throws IOException if SoX produced nothing usable.
         */
        fun stop(): ByteArray {
            // destroy() sends SIGTERM, which SoX handles by finalising the WAV
            // header. destroyForcibly() would leave a header claiming zero
            // frames — a file every decoder reads as empty.
            if (process.isAlive) process.destroy()
            process.waitFor(10, TimeUnit.SECONDS)

            val bytes = if (target.isFile) target.readBytes() else ByteArray(0)
            target.delete()
            if (bytes.size < MIN_USEFUL_BYTES) {
                throw IOException("No speech was recorded.")
            }
            return bytes
        }

        fun cancel() {
            if (process.isAlive) process.destroy()
            target.delete()
        }
    }

    /**
     * Start recording to a temp WAV at 16 kHz mono — the rate every whisper
     * backend resamples to anyway, so recording it directly skips a conversion.
     *
     * @param maxSeconds hard ceiling so a forgotten recording can't fill the disk.
     * @throws IOException with [SOX_INSTALL_HINT] when SoX is absent.
     */
    fun start(maxSeconds: Int = 300): Session {
        val target = File.createTempFile("vibecli-voice-", ".wav").apply { delete() }
        val command = listOf(
            "rec", target.absolutePath,
            "rate", "16000",
            "channels", "1",
            "trim", "0", maxSeconds.toString(),
        )
        val process = try {
            ProcessBuilder(command)
                .redirectOutput(ProcessBuilder.Redirect.DISCARD)
                .redirectError(ProcessBuilder.Redirect.DISCARD)
                .start()
        } catch (e: IOException) {
            target.delete()
            throw IOException(SOX_INSTALL_HINT, e)
        }
        return Session(process, target)
    }
}
