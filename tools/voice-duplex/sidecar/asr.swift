// Streaming ASR sidecar — on-device SFSpeechRecognizer.
//
// The point is not a faster model. It is that recognition runs *while the user
// is still speaking*, so end-of-speech only has to finalise. Whole-utterance
// Whisper cost 978 ms because none of its work could start until the turn was
// over; here almost all of it is already done.
//
// stdin  : "PCM" u32(len) i16le[]   feed audio
//          "EOU" u32(0)             end of utterance — finalise
//          "RST" u32(0)             abandon this utterance (barge-in)
// stdout : one JSON object per line — {"partial":"..."} / {"final":"...","ms":n}
import Foundation
import Speech
import AVFoundation

let out = FileHandle.standardOutput
func emit(_ o: [String: Any]) {
  guard let d = try? JSONSerialization.data(withJSONObject: o) else { return }
  out.write(d); out.write("\n".data(using: .utf8)!)
}

final class Asr {
  var localeId = "en-US"
  var recognizer = SFSpeechRecognizer(locale: Locale(identifier: "en-US"))

  /// Switch recognition locale. Returns whether it can run **on-device**;
  /// a locale without its offline asset would otherwise silently send audio to
  /// Apple, which is not a decision this process gets to make quietly.
  @discardableResult
  func setLocale(_ id: String) -> Bool {
    guard let r = SFSpeechRecognizer(locale: Locale(identifier: id)) else { return false }
    finishTask()
    localeId = id
    recognizer = r
    begin()
    return r.supportsOnDeviceRecognition
  }
  var request: SFSpeechAudioBufferRecognitionRequest?
  var task: SFSpeechRecognitionTask?
  var eouAt: Date?
  var lastPartial = ""
  let fmt = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: 16000, channels: 1, interleaved: false)!

  func begin() {
    finishTask()
    let r = SFSpeechAudioBufferRecognitionRequest()
    r.shouldReportPartialResults = true
    // Keep audio on the machine. Also the only way the latency is predictable —
    // a server round trip would dwarf everything else in the budget.
    if recognizer?.supportsOnDeviceRecognition == true { r.requiresOnDeviceRecognition = true }
    request = r
    lastPartial = ""
    task = recognizer?.recognitionTask(with: r) { [weak self] result, error in
      guard let self else { return }
      if let result {
        let text = result.bestTranscription.formattedString
        if result.isFinal {
          let ms = self.eouAt.map { Int(Date().timeIntervalSince($0) * 1000) } ?? -1
          emit(["final": text, "ms": ms])
          self.eouAt = nil
          self.begin()   // ready for the next turn immediately
          return
        }
        if text != self.lastPartial { self.lastPartial = text; emit(["partial": text]) }
      }
      if error != nil {
        // A recognition error still ends the turn; the caller must not wait
        // forever for a final that is never coming.
        let ms = self.eouAt.map { Int(Date().timeIntervalSince($0) * 1000) } ?? -1
        if self.eouAt != nil { emit(["final": self.lastPartial, "ms": ms]); self.eouAt = nil }
        self.begin()
      }
    }
  }

  func finishTask() { task?.cancel(); task = nil; request = nil }

  func feed(_ pcm: [Int16]) {
    guard let request, let buf = AVAudioPCMBuffer(pcmFormat: fmt, frameCapacity: AVAudioFrameCount(pcm.count)) else { return }
    buf.frameLength = AVAudioFrameCount(pcm.count)
    if let ch = buf.floatChannelData {
      for i in 0..<pcm.count { ch[0][i] = Float(pcm[i]) / 32768.0 }
    }
    request.append(buf)
  }

  func endOfUtterance() {
    guard request != nil else { emit(["final": "", "ms": 0]); return }
    if lastPartial.isEmpty {
      // Nothing was heard at all — finalising would just stall the turn.
      emit(["final": "", "ms": 0]); begin(); return
    }
    eouAt = Date()
    request?.endAudio()
  }

  func reset() { begin() }
}

// `--list` — which locales can recognise **offline** on this machine.
if CommandLine.arguments.contains("--list") {
  var onDevice: [String] = [], networkOnly: [String] = []
  for l in SFSpeechRecognizer.supportedLocales().map({ $0.identifier }).sorted() {
    guard let r = SFSpeechRecognizer(locale: Locale(identifier: l)) else { continue }
    if r.supportsOnDeviceRecognition { onDevice.append(l) } else { networkOnly.append(l) }
  }
  let out: [String: Any] = ["onDevice": onDevice, "networkOnly": networkOnly]
  if let d = try? JSONSerialization.data(withJSONObject: out) {
    FileHandle.standardOutput.write(d); FileHandle.standardOutput.write("\n".data(using: .utf8)!)
  }
  exit(0)
}

let asr = Asr()
if let i = CommandLine.arguments.firstIndex(of: "--locale"), i + 1 < CommandLine.arguments.count {
  asr.localeId = CommandLine.arguments[i + 1]
  asr.recognizer = SFSpeechRecognizer(locale: Locale(identifier: asr.localeId))
}
let sema = DispatchSemaphore(value: 0)
SFSpeechRecognizer.requestAuthorization { st in
  if st != .authorized { emit(["error": "speech recognition not authorized (\(st.rawValue))"]) }
  sema.signal()
}
_ = sema.wait(timeout: .now() + 20)
asr.begin()

DispatchQueue.global().async {
  let stdin = FileHandle.standardInput
  func readExactly(_ n: Int) -> Data? {
    var d = Data()
    while d.count < n {
      guard let c = try? stdin.read(upToCount: n - d.count), !c.isEmpty else { return nil }
      d.append(c)
    }
    return d
  }
  while true {
    guard let head = readExactly(7) else { break }
    let tag = String(data: head.prefix(3), encoding: .ascii) ?? ""
    let n = head.subdata(in: 3..<7).withUnsafeBytes { $0.load(as: UInt32.self).littleEndian }
    switch tag {
    case "PCM":
      guard let body = readExactly(Int(n)) else { break }
      let samples = body.withUnsafeBytes { raw -> [Int16] in
        Array(UnsafeBufferPointer(start: raw.baseAddress!.assumingMemoryBound(to: Int16.self), count: Int(n) / 2))
      }
      asr.feed(samples)
    case "EOU": asr.endOfUtterance()
    case "RST": asr.reset()
    case "LOC":
      guard let body = readExactly(Int(n)),
            let id = String(data: body, encoding: .utf8) else { break }
      let onDev = asr.setLocale(id)
      emit(["locale": id, "onDevice": onDev])
    default: break
    }
  }
  exit(0)
}
RunLoop.main.run()
