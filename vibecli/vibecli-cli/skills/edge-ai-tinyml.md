---
triggers: ["TinyML", "edge AI", "on-device ML", "model compression", "edge inference", "TensorFlow Lite Micro"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# Edge AI and TinyML

When working with on-device machine learning and TinyML:

1. Use TensorFlow Lite Micro (TFLM) for deploying neural networks on MCUs — convert trained TensorFlow/Keras models to .tflite format, then use the TFLM interpreter with a statically allocated tensor arena sized to fit within the target's available RAM (often 64-256 KB).
2. Apply post-training quantization to reduce model size and improve inference speed — int8 quantization typically cuts model size by 4x with minimal accuracy loss, while float16 quantization offers a 2x reduction and works well on hardware with FP16 support like Cortex-M55 with Helium.
3. Use knowledge distillation to train a small student model that mimics a larger teacher model — the student learns soft label distributions from the teacher, achieving higher accuracy than training from scratch on hard labels alone, which is critical when the deployment target has severe memory constraints.
4. Deploy models with ONNX Runtime for edge scenarios that need cross-framework compatibility — export from PyTorch or TensorFlow to ONNX format, apply ONNX Runtime's graph optimizations (operator fusion, constant folding), and use the execution providers tuned for your target hardware (ARM NN, XNNPACK).
5. Apply structured pruning to remove entire filters or attention heads that contribute least to accuracy, then fine-tune the pruned model to recover performance — this yields genuinely smaller and faster models unlike unstructured sparsity, which requires specialized hardware to realize speed gains.
6. Leverage hardware accelerators when available — route compute-intensive layers to NPUs (Neural Processing Units) or DSPs on chips like the MAX78000, nRF5340, or ESP32-S3, and fall back to optimized CPU kernels (CMSIS-NN on Cortex-M) for layers the accelerator does not support.
7. Schedule inference runs with power awareness — batch sensor readings and run the model periodically rather than continuously, gate the accelerator clock between inferences, and use interrupt-driven wake-up from sleep modes so the MCU draws microamps between inference cycles.
8. Preprocess sensor data on-device before feeding it to the model — apply windowing and FFT for audio (keyword spotting), rolling normalization for accelerometer data (gesture recognition), and downsampling or region-of-interest cropping for image sensors to reduce the input tensor size and inference cost.
9. Implement federated learning to improve models across a fleet of devices without centralizing raw data — devices train on local data, transmit only gradient updates or model deltas to a coordination server, which aggregates them (FedAvg) and distributes the improved global model back to the fleet.
10. Build reproducible model conversion pipelines — script the full flow from training checkpoint to deployment artifact (TFLite, ONNX) using tools like tf.lite.TFLiteConverter or torch.onnx.export, include quantization calibration with representative datasets, and version both the model and the conversion configuration.
11. Evaluate latency vs accuracy tradeoffs systematically — profile inference time on the actual target hardware (not just the host), measure accuracy on a held-out test set after each compression step (quantization, pruning, distillation), and define acceptable thresholds for both metrics before committing to a model architecture.
12. Deploy to popular edge platforms (Arduino Nano 33 BLE Sense, ESP32, Raspberry Pi) by integrating the inference library into the platform's build system — use Arduino libraries or PlatformIO for MCUs, and pip-installable runtimes (tflite-runtime, onnxruntime) for Linux-based boards — and validate end-to-end with real sensor input before field deployment.
