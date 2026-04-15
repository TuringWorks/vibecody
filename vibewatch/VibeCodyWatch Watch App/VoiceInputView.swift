// VoiceInputView.swift — Voice / text input for VibeCody Watch App.
//
// On watchOS, tapping a TextField presents the system input panel which includes
// a dictation option, emoji, and scribble — no Speech framework required.
// Audio processing stays on-device via the system speech engine.

import SwiftUI

struct VoiceInputView: View {
    let sessionId: String
    let onSend: (String) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var text = ""
    @FocusState private var isFocused: Bool

    var body: some View {
        VStack(spacing: 10) {
            // Mic icon
            ZStack {
                Circle()
                    .fill(isFocused ? Color.red.opacity(0.2) : Color.blue.opacity(0.15))
                    .frame(width: 56, height: 56)

                Image(systemName: isFocused ? "waveform" : "mic.fill")
                    .font(.system(size: 22))
                    .foregroundStyle(isFocused ? .red : .blue)
            }
            .onTapGesture { isFocused = true }

            // Text field — tap triggers watchOS dictation / keyboard sheet
            TextField("Speak or type…", text: $text)
                .focused($isFocused)
                .font(.caption2)
                .multilineTextAlignment(.center)
                .submitLabel(.send)
                .onSubmit {
                    guard !text.trimmingCharacters(in: .whitespaces).isEmpty else { return }
                    onSend(text)
                    dismiss()
                }

            // Controls
            HStack(spacing: 16) {
                Button { dismiss() } label: {
                    Image(systemName: "xmark").font(.caption)
                }
                .buttonStyle(.bordered)
                .tint(.gray)

                if !text.trimmingCharacters(in: .whitespaces).isEmpty {
                    Button {
                        onSend(text)
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
        .onAppear { isFocused = true }
    }
}
