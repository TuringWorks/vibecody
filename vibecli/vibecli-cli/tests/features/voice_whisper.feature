Feature: Local voice pipeline has real I/O (US-005)
  The voice_whisper module ships real model-file downloads over HTTP,
  real WAV PCM parsing, and a pluggable Transcriber trait. The
  whisper.cpp FFI is gated behind a build feature so environments
  without the C++ toolchain still get a working test surface.

  Scenario: Downloading a whisper model writes the full body to disk
    Given a mock model server serving 1024 bytes at path "/ggml-tiny.bin"
    When the client downloads that model to a temp file
    Then the temp file size is 1024 bytes
    And the download reports 1024 bytes

  Scenario: Downloading a model reports HTTP errors
    Given a mock model server that returns 404 at path "/missing"
    When the client attempts to download that path to a temp file
    Then the download returns an error mentioning "404"

  Scenario: Loading a 16-bit mono WAV yields the expected sample count
    Given a synthesized 16kHz mono WAV with 800 samples
    When the WAV file is loaded into PCM
    Then the PCM length is 800 samples
    And the sample rate is 16000

  Scenario: NullTranscriber produces a deterministic label for empty input
    When a NullTranscriber transcribes an empty buffer
    Then transcription returns an error mentioning "empty"

  Scenario: NullTranscriber produces a deterministic label for non-empty input
    When a NullTranscriber transcribes a buffer of 1600 samples at 16000 Hz
    Then the transcript contains "1600 samples"
    And the transcript contains "0.10s"
