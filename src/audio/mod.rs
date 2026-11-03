use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result, anyhow, bail};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

pub trait AudioPlayer {
    fn play(&self, path: &Path, allow_overlap: bool) -> Result<()>;
}

pub fn create_player() -> Result<Box<dyn AudioPlayer>> {
    Ok(Box::new(RodioAudioPlayer::new()?))
}

struct RodioAudioPlayer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    active_sinks: Mutex<Vec<Sink>>,
}

impl RodioAudioPlayer {
    fn new() -> Result<Self> {
        let (stream, stream_handle) =
            OutputStream::try_default().context("failed to open default audio output")?;
        Ok(Self {
            _stream: stream,
            stream_handle,
            active_sinks: Mutex::new(Vec::new()),
        })
    }
}

impl AudioPlayer for RodioAudioPlayer {
    fn play(&self, path: &Path, allow_overlap: bool) -> Result<()> {
        play_into_sinks(&self.active_sinks, path, allow_overlap, || {
            Sink::try_new(&self.stream_handle).context("failed to create audio sink")
        })
    }
}

// 先完成文件解码和新 Sink 创建，失败时保留已有播放。
fn play_into_sinks(
    active_sinks: &Mutex<Vec<Sink>>,
    path: &Path,
    allow_overlap: bool,
    create_sink: impl FnOnce() -> Result<Sink>,
) -> Result<()> {
    if !path.exists() {
        bail!("audio file does not exist");
    }
    if !path.is_file() {
        bail!("audio path is not a file");
    }
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let source = Decoder::new(BufReader::new(file))
        .with_context(|| format!("failed to decode {}", path.display()))?;
    let sink = create_sink()?;
    let mut sinks = active_sinks
        .lock()
        .map_err(|_| anyhow!("active audio sinks lock poisoned"))?;
    sinks.retain(|old| !old.empty());
    if !allow_overlap {
        for old in sinks.drain(..) {
            old.stop();
        }
    }
    sink.append(source);
    sinks.push(sink);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_wave(path: &Path) {
        // 一秒非零 PCM，直接读取 Sink 输出即可验证混音生命周期，无需声卡。
        let data_size = 44_100u32 * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&44_100u32.to_le_bytes());
        bytes.extend_from_slice(&88_200u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        for _ in 0..44_100 {
            bytes.extend_from_slice(&8192i16.to_le_bytes());
        }
        std::fs::write(path, bytes).unwrap();
    }

    #[test]
    fn repeated_audio_overlaps_and_exclusive_playback_stops_all_previous_sinks() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.wav");
        write_wave(&path);
        let sinks = Mutex::new(Vec::new());
        let mut outputs = Vec::new();
        for _ in 0..3 {
            play_into_sinks(&sinks, &path, true, || {
                let (sink, output) = Sink::new_idle();
                outputs.push(output);
                Ok(sink)
            })
            .unwrap();
        }
        assert_eq!(sinks.lock().unwrap().len(), 3);
        for output in &mut outputs {
            assert!(output.next().unwrap() > 0.0);
        }
        let mut latest_output = None;
        play_into_sinks(&sinks, &path, false, || {
            let (sink, output) = Sink::new_idle();
            latest_output = Some(output);
            Ok(sink)
        })
        .unwrap();
        assert_eq!(sinks.lock().unwrap().len(), 1);
        for output in &mut outputs {
            // rodio 最迟在下次周期检查（5ms）停止，Sink 丢弃后队列结束。
            for _ in 0..512 {
                output.next();
            }
            assert_eq!(output.next(), None);
        }
        assert!(latest_output.as_mut().unwrap().next().unwrap() > 0.0);
    }

    #[test]
    fn completed_sinks_are_removed_on_next_overlapping_playback() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.wav");
        write_wave(&path);
        let sinks = Mutex::new(Vec::new());
        let mut output = None;
        play_into_sinks(&sinks, &path, true, || {
            let (sink, queue) = Sink::new_idle();
            output = Some(queue);
            Ok(sink)
        })
        .unwrap();
        for _ in 0..45_000 {
            output.as_mut().unwrap().next();
        }
        assert!(sinks.lock().unwrap()[0].empty());
        let (next, _output) = Sink::new_idle();
        play_into_sinks(&sinks, &path, true, || Ok(next)).unwrap();
        assert_eq!(sinks.lock().unwrap().len(), 1);
        assert!(!sinks.lock().unwrap()[0].empty());
    }

    #[test]
    fn playback_errors_preserve_current_audio() {
        let dir = tempdir().unwrap();
        let valid = dir.path().join("sample.wav");
        let invalid = dir.path().join("invalid.wav");
        write_wave(&valid);
        std::fs::write(&invalid, b"invalid audio").unwrap();
        let sinks = Mutex::new(Vec::new());
        let (sink, mut output) = Sink::new_idle();
        play_into_sinks(&sinks, &valid, false, || Ok(sink)).unwrap();
        for path in [
            dir.path().join("missing.wav"),
            dir.path().to_path_buf(),
            invalid,
            valid,
        ] {
            assert!(
                play_into_sinks(&sinks, &path, false, || {
                    anyhow::bail!("simulated sink creation failure")
                })
                .is_err()
            );
            assert_eq!(sinks.lock().unwrap().len(), 1);
            assert!(output.next().unwrap() > 0.0);
        }
    }
}
