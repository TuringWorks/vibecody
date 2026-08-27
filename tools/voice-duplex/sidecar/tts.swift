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

/// Which generation of speech synthesis a voice belongs to. Quality alone does
/// not separate them: everything below is `default` quality until a user
/// downloads something, so an unranked pick lands on whatever sorts first —
/// which was Albert, a 1990s novelty voice, rather than Samantha.
///
///  * `com.apple.voice.*`      — the modern Siri-family voices. `compact` is
///                               the built-in tier; `enhanced` and `premium`
///                               are neural and are separate downloads.
///  * `com.apple.ttsbundle.*`  — the previous generation, still shipped for
///                               some locales.
///  * `com.apple.eloquence.*`  — 1980s formant synthesis, kept for the
///                               accessibility users who read at 700 wpm.
///  * `com.apple.speech.synthesis.voice.*` — 1990s MacinTalk, mostly sound
///                               effects (Bells, Boing, Zarvox).
private func family(_ id: String) -> Int {
  if id.hasPrefix("com.apple.voice.") { return 0 }
  if id.hasPrefix("com.apple.ttsbundle.") { return 1 }
  if id.hasPrefix("com.apple.eloquence.") { return 2 }
  return 3
}

/// The MacinTalk namespace is shared by five nameable voices and fourteen sound
/// effects. Only the effects are excluded from *listing*; none of the nineteen
/// is ever chosen automatically.
private let realMacinTalk: Set<String> = ["Albert", "Fred", "Junior", "Kathy", "Ralph"]

private func isNovelty(_ v: AVSpeechSynthesisVoice) -> Bool {
  v.identifier.contains("com.apple.speech.synthesis.voice") && !realMacinTalk.contains(v.name)
}

/// The best installed voice for a language.
///
/// Replaces `AVSpeechSynthesisVoice(language:)`, which returns whatever the
/// system default for that language happens to be. On a machine with only the
/// built-in voices the two agree — both give Samantha compact — so this is not
/// an audible change by itself. It is a change in what the pick is *based on*:
/// installed quality, ranked here, rather than a system setting this process
/// does not control and cannot see.
///
/// **Unverified:** whether the system default follows a premium voice once one
/// is downloaded. There is no premium voice on the machine this was written on,
/// so the case that matters most is the case that could not be tested. Ranking
/// explicitly is correct either way; the claim that it *fixes* something is not
/// made until someone with a premium voice installed checks.
///
/// Premium and enhanced are neural and cost nothing extra at synthesis time.
/// They are separate downloads, so a machine can legitimately have none.
private func bestVoice(for language: String) -> AVSpeechSynthesisVoice? {
  func rank(_ v: AVSpeechSynthesisVoice) -> Int {
    switch v.quality {
    case .premium: return 0
    case .enhanced: return 1
    default: return 2
    }
  }
  let prefix = String(language.prefix(2)).lowercased()
  let candidates = AVSpeechSynthesisVoice.speechVoices().filter {
    !isNovelty($0) && $0.language.lowercased().hasPrefix(prefix)
  }
  // Exact locale, then quality, then generation, then name — the last only so
  // the choice is stable across launches instead of following whatever order
  // the system returned.
  return candidates.min { a, b in
    let exactA = a.language.lowercased() == language.lowercased()
    let exactB = b.language.lowercased() == language.lowercased()
    if exactA != exactB { return exactA }
    if rank(a) != rank(b) { return rank(a) < rank(b) }
    if family(a.identifier) != family(b.identifier) {
      return family(a.identifier) < family(b.identifier)
    }
    return a.name < b.name
  } ?? AVSpeechSynthesisVoice(language: language)
}

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

  /// Audio frame carrying its own sample rate.
  ///
  /// `AUD` meant "22.05 kHz, by construction" and was correct while this was
  /// the only sidecar. It is not any more — the Kokoro sidecar produces 24 kHz
  /// — and a wrong rate does not fail, it plays at the wrong pitch. The rate
  /// now travels with the samples. The daemon still reads `AUD`, so an older
  /// build of this binary keeps working.
  private func emitAudio(_ samples: Data, rate: Double) {
    var r = UInt32(rate).littleEndian
    var payload = Data(bytes: &r, count: 4)
    payload.append(samples)
    emit("AUR", payload)
  }

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
    else { u.voice = bestVoice(for: "en-US") }
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
        mono.withUnsafeBufferPointer { p in self.emitAudio(Data(buffer: p), rate: sr) }
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
       "novelty": isNovelty(v),
       // 0 modern · 1 previous-gen · 2 eloquence · 3 MacinTalk. A picker that
       // lists 180 voices flat is not a choice, it is a phone book.
       "generation": family(v.identifier)]
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
    // What an unconfigured turn will actually use, so a caller can show it
    // rather than guessing that "default" means the system default.
    "default": bestVoice(for: "en-US")?.identifier ?? "",
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
