// Resident TTS sidecar — streams PCM from AVSpeechSynthesizer.
//
// `say` costs ~690 ms per utterance, almost all of it process spawn, and cannot
// write to a pipe. AVSpeechSynthesizer.write streams buffers as they are
// produced from a process that stays alive: ~290 ms for the first utterance,
// ~25 ms for every one after.
//
// Three behaviours of that API drove this design, each found the hard way:
//
//  1. It never invokes the callback at all for an utterance with nothing
//     speakable in it — no buffers, no completion. A reader waiting for a
//     terminator deadlocks.
//  2. It delivers *more than one* zero-length trailing callback per utterance.
//  3. The buffer must be read **synchronously** inside the callback.
//     AVFoundation recycles it on return, so a deferred read sees frameLength 0
//     — indistinguishable from the end-of-utterance sentinel, which silently
//     truncated the *next* utterance to zero samples.
//
// So: one shared synthesizer, utterances queued and started one at a time,
// every callback tagged with the id of the utterance that produced it, all
// mutable state on one serial queue, and the buffer copied before dispatching.
//
// Measured: ~300 ms to first buffer for the first utterance in a process,
// **15–21 ms** for every one after. A fresh synthesizer per utterance was tried
// and costs ~185 ms every time for no reliability gain.
//
// One trap worth recording because it cost an hour: a consumer that does not
// drain stdout continuously will block this process on a full pipe partway
// through an utterance. That looks exactly like "the synthesizer stopped
// delivering callbacks" and it is not.
//
// stdin  : one JSON object per line — {"text":"..."} or {"cmd":"cancel"}
// stdout : "AUD" u32(len) f32le[]  |  "END" u32(0)
import Foundation
import AVFoundation

let stdoutHandle = FileHandle.standardOutput
let stderrHandle = FileHandle.standardError

final class Sidecar {
  private let shared = AVSpeechSynthesizer()
  private var live: [Int: AVSpeechSynthesizer] = [:]
  /// Every mutable field below is touched only inside this queue.
  private let q = DispatchQueue(label: "tts.state")
  private var queue: [(String, String?, Float)] = []
  private var busy = false
  private var utteranceId = 0
  private var t0 = Date()
  private var firstBuffer = true

  private func emit(_ tag: String, _ payload: Data) {
    var d = tag.data(using: .ascii)!
    var len = UInt32(payload.count).littleEndian
    d.append(Data(bytes: &len, count: 4))
    d.append(payload)
    stdoutHandle.write(d)
  }

  func enqueue(_ text: String, voice: String?, rate: Float) {
    q.async { self.queue.append((text, voice, rate)); self.pump() }
  }

  func cancel() {
    q.async {
      self.queue.removeAll()
      self.shared.stopSpeaking(at: .immediate)
      // Supersede: any in-flight callback now carries a stale id.
      self.utteranceId += 1
      if self.busy { self.emit("END", Data()); self.busy = false }
      self.pump()
    }
  }

  /// q-only.
  private func pump() {
    guard !busy, !queue.isEmpty else { return }
    busy = true
    let (text, voice, rate) = queue.removeFirst()
    utteranceId += 1
    let myId = utteranceId
    t0 = Date()
    firstBuffer = true

    guard text.rangeOfCharacter(from: CharacterSet.alphanumerics) != nil else {
      emit("END", Data()); busy = false; pump(); return
    }

    let synth = shared
    let u = AVSpeechUtterance(string: text)
    if let v = voice, let av = AVSpeechSynthesisVoice(identifier: v) { u.voice = av }
    else { u.voice = AVSpeechSynthesisVoice(language: "en-US") }
    u.rate = rate

    DispatchQueue.main.async {
    synth.write(u) { [weak self] buf in
      guard let self else { return }
      // Read the buffer *synchronously*. AVFoundation recycles it once this
      // callback returns, so deferring the read to another queue hands you a
      // buffer whose frameLength is now 0 — indistinguishable from the
      // end-of-utterance sentinel, which silently truncated the next utterance
      // to zero samples. Copy first, dispatch second.
      guard let pcm = buf as? AVAudioPCMBuffer, pcm.frameLength > 0 else {
        self.q.async { self.finish(myId) }
        return
      }
      let frames = Int(pcm.frameLength)
      let ch = Int(pcm.format.channelCount)
      var mono = [Float](repeating: 0, count: frames)
      if let f = pcm.floatChannelData {
        for i in 0..<frames {
          var acc: Float = 0
          for c in 0..<ch { acc += f[c][i] }
          mono[i] = acc / Float(ch)
        }
      } else if let i16 = pcm.int16ChannelData {
        for i in 0..<frames {
          var acc: Float = 0
          for c in 0..<ch { acc += Float(i16[c][i]) / 32768.0 }
          mono[i] = acc / Float(ch)
        }
      }
      let sr = pcm.format.sampleRate
      self.q.async {
        guard myId == self.utteranceId, self.busy else { return }   // stale callback
        if self.firstBuffer {
          self.firstBuffer = false
          stderrHandle.write("first-buffer \(Int(Date().timeIntervalSince(self.t0)*1000))ms sr=\(sr)\n".data(using: .utf8)!)
        }
        mono.withUnsafeBufferPointer { p in self.emit("AUD", Data(buffer: p)) }
      }
    }
    }
  }

  /// q-only. Terminate utterance `id` exactly once, ignoring stale callbacks.
  private func finish(_ id: Int) {
    guard id == utteranceId, busy else { return }
    emit("END", Data())
    stderrHandle.write("done \(Int(Date().timeIntervalSince(t0)*1000))ms\n".data(using: .utf8)!)
    busy = false
    pump()
  }
}

/// `--list` — enumerate installed voices as JSON and exit.
///
/// Quality matters more than identity here: `default` is the compact,
/// concatenative tier and is what people mean when they say a system voice
/// sounds robotic. `enhanced` and `premium` are neural and are separate
/// downloads, so a machine can easily have none of them.
if CommandLine.arguments.contains("--list") {
  func tier(_ v: AVSpeechSynthesisVoice) -> String {
    switch v.quality {
    case .premium: return "premium"
    case .enhanced: return "enhanced"
    default: return "default"
    }
  }
  let rank = ["premium": 0, "enhanced": 1, "default": 2]
  let voices = AVSpeechSynthesisVoice.speechVoices()
    .map { v -> [String: Any] in
      ["id": v.identifier, "name": v.name, "lang": v.language, "quality": tier(v),
       // The novelty voices (Bells, Boing, Bubbles…) are not speech assistants.
       "novelty": v.identifier.contains("com.apple.speech.synthesis.voice")
                  && !["Albert", "Fred", "Junior", "Kathy", "Ralph"].contains(v.name)]
    }
    .sorted {
      let a = rank[$0["quality"] as! String]!, b = rank[$1["quality"] as! String]!
      if a != b { return a < b }
      return ($0["name"] as! String) < ($1["name"] as! String)
    }
  let langs = Set(voices.map { String(($0["lang"] as! String).prefix(2)) }).sorted()
  let out: [String: Any] = [
    "voices": voices,
    "languages": langs,
    "default": AVSpeechSynthesisVoice(language: "en-US")?.identifier ?? "",
    "hasNeural": voices.contains {
      ($0["quality"] as! String) != "default" && ($0["lang"] as! String).hasPrefix("en")
    },
  ]
  if let d = try? JSONSerialization.data(withJSONObject: out) {
    FileHandle.standardOutput.write(d); FileHandle.standardOutput.write("\n".data(using: .utf8)!)
  }
  exit(0)
}

let sc = Sidecar()

if CommandLine.arguments.contains("--bench") {
  for t in ["Hello.", "The capital of France is Paris.", "Sure, I can help with that."] {
    sc.enqueue(t, voice: nil, rate: 0.52)
  }
  let deadline = Date().addingTimeInterval(8)
  while Date() < deadline { RunLoop.current.run(until: Date().addingTimeInterval(0.02)) }
  exit(0)
}

DispatchQueue.global().async {
  while let line = readLine(strippingNewline: true) {
    guard let d = line.data(using: .utf8),
          let o = try? JSONSerialization.jsonObject(with: d) as? [String: Any] else { continue }
    if (o["cmd"] as? String) == "cancel" { sc.cancel(); continue }
    if let t = o["text"] as? String, !t.isEmpty {
      sc.enqueue(t, voice: o["voice"] as? String, rate: (o["rate"] as? NSNumber)?.floatValue ?? 0.52)
    }
  }
  exit(0)
}
RunLoop.main.run()
