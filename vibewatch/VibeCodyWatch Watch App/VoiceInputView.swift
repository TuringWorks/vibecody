// VoiceInputView.swift — On-device voice transcription → session dispatch.
//
// Uses WKExtension.shared().requestWhenInUseAuthorization() for mic access.
// On-device SFSpeechRecognizer (no audio leaves the Watch) transcribes speech.
// Once the user confirms, text is dispatched to the session via WatchNetworkManager.

import SwiftUI
import Speech
import AVFoundation

struct VoiceInputView: View {
    let sessionId: String
    let onSend: (String) -> Void

    @Environment(\.dismiss) private var dismiss
    @StateObject private var recognizer = SpeechRecognizer()
    @State private var transcript = ""
    @State private var isListening = false
    @State private var showConfirm = false
    @State private var permissionDenied = false

    var body: some View {
        VStack(spacing: 10) {
            // Waveform / mic indicator
            ZStack {
                Circle()
                    .fill(isListening ? Color.red.opacity(0.2) : Color.blue.opacity(0.15))
                    .frame(width: 60, height: 60)
                    .scaleEffect(isListening ? 1.1 : 1.0)
                    .animation(.easeInOut(duration: 0.6).repeatForever(autoreverses: true), value: isListening)

                Image(systemName: isListening ? "waveform" : "mic.fill")
                    .font(.system(size: 24))
                    .foregroundStyle(isListening ? .red : .blue)
            }

            // Transcript preview
            if !transcript.isEmpty {
                Text(transcript)
                    .font(.caption2)
                    .multilineTextAlignment(.center)
                    .lineLimit(4)
                    .padding(.horizontal, 4)
            } else if permissionDenied {
                Text("Mic access denied.\nEnable in Watch settings.")
                    .font(.caption2)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
            } else {
                Text(isListening ? "Listening…" : "Tap to speak")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }

            // Controls
            HStack(spacing: 16) {
                // Cancel
                Button {
                    recognizer.stop()
                    dismiss()
                } label: {
                    Image(systemName: "xmark")
                        .font(.caption)
                }
                .buttonStyle(.bordered)
                .tint(.gray)

                // Record / Stop
                Button {
                    if isListening {
                        recognizer.stop()
                        isListening = false
                        if !transcript.isEmpty { showConfirm = true }
                    } else {
                        startListening()
                    }
                } label: {
                    Image(systemName: isListening ? "stop.fill" : "mic.fill")
                        .font(.caption)
                }
                .buttonStyle(.bordered)
                .tint(isListening ? .red : .blue)

                // Send (if transcript ready)
                if !transcript.isEmpty {
                    Button {
                        onSend(transcript)
                        dismiss()
                    } label: {
                        Image(systemName: "arrow.up.circle.fill")
                            .font(.title3)
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(.green)
                }
            }
        }
        .padding()
        .onDisappear { recognizer.stop() }
        .confirmationDialog("Send message?", isPresented: $showConfirm, titleVisibility: .visible) {
            Button("Send") {
                onSend(transcript)
                dismiss()
            }
            Button("Re-record") {
                transcript = ""
                startListening()
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text(transcript)
        }
    }

    private func startListening() {
        SFSpeechRecognizer.requestAuthorization { status in
            DispatchQueue.main.async {
                switch status {
                case .authorized:
                    recognizer.start { text in
                        transcript = text
                    }
                    isListening = true
                default:
                    permissionDenied = true
                }
            }
        }
    }
}

// MARK: - SpeechRecognizer (on-device, no network)

@MainActor
final class SpeechRecognizer: ObservableObject {
    private var recognitionTask: SFSpeechRecognitionTask?
    private let audioEngine = AVAudioEngine()
    private let recognizer = SFSpeechRecognizer(locale: .autoupdatingCurrent)

    func start(onPartial: @escaping (String) -> Void) {
        guard let recognizer, recognizer.isAvailable else { return }
        let request = SFSpeechAudioBufferRecognitionRequest()
        request.shouldReportPartialResults = true
        request.requiresOnDeviceRecognition = true // privacy: on-device only

        let node = audioEngine.inputNode
        let fmt = node.outputFormat(forBus: 0)
        node.installTap(onBus: 0, bufferSize: 1024, format: fmt) { buf, _ in
            request.append(buf)
        }
        audioEngine.prepare()
        try? audioEngine.start()

        recognitionTask = recognizer.recognitionTask(with: request) { result, error in
            if let result {
                DispatchQueue.main.async { onPartial(result.bestTranscription.formattedString) }
            }
        }
    }

    func stop() {
        audioEngine.inputNode.removeTap(onBus: 0)
        audioEngine.stop()
        recognitionTask?.cancel()
        recognitionTask = nil
    }
}
