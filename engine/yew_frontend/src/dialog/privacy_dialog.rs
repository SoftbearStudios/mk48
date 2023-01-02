// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::dialog::dialog::Dialog;
use crate::frontend::{use_game_id, use_outbound_enabled};
use crate::translation::{use_translation, Translation};
use yew::{function_component, html, Html};

#[function_component(PrivacyDialog)]
pub fn privacy_dialog() -> Html {
    let t = use_translation();
    let game_id = use_game_id();
    let outbound_enabled = use_outbound_enabled();

    html! {
        <Dialog title={t.privacy_title(game_id)}>
            {privacy(outbound_enabled, crate::CONTACT_EMAIL)}
        </Dialog>
    }
}

pub fn privacy(outbound_enabled: bool, email: &str) -> Html {
    html! {
        <>
            <h2>{"Introduction"}</h2>

            <p>{"We collect information to provide, measure, and improve services
                for all our users."}</p>

            <p>{"Generally speaking, the amount of personal information we collect is minimal, and you can opt out of providing most of it."}</p>

            <h2>{"Information We Collect"}</h2>

            <table>
                <thead>
                    <tr>
                        <th>{"Information"}</th>
                        <th>{"Collection method"}</th>
                        <th>{"Suggested opt-out method"}</th>
                        <th>{"Primary purpose"}</th>
                        <th>{"Storage duration"}</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>{"Nickname"}</td>
                        <td>{"Splash screen"}</td>
                        <td>{"Leave blank"}</td>
                        <td>{"Differentiate between players"}</td>
                        <td>{"Forever, assuming score is leaderboard-worthy"}</td>
                    </tr>
                    <tr>
                        <td>{"Team name"}</td>
                        <td>{"Team panel"}</td>
                        <td>{"Don't make team"}</td>
                        <td>{"Differentiate between teams"}</td>
                        <td>{"As long as the team exists"}</td>
                    </tr>
                    <tr>
                        <td>{"Chat messages"}</td>
                        <td>{"Chat panel"}</td>
                        <td>{"Don't send any"}</td>
                        <td>{"Allow and moderate player communication"}</td>
                        <td>{"Forever, until manually deleted"}</td>
                    </tr>
                    <tr>
                        <td>{"IP address"}</td>
                        <td>{"Game server"}</td>
                        <td>{"Use a VPN"}</td>
                        <td>{"Security"}</td>
                        <td>{"While you play, unless potential abuse detected"}</td>
                    </tr>
                    <tr>
                        <td>{"User agent, referrer"}</td>
                        <td>{"Game server"}</td>
                        <td>{"Use a browser extension to hide"}</td>
                        <td>{"Aggregate statistics"}</td>
                        <td>{"Forever"}</td>
                    </tr>
                    <tr>
                        <td>{"How long you play, FPS"}</td>
                        <td>{"Game server"}</td>
                        <td>{"N/A"}</td>
                        <td>{"Aggregate statistics"}</td>
                        <td>{"Forever"}</td>
                    </tr>
                </tbody>
            </table>

            <h2>{"Use of Cookies"}</h2>

            <p>{r#"In order to ensure the continuity and consistency of your experience, and provide for internal operations, we store a persistent session identifier in your browser's local storage. You can reset it at any time, by using your browser's "clear site data" option. We do not use this information for advertising purposes."#}</p>

            <p>{"Settings, such as which language and volume level you select, are also stored in your browser's local storage but we don't collect them."}</p>

            <h2>{"Changes"}</h2>

            <p>{"We reserve the right to alter these privacy policies at any time, without notice."}</p>

            if outbound_enabled {
                <h2>{"Contact Us"}</h2>

                <p>{"If you have any concern, such as a desire to remove your nickname or your child's nickname from the
                    leaderboard, please contact us by email at "}<a href={format!("mailto:{email}")}>{email}</a>{"."}</p>
            }
        </>
    }
}
