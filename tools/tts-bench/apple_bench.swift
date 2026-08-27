// Apple AVSpeechSynthesizer bench — one row per (voice, sentence).
//
// Measures the two numbers that matter to a full-duplex conversation, which are
// not the same number:
//
//   first_ms — until the first sample can be played. This is what the user
//              experiences as the assistant's response time.
//   total_ms — until the sentence is fully synthesised. For a streaming engine
//              this is longer than first_ms and largely irrelevant; for a
//              one-pass engine like Kokoro the two are identical, which is the
//              whole comparison.
//
// Writes a WAV per row so the voices can be judged by ear rather than by RTF.
//
// The three AVSpeechSynthesizer traps this works around are documented at
// length in tools/voice-duplex/sidecar/tts.swift; the load-bearing one here is
// that the buffer must be read synchronously inside the callback, because
// AVFoundation recycles it on return.
import Foundation
import AVFoundation

let SENTENCES = [
  "Yes.",
  "The daemon is running on port seven eight seven eight.",
  "I found three functions that call it, all in the same file.",
  "That change looks safe, but it will need a migration for the existing rows.",
  "I could not tell from the file tree alone, so I opened the README to check.",
]

struct Row { let voice: String; let quality: String; let idx: Int
             let firstMs: Double; let totalMs: Double; let audioSec: Double }

func qualityName(_ q: AVSpeechSynthesisVoiceQuality) -> String {
  switch q { case .premium: return "premium"; case .enhanced: return "enhanced"; default: return "compact" }
}

/// Synthesise one sentence, returning timings and the samples.
/// Runs the synthesiser and blocks the calling thread on a semaphore — this is
/// a bench, so serial and simple beats concurrent and clever.
/// One synthesiser for the whole run, as the shipping sidecar does.
///
/// A fresh `AVSpeechSynthesizer` per utterance costs ~185 ms every single time.
/// The first version of this bench allocated one per sentence and measured
/// **245 ms** to first buffer for a voice the sidecar delivers in 15-21 ms — it
/// was benchmarking the harness, not the engine, and would have made every
/// neural alternative look competitive on latency when it is not.
let sharedSynth = AVSpeechSynthesizer()

func synth(_ text: String, voice: AVSpeechSynthesisVoice) -> (Double, Double, [Float], Double)? {
  let synthesizer = sharedSynth
  let u = AVSpeechUtterance(string: text)
  u.voice = voice
  u.rate = 0.52

  var samples = [Float]()
  var rate = 22050.0
  var firstMs: Double? = nil
  let done = DispatchSemaphore(value: 0)
  let t0 = Date()
  var finished = false

  DispatchQueue.main.async {
    synthesizer.write(u) { buf in
      guard let pcm = buf as? AVAudioPCMBuffer else { return }
      if pcm.frameLength == 0 {
        // Zero-length is the end sentinel, and it arrives more than once.
        if !finished { finished = true; done.signal() }
        return
      }
      if firstMs == nil { firstMs = Date().timeIntervalSince(t0) * 1000 }
      rate = pcm.format.sampleRate
      // Read synchronously: the buffer is recycled once this returns.
      if let ch = pcm.floatChannelData?[0] {
        samples.append(contentsOf: UnsafeBufferPointer(start: ch, count: Int(pcm.frameLength)))
      }
    }
  }
  // An utterance with nothing speakable never calls back at all, so a bench
  // that waits forever would hang rather than report.
  if done.wait(timeout: .now() + 20) == .timedOut { return nil }
  guard let f = firstMs else { return nil }
  return (f, Date().timeIntervalSince(t0) * 1000, samples, Double(samples.count) / rate)
}

func writeWav(_ samples: [Float], rate: Double, to path: String) {
  var d = Data()
  let n = samples.count, byteRate = Int(rate) * 2
  func u32(_ v: Int) { var x = UInt32(v).littleEndian; d.append(Data(bytes: &x, count: 4)) }
  func u16(_ v: Int) { var x = UInt16(v).littleEndian; d.append(Data(bytes: &x, count: 2)) }
  d.append("RIFF".data(using: .ascii)!); u32(36 + n * 2); d.append("WAVE".data(using: .ascii)!)
  d.append("fmt ".data(using: .ascii)!); u32(16); u16(1); u16(1)
  u32(Int(rate)); u32(byteRate); u16(2); u16(16)
  d.append("data".data(using: .ascii)!); u32(n * 2)
  for s in samples { var v = Int16(max(-1, min(1, s)) * 32767).littleEndian; d.append(Data(bytes: &v, count: 2)) }
  try? d.write(to: URL(fileURLWithPath: path))
}

let outDir = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "out"
try? FileManager.default.createDirectory(atPath: outDir, withIntermediateDirectories: true)

/// The bench runs on a background thread and `main` runs a RunLoop.
///
/// AVSpeechSynthesizer needs a live run loop on the main thread to deliver its
/// callbacks, and `write` is dispatched there to match the shipping sidecar.
/// The first version of this ran the work *on* main and blocked main on the
/// semaphore, so the work it was waiting for could never be scheduled: every
/// sentence timed out and the run reported zero rows rather than a deadlock.
func runAll() {
  // Every English voice in the modern family, at whatever quality is installed.
  // A downloaded premium voice appears here automatically; if none has been,
  // the run says so rather than quietly reporting only the compact tier.
  let candidates = AVSpeechSynthesisVoice.speechVoices()
    .filter { $0.language.hasPrefix("en-US") && $0.identifier.hasPrefix("com.apple.voice.") }
    .sorted { $0.identifier < $1.identifier }

  var rows = [Row]()
  for v in candidates {
    for (i, s) in SENTENCES.enumerated() {
      // Warm the engine once per voice: the first utterance in a process pays
      // ~300 ms of one-time setup no later turn pays again, and reporting that
      // as this voice's latency would be a lie about every turn after the first.
      if i == 0 { _ = synth("ok", voice: v) }
      guard let (f, t, samples, dur) = synth(s, voice: v) else {
        FileHandle.standardError.write("no callback: \(v.name) #\(i)\n".data(using: .utf8)!)
        continue
      }
      let tag = "apple-\(qualityName(v.quality))-\(v.name)"
      writeWav(samples, rate: Double(samples.count) / dur, to: "\(outDir)/\(tag)-\(i).wav")
      rows.append(Row(voice: v.name, quality: qualityName(v.quality), idx: i,
                      firstMs: f, totalMs: t, audioSec: dur))
    }
  }

  let json = rows.map { r -> [String: Any] in
    ["engine": "apple", "voice": r.voice, "quality": r.quality, "sentence": r.idx,
     "first_ms": r.firstMs, "total_ms": r.totalMs, "audio_sec": r.audioSec,
     "rtf": r.totalMs / 1000 / max(r.audioSec, 0.001)]
  }
  let payload: [String: Any] = [
    "rows": json,
    "voices_found": candidates.count,
    "has_neural": candidates.contains { $0.quality != .default },
  ]
  if let d = try? JSONSerialization.data(withJSONObject: payload) {
    FileHandle.standardOutput.write(d)
  }
  exit(0)
}

DispatchQueue.global().async { runAll() }
RunLoop.main.run()
