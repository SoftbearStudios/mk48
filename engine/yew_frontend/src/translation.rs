use crate::frontend::Ctw;
use core_protocol::id::LanguageId;

/// Only works in function component.
pub fn use_translation() -> Box<dyn Translation> {
    let language_id = yew::use_context::<Ctw>().unwrap().language_id;

    // Doesn't allocate, since translations are zero-sized types.
    match language_id {
        LanguageId::Bork => Box::new(Bork),
        _ => Box::new(English),
    }
}

/// Alias of [`use_translation`] to be more concise.
pub fn t() -> Box<dyn Translation> {
    use_translation()
}

/// Declare static translations.
macro_rules! s {
    ($name: ident) => {
        fn $name(&self) -> &'static str;
    };
    ($name: ident, $value: literal) => {
        fn $name(&self) -> &'static str {
            $value
        }
    };
}

pub trait Translation {
    // The name of the language.
    s!(label);

    // Leaderboard screen.
    s!(panel_leaderboard_label, "Leaderboard");
    s!(panel_leaderboard_all, "All-time Leaderboard");
    s!(panel_leaderboard_day, "Daily Leaderboard");
    s!(panel_leaderboard_week, "Weekly Leaderboard");

    // Splash screen.
    s!(splash_screen_play_button, "Play");
    s!(splash_screen_alias_placeholder, "Nickname");

    // Score.
    s!(point, "point");
    s!(points, "points");
    fn score(&self, score: u32) -> String {
        // Good enough for simple plural vs. singular dichotomy, but can be overridden if needed.
        let suffix = match score {
            1 => self.point(),
            _ => self.points(),
        };
        format!("{} {}", score, suffix)
    }
}

pub struct English;

impl Translation for English {
    s!(label, "English");

    // Defaults are already in English.
}

pub struct Bork;

impl Translation for Bork {
    s!(label, "Bork");
    s!(panel_leaderboard_label, "Bork");
    s!(panel_leaderboard_all, "All-time Bork");
    s!(panel_leaderboard_day, "Daily Bork");
    s!(panel_leaderboard_week, "Weekly Bork");
    s!(splash_screen_alias_placeholder, "Bork");
    s!(point, "bork");
    s!(points, "borks");
}
