use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rodio::{OutputStream, Sink, Source};
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

const SAMPLE_RATE: i32 = 44100;
const BLOCK_SIZE: usize = 256;

pub enum AudioCommand {
    NoteOn(u8),
    NoteOff(u8),
    StopAll,
    Shutdown,
}

pub struct AudioEngine {
    pub cmd_tx: Sender<AudioCommand>,
}

impl AudioEngine {
    pub fn start(sf_path: &Path) -> Result<Self, String> {
        let sound_font = {
            let mut file = BufReader::new(
                File::open(sf_path).map_err(|e| format!("Cannot open soundfont: {}", e))?,
            );
            Arc::new(
                SoundFont::new(&mut file)
                    .map_err(|e| format!("Cannot parse soundfont: {}", e))?,
            )
        };

        let settings = SynthesizerSettings::new(SAMPLE_RATE);
        let synth = Synthesizer::new(&sound_font, &settings)
            .map_err(|e| format!("Cannot create synthesizer: {}", e))?;
        let synth = Arc::new(Mutex::new(synth));

        let (cmd_tx, cmd_rx) = mpsc::channel::<AudioCommand>();

        thread::spawn(move || {
            if let Err(e) = Self::run_audio_thread(synth, cmd_rx) {
                eprintln!("Audio thread error: {}", e);
            }
        });

        Ok(Self { cmd_tx })
    }

    fn run_audio_thread(
        synth: Arc<Mutex<Synthesizer>>,
        cmd_rx: mpsc::Receiver<AudioCommand>,
    ) -> Result<(), String> {
        let (stream, stream_handle) =
            OutputStream::try_default().map_err(|e| format!("Cannot open audio: {}", e))?;

        let source = SynthSource::new(synth.clone());
        let sink =
            Sink::try_new(&stream_handle).map_err(|e| format!("Cannot create sink: {}", e))?;
        sink.append(source);

        loop {
            match cmd_rx.recv() {
                Ok(AudioCommand::NoteOn(key)) => {
                    if let Ok(mut s) = synth.lock() {
                        s.note_on(0, i32::from(key), 100);
                    }
                }
                Ok(AudioCommand::NoteOff(key)) => {
                    if let Ok(mut s) = synth.lock() {
                        s.note_off(0, i32::from(key));
                    }
                }
                Ok(AudioCommand::StopAll) => {
                    if let Ok(mut s) = synth.lock() {
                        for k in 0..128u8 {
                            s.note_off(0, i32::from(k));
                        }
                    }
                }
                Ok(AudioCommand::Shutdown) | Err(_) => break,
            }
        }

        drop(sink);
        drop(stream);
        Ok(())
    }
}

struct SynthSource {
    synth: Arc<Mutex<Synthesizer>>,
    left: Vec<f32>,
    right: Vec<f32>,
    buffer: Vec<f32>,
    pos: usize,
}

impl SynthSource {
    fn new(synth: Arc<Mutex<Synthesizer>>) -> Self {
        Self {
            synth,
            left: vec![0.0; BLOCK_SIZE],
            right: vec![0.0; BLOCK_SIZE],
            buffer: Vec::with_capacity(BLOCK_SIZE * 2),
            pos: BLOCK_SIZE * 2,
        }
    }

    fn refill(&mut self) {
        if let Ok(mut s) = self.synth.lock() {
            s.render(&mut self.left, &mut self.right);
        }
        self.buffer.clear();
        for (l, r) in self.left.iter().zip(self.right.iter()) {
            self.buffer.push(*l);
            self.buffer.push(*r);
        }
        self.pos = 0;
    }
}

impl Iterator for SynthSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.buffer.len() {
            self.refill();
        }
        let sample = self.buffer[self.pos];
        self.pos += 1;
        Some(sample)
    }
}

impl Source for SynthSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE as u32
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub fn default_soundfont_path(resource_dir: Option<&Path>) -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = resource_dir {
        candidates.push(dir.join("soundfont.sf2"));
        candidates.push(dir.join("resources").join("soundfont.sf2"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("soundfont.sf2"));
            candidates.push(dir.join("resources").join("soundfont.sf2"));
        }
    }
    candidates.push(PathBuf::from("soundfont.sf2"));
    candidates.push(PathBuf::from("../soundfont.sf2"));
    candidates.push(PathBuf::from("resources/soundfont.sf2"));
    candidates.push(PathBuf::from("../resources/soundfont.sf2"));
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    PathBuf::from("soundfont.sf2")
}

pub const fn midi_key(string: u8, fret: u8) -> u8 {
    let base = match string {
        1 => 64,
        2 => 59,
        3 => 55,
        4 => 50,
        5 => 45,
        6 => 40,
        _ => 60,
    };
    base + fret
}
