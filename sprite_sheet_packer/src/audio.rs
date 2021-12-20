// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use lazy_static::lazy_static;
use rayon::prelude::*;
use sprite_sheet::{AudioSprite, AudioSpriteSheet};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::iter;
use std::process::{Command, Stdio};

struct Sound {
    name: &'static str,
    /// Source file relative to directory.
    source: &'static str,

    // Author to credit.
    author: Option<&'static str>,
    // Url to credit.
    url: Option<&'static str>,

    /// Trim start seconds.
    start: Option<f32>,
    /// Trim end seconds.
    end: Option<f32>,

    /// Adjust volume (negative decreases, positive increases)
    volume: f32,
    /// Adjust pitch (negative decreases, positive increases)
    pitch: f32,
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
            volume: 0.0,
            pitch: 0.0,
        }
    }
}

lazy_static! {
    static ref SOUNDS: Vec<Sound> = vec![
        Sound {
            name: "achievement",
            source: "timbeek.com/Mk48.io OST - Epic Moments1_Short2.mp3",
            author: Some("Tim Beek"),
            volume: -3.0,
            ..Sound::default()
        },
        Sound {
            name: "alarm_slow",
            source: "freesound.org/165504__ryanconway__missile-lock-detected.mp3",
            author: Some("Ryan Conway"),
            url: Some("https://freesound.org/people/ryanconway/sounds/165504/"),
            end: Some(1.243),
            volume: -2.0,
            ..Sound::default()
        },
        Sound {
            name: "alarm_fast",
            source: "freesound.org/189327__alxy__missile-lock-on-sound.mp3",
            author: Some("Alxy"),
            url: Some("https://freesound.org/people/Alxy/sounds/189327/"),
            end: Some(0.641),
            volume: -2.5,
            ..Sound::default()
        },
        Sound {
            name: "collect",
            source: "freesound.org/512216__saviraz__coins.mp3",
            author: Some("Saviraz"),
            url: Some("https://freesound.org/people/Saviraz/sounds/512216/"),
            start: Some(0.065),
            end: Some(0.267),
            volume: -1.0,
            ..Sound::default()
        },
        Sound {
            name: "damage",
            source: "freesound.org/321485__dslrguide__rough-metal-scrape-textured.wav",
            author: Some("DSLR Guide"),
            url: Some("https://freesound.org/people/dslrguide/sounds/321485/"),
            volume: -3.0,
            pitch: -0.5,
            ..Sound::default()
        },
        Sound {
            name: "dive",
            source: "freesound.org/480002__craigsmith__r18-31-old-car-ahooga-horn.wav",
            author: Some("Craig Smith"),
            url: Some("https://freesound.org/people/craigsmith/sounds/480002/"),
            start: Some(2.85),
            end: Some(4.75),
            volume: -2.5,
            ..Sound::default()
        },
        Sound {
            name: "dodge",
            source: "timbeek.com/Mk48.io OST - Epic Moments2.mp3",
            author: Some("Tim Beek"),
            volume: -4.0,
            ..Sound::default()
        },
        Sound {
            name: "explosion_short",
            source: "freesound.org/514647__david2317__03-gran-explosion.wav",
            author: Some("David2317"),
            url: Some("https://freesound.org/people/David2317/sounds/514647/"),
            start: Some(2.471),
            volume: -4.0,
            ..Sound::default()
        },
        Sound {
            name: "explosion_long",
            source: "freesound.org/235968__tommccann__explosion-01.wav",
            author: Some("Tom McCann"),
            url: Some("https://freesound.org/people/tommccann/sounds/235968/"),
            start: Some(0.317),
            end: Some(6.0),
            volume: -5.0,
            ..Sound::default()
        },
        Sound {
            name: "aircraft",
            source: "freesound.org/513397__shelbyshark__helicopter-flying-overhead.wav",
            author: Some("Shelby Shark"),
            url: Some("https://freesound.org/people/shelbyshark/sounds/513397/"),
            start: Some(1.0),
            end: Some(2.0),
            volume: -1.0,
            ..Sound::default()
        },
        Sound {
            name: "jet",
            source: "freesound.org/105222__vax6131__hawk-jets.wav",
            author: Some("Vax6131"),
            url: Some("https://freesound.org/people/Vax6131/sounds/105222/"),
            start: Some(53.0),
            end: Some(54.0),
            volume: -3.5,
            ..Sound::default()
        },
        Sound {
            name: "horn",
            source: "freesound.org/532339__reznik-krkovicka__horn-mild.mp3",
            author: Some("Reznik Krkovicka"),
            url: Some("https://freesound.org/people/reznik_Krkovicka/sounds/532339/"),
            start: Some(1.328),
            end: Some(5.588),
            volume: -1.0,
            ..Sound::default()
        },
        Sound {
            name: "impact",
            source: "freesound.org/4366__qubodup__military-sounds/67468__qubodup__howitzer-gun-impacts-1.flac",
            author: Some("qubodup"),
            url: Some("https://freesound.org/people/qubodup/sounds/67468/"),
            volume: -3.0,
            pitch: -1.0,
            ..Sound::default()
        },
        Sound {
            name: "intense",
            source: "timbeek.com/Mk48.io OST - Epic Moments4_B_Short1.mp3",
            author: Some("Tim Beek"),
            volume: -4.0,
            end: Some(9.0),
            ..Sound::default()
        },
        Sound {
            name: "ocean",
            source: "freesound.org/372181__amholma__ocean-noise-surf.wav",
            author: Some("amholma"),
            url: Some("https://freesound.org/people/amholma/sounds/372181/"),
            start: Some(1.0),
            end: Some(6.0),
            volume: -3.0,
            ..Sound::default()
        },
        Sound {
            name: "rocket",
            source: "freesound.org/4366__qubodup__military-sounds/67541__qubodup__bgm-71-tow-missile-launch-1.flac",
            author: Some("qubodup"),
            url: Some("https://freesound.org/people/qubodup/sounds/67541/"),
            volume: -3.5,
            ..Sound::default()
        },
        Sound {
            name: "shell",
            source: "freesound.org/4366__qubodup__military-sounds/162365__qubodup__navy-battleship-soundscape-turret-gunshots-mechanical-engine-humm-radio-chatter-officer-command-voices.flac",
            author: Some("qubodup"),
            url: Some("https://freesound.org/people/qubodup/sounds/162365/"),
            start: Some(0.057),
            end: Some(2.0),
            volume: -3.5,
            ..Sound::default()
        },
        Sound {
            name: "sonar0",
            source: "freesound.org/90340__digit-al__sonar.wav",
            author: Some("Digit-al"),
            url: Some("https://freesound.org/people/digit-al/sounds/90340/"),
            end: Some(5.0),
            volume: -2.5,
            ..Sound::default()
        },
        Sound {
            name: "sonar1",
            source: "freesound.org/493162__breviceps__submarine-sonar.wav",
            author: Some("Breviceps"),
            url: Some("https://freesound.org/people/Breviceps/sounds/493162/"),
            start: Some(0.184),
            end: Some(1.964),
            volume: -3.0,
            ..Sound::default()
        },
        Sound {
            name: "sonar2",
            source: "freesound.org/38702__elanhickler__archi-sonar-03.wav",
            author: Some("Elan Hickler"),
            url: Some("https://freesound.org/people/ElanHickler/sounds/38702/"),
            end: Some(2.5),
            volume: -1.0,
            ..Sound::default()
        },
        Sound {
            name: "sonar3",
            source: "freesound.org/70299__kizilsungur__sonar.wav",
            author: Some("KIZILSUNGUR"),
            url: Some("https://freesound.org/people/KIZILSUNGUR/sounds/70299/"),
            volume: -3.0,
            ..Sound::default()
        },
        Sound {
            name: "surface",
            source: "freesound.org/416079__davidlay1__shaving-cream-can-release.wav",
            author: Some("David Lay"),
            url: Some("https://freesound.org/people/davidlay1/sounds/416079/"),
            end: Some(2.0),
            volume: -3.0,
            ..Sound::default()
        },
        Sound {
            name: "splash",
            source: "freesound.org/398032__swordofkings128__splash.wav",
            author: Some("swordofkings128"),
            url: Some("https://freesound.org/people/swordofkings128/sounds/398032/"),
            volume: -3.5,
            ..Sound::default()
        },
        Sound {
            name: "torpedo_launch",
            source: "freesound.org/367125__jofae__air-hiss.mp3",
            author: Some("Jofae"),
            url: Some("https://freesound.org/people/Jofae/sounds/367125/"),
            volume: -4.0,
            ..Sound::default()
        },
        Sound {
            name: "upgrade",
            source: "opengameart.org/Rise05.aif",
            author: Some("wobbleboxx"),
            url: Some("https://opengameart.org/content/level-up-power-up-coin-get-13-sounds"),
            start: Some(0.809),
            end: Some(1.4),
            volume: -2.0,
            ..Sound::default()
        }
    ];
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

pub(crate) fn pack_audio_sprite_sheet(
    channels: usize,
    sample_rate: usize,
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

    let raws: BTreeMap<_, _> = SOUNDS
        .par_iter()
        .map(|sound| {
            let path = format!("../assets/sounds/{}", sound.source);
            let raw = read_raw_audio(&path, sound, channels, sample_rate);
            (sound, raw)
        })
        .collect();

    let sprites: HashMap<_, _> = raws
        .into_iter()
        .map(|(sound, raw)| {
            const SIZEOF_FLOAT16: usize = 2;

            let sprite = AudioSprite {
                start: audio.len() as f32 / (channels * sample_rate * SIZEOF_FLOAT16) as f32,
                duration: raw.len() as f32 / (channels * sample_rate * SIZEOF_FLOAT16) as f32,
            };

            audio.extend(raw.into_iter());
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

    command.arg("-i").arg(src).arg("-vn");

    if let Some(start) = sound.start {
        command.arg("-ss").arg(format!("{}", start));
    }

    if let Some(end) = sound.end {
        command.arg("-to").arg(format!("{}", end));
    }

    let output = command
        // The following two args should be removed, but that would require
        // adjusting pitch values.
        .arg("-ab")
        .arg("128k")
        .arg("-ac")
        .arg(&format!("{}", channels))
        .arg("-af")
        .arg(format!(
            "volume={:.02},asetrate={},atempo={:.02},aresample={}",
            volume_factor,
            (sample_rate as f32 * pitch_factor) as usize,
            1.0 / pitch_factor,
            sample_rate,
        ))
        .arg("-f")
        .arg("s16le")
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
