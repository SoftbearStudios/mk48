// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use sprite_sheet::{AudioSprite, AudioSpriteSheet};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::iter;
use std::process::{Command, Stdio};

/// A single audio file to add to pass to `pack_audio_sprite_sheet`.
pub struct Sound {
    /// Name of the sound such as "upgrade".
    pub name: &'static str,
    /// Source file relative to directory.
    pub source: &'static str,
    /// Author to credit.
    pub author: Option<&'static str>,
    /// Url to credit.
    pub url: Option<&'static str>,
    /// Trim start seconds.
    pub start: Option<f32>,
    /// Trim end seconds.
    pub end: Option<f32>,
    /// Whether the looping section starts.
    pub loop_start: Option<f32>,
    /// Adjust volume (negative decreases, positive increases)
    pub volume: f32,
    /// Adjust pitch (negative decreases, positive increases)
    pub pitch: f32,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            name: "",
            source: "",
            author: None,
            url: None,
            start: None,
            end: None,
            loop_start: None,
            volume: 0.0,
            pitch: 0.0,
        }
    }
}

impl PartialOrd for Sound {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(other.name)
    }
}

impl Ord for Sound {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(other.name)
    }
}

impl PartialEq for Sound {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Sound {}

/// Packs `sounds` into an [`AudioSpriteSheet`]. Requires ffmpeg to be installed.
/// TODO allow input_directory to end in a /.
pub fn pack_audio_sprite_sheet(
    sounds: Vec<Sound>,
    channels: usize,
    sample_rate: usize,
    input_directory: &str,
    output_audio: &str,
    output_data: &str,
    output_manifest: &str,
) {
    // The raw 16 bit, little endian floats of audio data.
    let mut audio = Vec::new();

    // Readme file.
    let mut manifest_string = String::new();
    let manifest = &mut manifest_string;

    writeln!(manifest, "# Sound Credits").unwrap();
    writeln!(manifest).unwrap();
    writeln!(
        manifest,
        "Sounds are either licensed under CC0/public domain or via transfer of Copyright"
    )
    .unwrap();
    writeln!(manifest).unwrap();
    writeln!(
        manifest,
        "**Warning: Sounds in this directory are unprocessed and potentially very loud!**"
    )
    .unwrap();
    writeln!(manifest).unwrap();

    let raws: BTreeMap<_, _> = sounds
        .into_iter()
        .map(|sound| {
            let path = format!("{}/{}", input_directory, sound.source);
            let raw = read_raw_audio(&path, &sound, channels, sample_rate);
            (sound, raw)
        })
        .collect();

    let sprites: HashMap<_, _> = raws
        .into_iter()
        .map(|(sound, raw)| {
            const SIZEOF_FLOAT16: usize = 2;

            let start = audio.len() as f32 / (channels * sample_rate * SIZEOF_FLOAT16) as f32;

            let sprite = AudioSprite {
                start,
                loop_start: sound.loop_start.map(|ls| start + ls),
                duration: raw.len() as f32 / (channels * sample_rate * SIZEOF_FLOAT16) as f32,
            };

            audio.extend(raw);
            // Gap of silence (half second).
            audio.extend(iter::repeat(0u8).take(channels * sample_rate * SIZEOF_FLOAT16 / 2));

            // Write manifest line.
            if let Some(url) = sound.url {
                write!(manifest, " - [{}]({})", sound.name, url).unwrap();
            } else {
                write!(manifest, " - {}", sound.name).unwrap();
            }
            if let Some(author) = sound.author {
                if author == "Tim Beek" {
                    write!(manifest, " by [Tim Beek](https://timbeek.com)").unwrap();
                } else {
                    write!(manifest, " by {}", author).unwrap();
                }
            }
            writeln!(manifest).unwrap();

            (String::from(sound.name), sprite)
        })
        .collect();

    let sprite_sheet = AudioSpriteSheet { sprites };

    let audio_path = format!("{}.mp3", output_audio);
    println!("Writing {}...", audio_path);
    write_raw_audio(audio, &audio_path, channels, sample_rate);

    let json = serde_json::to_string(&sprite_sheet).unwrap();
    let data_path = format!("{}.json", output_data);
    println!("Writing {}...", data_path);
    fs::write(&data_path, json).unwrap();

    let manifest_path = format!("{}.md", output_manifest);
    println!("Writing {}...", manifest_path);
    fs::write(&manifest_path, manifest_string).unwrap();
}

fn read_raw_audio(src: &str, sound: &Sound, channels: usize, sample_rate: usize) -> Vec<u8> {
    let volume_factor = 2f32.powf(sound.volume);
    let pitch_factor = 2f32.powf(sound.pitch);

    let mut command = Command::new("ffmpeg");

    // Seek in input file.
    if let Some(start) = sound.start {
        command.arg("-ss").arg(format!("{}", start));
    }

    if let Some(end) = sound.end {
        command.arg("-to").arg(format!("{}", end));
    }

    // Input file.
    command.arg("-i").arg(src);

    // No video.
    command.arg("-vn");

    // The following two args should be removed, but that would require
    // adjusting pitch values.
    command.arg("-ab").arg("128k");

    // Output this many channels.
    command.arg("-ac").arg(&format!("{}", channels));

    // Change volume, tempo, etc.
    command.arg("-af").arg(format!(
        "volume={:.02},asetrate={},atempo={:.02},aresample={}",
        volume_factor,
        (sample_rate as f32 * pitch_factor) as usize,
        1.0 / pitch_factor,
        sample_rate,
    ));

    // Output 16 bit, little endian floats.
    command.arg("-f").arg("s16le");

    let output = command
        .arg("pipe:")
        .stdout(Stdio::piped())
        .output()
        .expect("ffmpeg failed");

    output
        .status
        .exit_ok()
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr).into_owned()));

    output.stdout
}

fn write_raw_audio(src: Vec<u8>, dest: &str, channels: usize, sample_rate: usize) {
    let mut child = Command::new("ffmpeg")
        .arg("-y")
        .arg("-f")
        .arg("s16le")
        .arg("-ac")
        .arg(&format!("{}", channels))
        .arg("-ar")
        .arg(&format!("{}", sample_rate))
        .arg("-i")
        .arg("pipe:")
        .arg(dest)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("ffmpeg failed");

    let mut stdin = child.stdin.take().expect("failed to open stdin");
    std::thread::spawn(move || {
        stdin.write_all(&src).expect("failed to write to stdin");
    });

    let output = child.wait_with_output().expect("failed to read stdout");

    output
        .status
        .exit_ok()
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr).into_owned()));

    /*
    String::from_utf8_lossy(&output.stdout)
        .into_owned()
        .into_bytes()
     */
}
