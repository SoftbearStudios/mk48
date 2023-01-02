// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::Mk48Route;
use common::entity::{EntityData, EntityKind, EntityType};
use std::collections::HashSet;
use yew::{function_component, html, Html};
use yew_frontend::component::discord_icon::DiscordIcon;
use yew_frontend::component::link::Link;
use yew_frontend::component::route_link::RouteLink;
use yew_frontend::dialog::dialog::Dialog;
use yew_frontend::frontend::{use_game_id, use_outbound_enabled};
use yew_frontend::translation::{use_translation, Translation};
use yew_frontend::{Route, CONTACT_EMAIL};

#[function_component(AboutDialog)]
pub fn about_dialog() -> Html {
    let t = use_translation();
    let game_id = use_game_id();
    let game_name = game_id.name();
    let outbound_enabled = use_outbound_enabled();
    let boat_type_count = EntityType::iter()
        .filter(|t| t.data().kind == EntityKind::Boat)
        .count();
    let weapon_sub_kind_count = EntityType::iter()
        .filter_map(|t| (t.data().kind == EntityKind::Weapon).then(|| t.data().sub_kind))
        .collect::<HashSet<_>>()
        .len();

    html! {
        <Dialog title={t.about_title(game_id)}>

            <h2>{"Description"}</h2>

            <p>
                {format!("{} is an online multiplayer ship combat game created by Softbear Studios. ", game_name)}
                {"The goal is to level up your ship by collecting crates and sinking other ships. There are "}
                <RouteLink<Mk48Route> route={Mk48Route::Ships}>{format!("{} boats", boat_type_count)}</RouteLink<Mk48Route>>
                {format!(" and {} weapon types to chose from, spread over ", weapon_sub_kind_count)}
                <RouteLink<Mk48Route> route={Mk48Route::Levels}>{format!("{} progressively more powerful levels", EntityData::MAX_BOAT_LEVEL)}</RouteLink<Mk48Route>>
                {"."}
            </p>

            <p>
                {"To learn more about the game, visit the "}
                <RouteLink<Mk48Route> route={Mk48Route::Help}>{"help page"}</RouteLink<Mk48Route>>
                {". You can also view the "}
                <RouteLink<Mk48Route> route={Mk48Route::Changelog}>{"changelog"}</RouteLink<Mk48Route>>
                {" to see what changed recently."}
            </p>

            <h2>{"Technical details"}</h2>

            <p>{"The game's source code and assets are "}<Link href="https://github.com/SoftbearStudios/mk48">{"open source"}</Link>{"."}</p>
            <ul>
                <li>
                    {"The "}
                    <Link href="https://github.com/SoftbearStudios/mk48/tree/main/client">{"client"}</Link>
                    {" and "}
                    <Link href="https://github.com/SoftbearStudios/mk48/tree/main/server">{"server"}</Link>
                    {" are written in "}
                    <Link href="https://www.rust-lang.org/">{"Rust"}</Link>
                    {" and rely on "}
                    <RouteLink<Route> route={Route::Licensing}>{"open-source software"}</RouteLink<Route>>
                    {"."}
                </li>
                <li>
                    {"The textures were modeled and rendered in "}
                    <Link href="https://www.blender.org/">{"Blender"}</Link>
                    {", except for the "}
                    <Link href="https://opengameart.org/content/simple-seamless-tiles-of-dirt-and-sand-sand2png">{"sand"}</Link>
                    {", "}
                    <Link href="https://opengameart.org/content/grass-textureseamless-2d">{"grass"}</Link>
                    {", and "}
                    <Link href="https://opengameart.org/content/seamless-snow-texture-0">{"snow"}</Link>
                    {"."}
                </li>
                <li>
                    {"The title-screen logo was designed by "}
                    <Link href="https://www.fiverr.com/skydesigner">{"skydesigner"}</Link>
                    {"."}
                </li>
                <li>
                    {"The sounds are from "}
                    <Link href="https://github.com/SoftbearStudios/mk48/blob/main/assets/sounds/README.md">{"these sources."}</Link>
                </li>
                <li>
                    {"IP Geolocation by "}
                    <Link href="https://db-ip.com">{"DB-IP"}</Link>
                    {"."}
                </li>
            </ul>

            if outbound_enabled {
                <h2>{"Contact Us"}</h2>
                <p>
                    {"If you have any feedback to share, business inquiries, or any other concern, please contact us on "}
                    <DiscordIcon size={"1.5rem"}/>
                    {" or by email at "}
                    <a href={format!("mailto:{}", CONTACT_EMAIL)}>{CONTACT_EMAIL}</a>
                    {"."}
                </p>
            }
        </Dialog>
    }
}
