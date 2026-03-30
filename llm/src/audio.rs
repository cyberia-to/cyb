//! Audio loading and mel spectrogram computation for Whisper.
//!
//! Expects 16-bit PCM WAV input (mono or stereo, any sample rate).
//! Resamples to 16 kHz mono and computes 80-bin log mel spectrogram
//! using the mel filterbank from the GGML model file.

use std::path::Path;

use crate::loader::ggml::MelFilters;

/// Whisper audio parameters
const WHISPER_SAMPLE_RATE: usize = 16000;
const WHISPER_N_FFT: usize = 400;
const WHISPER_HOP_LENGTH: usize = 160;
/// Chunk length in seconds (for reference)
#[allow(dead_code)]
const WHISPER_CHUNK_LENGTH: usize = 30;
/// Maximum frames for 30-second chunk: 16000 * 30 / 160 = 3000
const WHISPER_N_FRAMES: usize = 3000;

/// Load a WAV file and compute the log mel spectrogram.
///
/// Returns a flattened [n_mel, n_frames] array (row-major).
/// n_frames is padded/truncated to 3000 (30 seconds).
pub fn audio_to_mel(path: &Path, filters: &MelFilters) -> Result<(Vec<f32>, usize), String> {
    // Read and decode WAV
    let samples = read_wav_16khz(path)?;
    let n_samples = samples.len();
    log::info!("Audio: {} samples ({:.1}s at 16kHz)", n_samples, n_samples as f64 / WHISPER_SAMPLE_RATE as f64);

    // Compute mel spectrogram
    let mel = compute_mel_spectrogram(&samples, filters)?;
    Ok(mel)
}

/// Read a WAV file and return 16kHz mono f32 samples.
///
/// Supports 16-bit PCM (format 1) WAV files. If stereo, downmixes to mono.
/// If sample rate differs from 16kHz, performs linear resampling.
fn read_wav_16khz(path: &Path) -> Result<Vec<f32>, String> {
    let data = std::fs::read(path)
        .map_err(|e| format!("Cannot read WAV file {}: {e}", path.display()))?;

    if data.len() < 44 {
        return Err("WAV file too short (< 44 bytes header)".to_string());
    }

    // Validate RIFF header
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err("Not a valid WAV file (missing RIFF/WAVE header)".to_string());
    }

    // Parse chunks — find fmt and data
    let mut pos = 12;
    let mut format_code: u16 = 0;
    let mut num_channels: u16 = 0;
    let mut sample_rate: u32 = 0;
    let mut bits_per_sample: u16 = 0;
    let mut audio_data_start: usize = 0;
    let mut audio_data_len: usize = 0;

    while pos + 8 <= data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;

        if chunk_id == b"fmt " {
            if chunk_size < 16 || pos + 8 + chunk_size > data.len() {
                return Err("Invalid fmt chunk".to_string());
            }
            let fmt = &data[pos + 8..];
            format_code = u16::from_le_bytes([fmt[0], fmt[1]]);
            num_channels = u16::from_le_bytes([fmt[2], fmt[3]]);
            sample_rate = u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]);
            bits_per_sample = u16::from_le_bytes([fmt[14], fmt[15]]);
        } else if chunk_id == b"data" {
            audio_data_start = pos + 8;
            audio_data_len = chunk_size.min(data.len() - pos - 8);
        }

        pos += 8 + chunk_size;
        // Chunks are word-aligned
        if chunk_size % 2 != 0 && pos < data.len() {
            pos += 1;
        }
    }

    if format_code != 1 {
        return Err(format!("Unsupported WAV format code {format_code} (only PCM=1 supported)"));
    }
    if bits_per_sample != 16 {
        return Err(format!("Unsupported bits per sample {bits_per_sample} (only 16-bit supported)"));
    }
    if num_channels == 0 || num_channels > 2 {
        return Err(format!("Unsupported channel count {num_channels} (only mono/stereo)"));
    }
    if audio_data_len == 0 {
        return Err("No audio data found in WAV file".to_string());
    }

    log::info!("WAV: {}Hz, {} channels, {}-bit, {} bytes of audio data",
        sample_rate, num_channels, bits_per_sample, audio_data_len);

    // Convert to f32 samples, downmix to mono
    let bytes_per_sample = (bits_per_sample / 8) as usize;
    let frame_size = bytes_per_sample * num_channels as usize;
    let num_frames = audio_data_len / frame_size;

    let mut mono_samples = Vec::with_capacity(num_frames);
    for i in 0..num_frames {
        let frame_start = audio_data_start + i * frame_size;
        if num_channels == 1 {
            let sample = i16::from_le_bytes([
                data[frame_start],
                data[frame_start + 1],
            ]);
            mono_samples.push(sample as f32 / 32768.0);
        } else {
            // Stereo: average both channels
            let left = i16::from_le_bytes([
                data[frame_start],
                data[frame_start + 1],
            ]);
            let right = i16::from_le_bytes([
                data[frame_start + 2],
                data[frame_start + 3],
            ]);
            mono_samples.push((left as f32 + right as f32) / (2.0 * 32768.0));
        }
    }

    // Resample to 16kHz if needed
    if sample_rate as usize != WHISPER_SAMPLE_RATE {
        log::info!("Resampling from {}Hz to {}Hz", sample_rate, WHISPER_SAMPLE_RATE);
        mono_samples = resample_linear(&mono_samples, sample_rate as usize, WHISPER_SAMPLE_RATE);
    }

    Ok(mono_samples)
}

/// Simple linear interpolation resampling
fn resample_linear(samples: &[f32], from_rate: usize, to_rate: usize) -> Vec<f32> {
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let src_idx = src_pos as usize;
        let frac = src_pos - src_idx as f64;

        if src_idx + 1 < samples.len() {
            let val = samples[src_idx] as f64 * (1.0 - frac) + samples[src_idx + 1] as f64 * frac;
            output.push(val as f32);
        } else if src_idx < samples.len() {
            output.push(samples[src_idx]);
        }
    }

    output
}

/// Compute log mel spectrogram from audio samples using the model's mel filterbank.
///
/// Returns (mel_data, n_frames) where mel_data is [n_mel * n_frames] row-major,
/// padded to WHISPER_N_FRAMES (3000).
fn compute_mel_spectrogram(samples: &[f32], filters: &MelFilters) -> Result<(Vec<f32>, usize), String> {
    let n_mel = filters.n_mel;
    let n_fft = WHISPER_N_FFT;
    let hop = WHISPER_HOP_LENGTH;
    let n_fft_bins = n_fft / 2 + 1; // 201

    // Pad samples to be divisible by hop length
    let padded_len = samples.len() + n_fft; // pad with n_fft zeros at the end
    let mut padded = vec![0.0f32; padded_len];
    padded[n_fft / 2..n_fft / 2 + samples.len()].copy_from_slice(samples);

    // Hann window
    let window = hann_window(n_fft);

    // Number of frames
    let n_frames_actual = (padded_len - n_fft) / hop + 1;
    let n_frames = n_frames_actual.min(WHISPER_N_FRAMES);

    log::info!("STFT: {} frames (actual {}), n_fft={}, hop={}", n_frames, n_frames_actual, n_fft, hop);

    // Compute STFT magnitudes for each frame
    let mut magnitudes = vec![0.0f32; n_frames * n_fft_bins];

    for frame in 0..n_frames {
        let start = frame * hop;
        // Apply window and compute DFT
        let mut windowed = vec![0.0f32; n_fft];
        for i in 0..n_fft {
            if start + i < padded.len() {
                windowed[i] = padded[start + i] * window[i];
            }
        }

        // Compute magnitude spectrum (only positive frequencies)
        let spectrum = dft_magnitude(&windowed);
        magnitudes[frame * n_fft_bins..(frame + 1) * n_fft_bins]
            .copy_from_slice(&spectrum[..n_fft_bins]);
    }

    // Apply mel filterbank
    // filters.data is [n_mel, filters.n_fft] — but filters.n_fft is the number of FFT bins
    // In whisper.cpp GGML: filters are stored as [n_mel, n_fft/2+1]
    // filters.n_fft from the file corresponds to n_fft/2+1 = 201 for n_fft=400
    let filter_bins = filters.n_fft; // This is n_fft/2+1 from the model

    if filter_bins != n_fft_bins {
        log::warn!("Mel filter bins ({}) differs from expected n_fft/2+1 ({}), adjusting",
            filter_bins, n_fft_bins);
    }

    let bins = filter_bins.min(n_fft_bins);

    // Output: [n_mel, WHISPER_N_FRAMES]
    let mut mel = vec![0.0f32; n_mel * WHISPER_N_FRAMES];

    for m in 0..n_mel {
        for frame in 0..n_frames {
            let mut sum = 0.0f32;
            for b in 0..bins {
                sum += filters.data[m * filter_bins + b] * magnitudes[frame * n_fft_bins + b];
            }
            // Clamp to avoid log(0)
            sum = sum.max(1e-10);
            mel[m * WHISPER_N_FRAMES + frame] = sum.log10();
        }
    }

    // Apply whisper normalization:
    // 1. Find max value
    let mut max_val = f32::NEG_INFINITY;
    for m in 0..n_mel {
        for frame in 0..n_frames {
            let v = mel[m * WHISPER_N_FRAMES + frame];
            if v > max_val {
                max_val = v;
            }
        }
    }

    // 2. Clamp to max_val - 8.0 (dynamic range of 80 dB)
    let min_val = max_val - 8.0;
    for v in mel.iter_mut() {
        if *v < min_val {
            *v = min_val;
        }
    }

    // 3. Scale to [-1, 1] range: (val - min_val) / (max_val - min_val) * 2 - 1
    // This is equivalent to: (val + 4.0) / 4.0 after the log10 -> multiply by (10/log10(e))
    // Actually whisper uses: (log_mel - max + 8) / 4 - 1 = (log_mel - max) / 4 + 1
    let range = max_val - min_val;
    if range > 0.0 {
        for v in mel.iter_mut() {
            *v = (*v - min_val) / range * 2.0 - 1.0;
        }
    }

    // Zero-pad remaining frames (already done by vec initialization)

    log::info!("Mel spectrogram: [{}, {}] (padded to {})", n_mel, n_frames, WHISPER_N_FRAMES);

    Ok((mel, n_frames))
}

/// Hann window of given length
fn hann_window(len: usize) -> Vec<f32> {
    (0..len)
        .map(|i| {
            let x = std::f32::consts::PI * 2.0 * i as f32 / len as f32;
            0.5 * (1.0 - x.cos())
        })
        .collect()
}

/// Compute magnitude spectrum via DFT (O(n^2) — fine for n_fft=400).
///
/// Returns magnitudes for all n_fft frequencies (only first n_fft/2+1 are useful).
fn dft_magnitude(signal: &[f32]) -> Vec<f32> {
    let n = signal.len();
    let n_bins = n / 2 + 1;
    let mut magnitudes = Vec::with_capacity(n_bins);

    for k in 0..n_bins {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for t in 0..n {
            let angle = -2.0 * std::f64::consts::PI * k as f64 * t as f64 / n as f64;
            re += signal[t] as f64 * angle.cos();
            im += signal[t] as f64 * angle.sin();
        }
        // Magnitude squared (whisper uses power spectrum, not magnitude)
        magnitudes.push((re * re + im * im) as f32);
    }

    magnitudes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hann_window() {
        let w = hann_window(4);
        // 0.5 * (1 - cos(2*pi*i/4))
        // i=0: 0, i=1: 0.5, i=2: 1.0, i=3: 0.5
        assert!((w[0] - 0.0).abs() < 1e-6);
        assert!((w[1] - 0.5).abs() < 1e-6);
        assert!((w[2] - 1.0).abs() < 1e-6);
        assert!((w[3] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_dft_dc() {
        // DC signal should have energy only at bin 0
        let signal = vec![1.0; 8];
        let mag = dft_magnitude(&signal);
        assert!(mag[0] > 60.0); // 8^2 = 64
        for i in 1..mag.len() {
            assert!(mag[i] < 1e-6, "bin {} = {}", i, mag[i]);
        }
    }
}
