use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result, bail};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

pub trait AudioPlayer {
    fn play(&self, path: &Path) -> Result<()>;
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
    fn play(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            bail!("audio file does not exist");
        }
        if !path.is_file() {
            bail!("audio path is not a file");
        }
        let file =
            File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
        let source = Decoder::new(BufReader::new(file))
            .with_context(|| format!("failed to decode {}", path.display()))?;
        let sink = Sink::try_new(&self.stream_handle).context("failed to create audio sink")?;
        sink.append(source);
        if let Ok(mut sinks) = self.active_sinks.lock() {
            // 同一时间只保留一条播放：先停止并丢弃之前的所有播放
            for old in sinks.drain(..) {
                old.stop();
            }
            sinks.push(sink);
        }
        Ok(())
    }
}
