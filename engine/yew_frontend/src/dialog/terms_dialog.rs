// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::dialog::dialog::Dialog;
use crate::frontend::use_game_id;
use crate::translation::{use_translation, Translation};
use yew::{function_component, html, Html};

#[function_component(TermsDialog)]
pub fn terms_dialog() -> Html {
    let t = use_translation();
    let game_id = use_game_id();
    let game_name = game_id.name();

    html! {
        <Dialog title={t.terms_title(game_id)}>
            {terms(game_name, "game")}
        </Dialog>
    }
}

/// noun may be "game" or "games"
pub fn terms(site: &str, noun: &str) -> Html {
    html! {
        <>
            <p>{format!("The following terms govern your use of the {site} website and {noun}.")}</p>

            <h2>{"Allowed Activities"}</h2>

            <p>{"You are granted a non-exclusive license to do the following for either commercial or non-commercial purposes, provided you adhere to all the terms."}</p>

            <ol>
                <li>{format!("Playing the {noun}.")}</li>
                <li>{format!("Recording and/or publishing screenshots, videos, or other content involving the {noun}.")}</li>
                <li>{format!("Using the individual game textures or adaptations of them in connection with content involving the {noun} (such as in a video thumbnail).")}</li>
                <li>{format!("Embedding the {noun} on another website.")}</li>
                <li>{format!("Linking to the {noun} on another website.")}</li>
            </ol>

            <h2>{"Prohibited Activities"}</h2>

            <p>{"You are prohibited from engaging in any of the following activities."}</p>

            <ol>
                <li>{"Disclosing any personal information (full name, contact information, etc.), by any means, if you are under 13 years of age."}</li>
                <li>{format!("Using inappropriate or offensive language for a nickname, team name, in game chat, or in any comments section corresponding to the {noun}.")}</li>
                <li>{"Placing a higher burden on the game's server(s) than two instances of the official game client would (opening more than two connections at a time, sending messages at a higher frequency, or otherwise compromising the integrity of the server(s))."}</li>
                <li>{"Violating any applicable law, or violating the rights or privacy of others."}</li>
                <li>{format!("Claiming to offer a downloadable version of the {noun} (as one does not exist).")}</li>
                <li>{"Attempting to bypass our policy preventing leaderboard score attempts on non-public servers."}</li>
            </ol>

            <h2>{"Liability"}</h2>

            <p><b>{format!(r#"{site} is provided "AS IS". The developers make no warranties, express
            or implied, and hereby disclaim all implied warranties, including any warranty of
            merchantability and warranty of fitness for a particular purpose."#)}</b></p>

            <h2>{"Trademark"}</h2>

            <p>
                <b>{"Softbear"}</b>
                {" is a trademark of Softbear, Inc."}
            </p>

            <h2>{"Changes"}</h2>

            <p>{"We reserve the right to alter these Terms of Service at any time, without notice."}</p>
        </>
    }
}
