---
triggers: ["BCI", "brain computer interface", "EEG", "neural interface", "neurofeedback", "brain signals", "OpenBCI", "MNE", "brainflow", "P300", "SSVEP", "motor imagery"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["python3"]
category: scientific
---

# Brain-Computer Interface (BCI) Development

When building BCI systems and neural signal processing pipelines:

1. Use `MNE-Python` for EEG/MEG data processing — load raw data with `mne.io.read_raw_edf()`, `read_raw_brainvision()`, or `read_raw_fif()` depending on the acquisition system.
2. Use `BrainFlow` for real-time data acquisition — `BoardShim(board_id, params).start_stream()` supports 40+ boards (OpenBCI, Muse, Emotiv, g.tec, BioSemi).
3. Apply band-pass filtering before analysis: `raw.filter(l_freq=1.0, h_freq=40.0)` removes DC drift and high-frequency noise; use notch filter at 50/60 Hz for power-line interference.
4. For ERP-based BCIs (P300): epoch data time-locked to stimuli with `mne.Epochs(raw, events, tmin=-0.2, tmax=0.8)`, apply baseline correction, and average across trials.
5. For SSVEP-based BCIs: compute PSD with `mne.time_frequency.psd_welch()` and detect peaks at stimulus frequencies (e.g., 8, 10, 12 Hz) using canonical correlation analysis (CCA).
6. For motor imagery BCIs: extract Common Spatial Patterns (CSP) features with `mne.decoding.CSP(n_components=6)` — these maximize variance differences between classes.
7. Use `scikit-learn` pipelines for classification: `Pipeline([('csp', CSP()), ('clf', LDA())])` — Linear Discriminant Analysis works well for CSP features with small training sets.
8. Apply ICA for artifact removal: `ica = mne.preprocessing.ICA(n_components=20); ica.fit(raw)` — use `ica.plot_components()` to visually identify and exclude eye-blink/muscle artifacts.
9. Implement online processing with circular buffers: `BrainFlow.DataFilter.perform_bandpass()` for real-time filtering; use sliding windows of 1-4 seconds for feature extraction.
10. Use `pylsl` (Lab Streaming Layer) for multi-device synchronization — `StreamInlet` and `StreamOutlet` enable time-synchronized streaming between EEG, eye-tracking, and stimulus systems.
11. Evaluate BCIs with Information Transfer Rate (ITR): `ITR = log2(N) + P*log2(P) + (1-P)*log2((1-P)/(N-1))` bits/trial — report both accuracy and speed for fair comparison.
12. Store data in BIDS format (Brain Imaging Data Structure): use `mne_bids.write_raw_bids()` — standardized directory layout enables reproducibility and data sharing.
13. For deep learning approaches: use EEGNet (`keras`/`pytorch`) — a compact CNN architecture designed for EEG with depthwise and separable convolutions that works well with limited training data.
14. Handle electrode montage: `raw.set_montage('standard_1020')` ensures correct spatial information; always verify channel names match the montage before source localization or topographic plots.
