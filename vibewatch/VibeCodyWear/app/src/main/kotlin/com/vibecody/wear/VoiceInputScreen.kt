// VoiceInputScreen.kt — On-device speech recognition for Wear OS.
//
// Uses SpeechRecognizer with EXTRA_PREFER_OFFLINE = true so no audio leaves
// the watch.  Shows a confirmation screen before dispatching the transcription.

package com.vibecody.wear

import android.content.Intent
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.wear.compose.material.*

@Composable
fun VoiceInputScreen(onDismiss: () -> Unit, onSend: (String) -> Unit) {
    val context = LocalContext.current
    var phase by remember { mutableStateOf(Phase.LISTENING) }
    var transcript by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }

    val recognizer = remember {
        SpeechRecognizer.createSpeechRecognizer(context).apply {
            setRecognitionListener(object : RecognitionListener {
                override fun onReadyForSpeech(p: Bundle?) {}
                override fun onBeginningOfSpeech() {}
                override fun onRmsChanged(v: Float) {}
                override fun onBufferReceived(b: ByteArray?) {}
                override fun onPartialResults(partial: Bundle?) {}
                override fun onEvent(type: Int, params: Bundle?) {}

                override fun onResults(results: Bundle?) {
                    val texts = results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                    if (!texts.isNullOrEmpty()) {
                        transcript = texts[0]
                        phase = Phase.CONFIRM
                    } else {
                        error = "No speech detected"
                        phase = Phase.ERROR
                    }
                }

                override fun onError(errorCode: Int) {
                    error = speechErrorMessage(errorCode)
                    phase = Phase.ERROR
                }

                override fun onEndOfSpeech() {
                    if (phase == Phase.LISTENING) phase = Phase.PROCESSING
                }
            })
        }
    }

    DisposableEffect(Unit) {
        // Start listening immediately on-device
        val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true)   // privacy: no cloud
            putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)
        }
        recognizer.startListening(intent)
        onDispose { recognizer.destroy() }
    }

    when (phase) {
        Phase.LISTENING -> ListeningUI(onCancel = onDismiss)
        Phase.PROCESSING -> ProcessingUI()
        Phase.CONFIRM -> ConfirmUI(transcript, onConfirm = { onSend(transcript) }, onCancel = onDismiss)
        Phase.ERROR -> ErrorUI(error ?: "Error", onRetry = {
            phase = Phase.LISTENING
            error = null
            val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
                putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
                putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true)
                putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)
            }
            recognizer.startListening(intent)
        }, onCancel = onDismiss)
    }
}

@Composable
private fun ListeningUI(onCancel: () -> Unit) {
    Column(modifier = Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center) {
        Text("🎤", style = MaterialTheme.typography.display2, textAlign = TextAlign.Center)
        Text("Listening…", style = MaterialTheme.typography.body1, textAlign = TextAlign.Center,
            modifier = Modifier.padding(top = 8.dp))
        Button(onClick = onCancel, colors = ButtonDefaults.secondaryButtonColors(),
            modifier = Modifier.padding(top = 12.dp)) { Text("Cancel") }
    }
}

@Composable
private fun ProcessingUI() {
    Column(modifier = Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center) {
        CircularProgressIndicator()
        Text("Processing…", style = MaterialTheme.typography.body2, modifier = Modifier.padding(top = 8.dp))
    }
}

@Composable
private fun ConfirmUI(text: String, onConfirm: () -> Unit, onCancel: () -> Unit) {
    ScalingLazyColumn(modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally) {
        item { Text(text, style = MaterialTheme.typography.body2, textAlign = TextAlign.Center,
            modifier = Modifier.padding(horizontal = 8.dp)) }
        item {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = onConfirm, colors = ButtonDefaults.primaryButtonColors()) { Text("Send") }
                Button(onClick = onCancel, colors = ButtonDefaults.secondaryButtonColors()) { Text("Cancel") }
            }
        }
    }
}

@Composable
private fun ErrorUI(message: String, onRetry: () -> Unit, onCancel: () -> Unit) {
    Column(modifier = Modifier.fillMaxSize(), horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center) {
        Text(message, color = MaterialTheme.colors.error, textAlign = TextAlign.Center,
            style = MaterialTheme.typography.caption2)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.padding(top = 12.dp)) {
            Button(onClick = onRetry, colors = ButtonDefaults.primaryButtonColors()) { Text("Retry") }
            Button(onClick = onCancel, colors = ButtonDefaults.secondaryButtonColors()) { Text("Cancel") }
        }
    }
}

private enum class Phase { LISTENING, PROCESSING, CONFIRM, ERROR }

private fun speechErrorMessage(code: Int) = when (code) {
    SpeechRecognizer.ERROR_AUDIO -> "Audio recording error"
    SpeechRecognizer.ERROR_NO_MATCH -> "No speech recognised"
    SpeechRecognizer.ERROR_SPEECH_TIMEOUT -> "Speech timeout"
    SpeechRecognizer.ERROR_NETWORK -> "Network error (enable offline mode)"
    SpeechRecognizer.ERROR_NOT_SUPPORTED -> "Speech not supported on this device"
    else -> "Error ($code)"
}
