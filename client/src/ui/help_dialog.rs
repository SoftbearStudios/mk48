// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::ui::Mk48Route;
use yew::{function_component, html, Html};
use yew_frontend::component::route_link::RouteLink;
use yew_frontend::dialog::dialog::Dialog;
use yew_frontend::frontend::use_game_id;
use yew_frontend::translation::{use_translation, Translation};

#[function_component(HelpDialog)]
pub fn help_dialog() -> Html {
    let t = use_translation();
    let game_id = use_game_id();
    let game_name = game_id.name();
    html! {
        <Dialog title={t.help_title(game_id)}>

            <h2>{"Basics"}</h2>

            <p>
                {format!("{} is a multiplayer naval combat game. ", game_name)}
                {"You start off a small ship, and consume crates to increase your score (to level up your ship). "}
                {"While a small amount of crates spawn naturally, sinking other ships directly increases your score and spawns many more collectibles."}
            </p>

            <h2>{"Movement"}</h2>

            <p>{"There are four ways to move your ship:"}</p>

            <ol>
                <li><b>{"Left click and hold"}</b></li>
                <li><b>{"Right click"}</b>{" and optionally hold "}<i>{"(this is the recommended way to move)"}</i></li>
                <li><b>{"Tap and hold"}</b>{" (on touch screen)"}</li>
                <li>{"Use the "}<b>{"WASD"}</b>{" or "}<b>{"arrow"}</b>{" keys to move, and 'x' key to stop"}</li>
            </ol>

            <p>
                {"Once your ship is moving in a direction, it will keep moving, so you can focus on using your weapons."}</p>

            <p>
                {"If using mouse controls, your ship will turn to point towards your mouse. "}
                {"You can control the speed of your ship by varying the distance between your mouse and your ship. "}
            </p>

            <h2>{"Ships"}</h2>

            <p>
                {"Just like in real life, there are many different types of ships, each with different advantages:"}
            </p>

            <ol>
                <li><b>{"Motor-torpedo boats"}</b>{" occupy the lower levels and generally carry
                multiple torpedoes and possibly guns."}</li>
                <li><b>{"Corvettes"}</b>{" and "}<b>{"Destroyers"}</b>{" are larger and carry more weapons, including
                slightly more powerful guns."}</li>
                <li><b>{"Cruisers"}</b>{" are a compromise between destroyers and battleships."}</li>
                <li><b>{"Battleships"}</b>{" and "}<b>{"Dreadnoughts"}</b>{" are very formidable ships, having extremely
                powerful main cannons. They may carry a minimal complement of aircraft for submarine defense."}</li>
                <li><b>{"Submarines"}</b>{" travel underwater, making them immune to certain
                types of weapons, but must surface to fire certain types of weapons."}</li>
                <li><b>{"Hovercraft"}</b>{" can travel on land and water."}</li>
                <li><b>{"Rams"}</b>{" are specially designed to ram other ships."}</li>
                <li><b>{"Dredgers"}</b>{" have the ability to modify the land. New land
                can be created by clicking in front of them, and old land can be destroyed
                by sailing over it."}</li>
                <li><b>{"Icebreakers"}</b>{" can plow through ice and snow without taking damage."}</li>
                <li><b>{"Minelayers"}</b>{" dispense magnetic mines that can help guard a small area."}</li>
                <li><b>{"Aircraft carriers"}</b>{" command a squadron of aircraft which follow your mouse cursor to attack enemy ships!"}</li>
            </ol>

            <p>
                {"Once you earn enough points, you can pick an "}<b>{"upgrade"}</b>{" from the top-middle of the screen. "}
                {"Be careful when upgrading, as becoming a larger ship may lead to crashing into land "}
                {"or reduced mobility if the water is too shallow."}
            </p>

            <p>
                {"Enemies are more likely to detect larger ships. However, a few ships have a property known as "}
                <b>{"stealth"}</b>
                {" to help you evade detection."}
            </p>

            <p>
                {"Here is a full list of ships: "}
                <RouteLink<Mk48Route> route={Mk48Route::Ships}>{"Mk48.io Ships"}</RouteLink<Mk48Route>>
                {"."}
            </p>

            <h2>{"Weapons"}</h2>

            <p>
                {"There are multiple different types of weapons available on various ships. "}
                {"In general, weapons are fired by clicking in the appropriate direction "}
                {"(although the Space or 'e' keys can also be used)."}
            </p>

            <ol>
                <li><b>{"Torpedoes"}</b>{" are powerful underwater weapons. Some torpedoes have the ability to track targets automatically, with sonar."}</li>

                <li><b>{"Missiles"}</b>{" are airborne and are faster, but less maneuverable than torpedoes."}</li>

                <li><b>{"Rockets"}</b>{" are like missiles but lack guidance."}</li>

                <li><b>{"Rocket torpedoes"}</b>{" deploy a torpedo when in the vicinity of an enemy submarine."}</li>

                <li><b>{"SAMs"}</b>{" (surface-to-air missiles) can shoot down aircraft and missiles."}</li>

                <li><b>{"Gun turrets"}</b>{" shoot very fast shells that do moderate damage."}</li>

                <li><b>{"Depth charges"}</b>{" are stationary weapons that can be deployed against pursuing ships or submerged submarines."}</li>

                <li><b>{"Mines"}</b>{" are like depth charges but last much longer and are more damaging."}</li>

                <li><b>{"Aircraft"}</b>{" fly towards your mouse cursor, and automatically deploy weapons of their own."}</li>

                <li><b>{"Depositor"}</b>{" creates new land. We'll let you figure out if this can be used as a weapon."}</li>
            </ol>

            <p>
                {"Once fired, all weapons take some time to "}
                <b>{"reload"}</b>
                {". Each type of weapon and each turret reload independently. "}
                {"Consuming crates speeds up reloading."}
            </p>

            <h2>{"Sensors"}</h2>

            <p>{"All ships have some combination of sensors to identify other ships and obstacles:"}</p>

            <ol>
                <li><b>{"Visual"}</b>{" tracks all targets, with a range that depends on conditions."}</li>
                <li><b>{"Radar"}</b>{" tracks targets above water."}</li>
                <li><b>{"Sonar"}</b>{" tracks underwater targets."}</li>
            </ol>

            <p>
                {"There are two modes for sensors, "}<b>{"active"}</b>{" and "}<b>{"passive"}</b>
                {". Passive mode listens for emissions (e.g. sound in the case of sonar) from other entities. "}
                {"Active mode emits a signal and resolves contacts based on the signals that bounce back. "}
                {"In general, active mode allows you to see more, but has the potential to give away your position. "}
                {"You can toggle between the modes with the 'z' key."}
            </p>

            <p>
                {"If a contact is on the border of your sensor range, it will appear as an arrow. "}
                {"In this case, you know something is there, but not what it is."}
            </p>

            <h2>{"Fleets"}</h2>

            <p>
                {"You can join other players to form a fleet, using the panel in the top left corner of the screen. "}
                {"As the creator of a fleet, you can accept or deny those who request to join it. "}
                {"Members of a fleet cannot hurt each other with weapons. "}
                {"Importantly, you cannot request to join a fleet until you are close enough to see one of its members, and the fleet has slots remaining."}
            </p>

            <h2>{"The Arctic"}</h2>

            <p>
                {"Experienced players may attempt to explore the Arctic biome, located far North. "}
                {"It is important to understand that Arctic has three separate types of terrain, snow, ice sheet, and cracked ice sheet. "}
                {"Snow, which appears white and smooth, will damage any ship upon contact. "}
                {"The two types of ice sheet will slow and damage most ships, the difference being that non-icebreakers cannot destroy non-cracked ice sheet. "}
                {"Submarines can travel beneath both types of ice sheet."}
            </p>

        </Dialog>
    }
}
