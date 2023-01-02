// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use sprite_sheet_util::Sound;

pub(crate) fn sounds() -> Vec<Sound> {
    vec![
        Sound {
            name: "aa",
            source: "freesound.org/260939__picassoct__antiair.ogg",
            author: Some("PicassoCT"),
            url: Some("https://freesound.org/people/PicassoCT/sounds/260939/"),
            volume: -3.0,
            pitch: -0.5,
            start: Some(0.05),
            end: Some(0.63),
            ..Sound::default()
        },
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
            source: "freesound.org/131315__rickbuzzin__small-jet-flyover.wav",
            author: Some("rickbuzzin"),
            url: Some("https://freesound.org/people/rickbuzzin/sounds/131315/"),
            start: Some(11.5),
            end: Some(12.5),
            volume: -3.0,
            pitch: -0.25,
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
    ]
}
