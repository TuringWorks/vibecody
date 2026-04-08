import { useState, useRef, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useVoiceInput(onTranscript: (text: string) => void) {
  const [isListening, setIsListening] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [interimText, setInterimText] = useState("");
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const recognitionRef = useRef<any>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);

  useEffect(() => {
    return () => {
      if (recognitionRef.current) {
        try { recognitionRef.current.abort(); } catch { /* ignore */ }
      }
    };
  }, []);

  const toggle = useCallback(async () => {
    if (isListening) {
      if (recognitionRef.current) {
        recognitionRef.current.stop();
      } else if (recorderRef.current) {
        recorderRef.current.stop();
      }
      return;
    }

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    if (SpeechRecognition) {
      try {
        const recognition = new SpeechRecognition();
        recognition.continuous = true;
        recognition.interimResults = true;
        recognition.lang = "en-US";
        recognition.maxAlternatives = 1;

        let finalTranscript = "";

        recognition.onresult = (event: { resultIndex: number; results: { length: number; [i: number]: { isFinal: boolean; [j: number]: { transcript: string } } } }) => {
          let interim = "";
          for (let i = event.resultIndex; i < event.results.length; i++) {
            const result = event.results[i];
            if (result.isFinal) {
              finalTranscript += result[0].transcript;
              setInterimText("");
            } else {
              interim += result[0].transcript;
            }
          }
          if (interim) setInterimText(interim);
          if (finalTranscript) {
            onTranscript(finalTranscript);
            finalTranscript = "";
          }
        };

        recognition.onerror = (_event: { error: string }) => {
          setIsListening(false);
          setInterimText("");
          recognitionRef.current = null;
        };

        recognition.onend = () => {
          setIsListening(false);
          setInterimText("");
          recognitionRef.current = null;
        };

        recognition.start();
        recognitionRef.current = recognition;
        setIsListening(true);
      } catch {
        // Speech recognition not available
      }
      return;
    }

    // Fallback: MediaRecorder + Groq Whisper
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mimeType = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
        ? "audio/webm;codecs=opus"
        : "audio/webm";
      const recorder = new MediaRecorder(stream, { mimeType });
      chunksRef.current = [];

      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) chunksRef.current.push(e.data);
      };

      recorder.onstop = async () => {
        stream.getTracks().forEach((t) => t.stop());
        setIsListening(false);

        const blob = new Blob(chunksRef.current, { type: mimeType });
        if (blob.size < 100) return;

        setIsTranscribing(true);
        try {
          const arrayBuf = await blob.arrayBuffer();
          const bytes = new Uint8Array(arrayBuf);
          let binary = "";
          for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
          const base64 = btoa(binary);

          const text = await invoke<string>("transcribe_audio_bytes", {
            audioBase64: base64,
            mimeType: mimeType.split(";")[0],
          });
          if (text.trim()) onTranscript(text);
        } catch {
          // Transcription failed — GROQ_API_KEY may not be set
        }
        setIsTranscribing(false);
      };

      recorder.onerror = () => {
        stream.getTracks().forEach((t) => t.stop());
        setIsListening(false);
      };

      recorder.start();
      recorderRef.current = recorder;
      setIsListening(true);
    } catch {
      // Microphone access denied
    }
  }, [isListening, onTranscript]);

  return { isListening, isTranscribing, interimText, toggle };
}
