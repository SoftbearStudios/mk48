// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use yew::{function_component, html, Html};
use yew_frontend::component::link::Link;
use yew_frontend::dialog::dialog::Dialog;
use yew_frontend::frontend::use_game_id;
use yew_frontend::translation::{use_translation, Translation};

#[function_component(ChangelogDialog)]
pub fn changelog_dialog() -> Html {
    let t = use_translation();
    let game_id = use_game_id();
    html! {
        <Dialog title={t.changelog_title(game_id)}>
            <p>{"Warning: This changelog may not always be fully up to date"}</p>
            {changelog_2022()}
            {changelog_2021_part_2()}
            {changelog_2021_part_1()}
        </Dialog>
    }
}

#[inline(never)]
fn changelog_2022() -> Html {
    html! {
        <>
            <h2>{"2022"}</h2>

            <h3>{"12/26/2022"}</h3>

            <ul>
                <li>{"Fix surface and active sensor sounds when using key bindings."}</li>
                <li>{"Fix accept button when team is full."}</li>
                <li>{"Fix weapon selector when weapon is consumed."}</li>
                <li>{"Save whether fleet/leaderboard are open between plays/refreshes."}</li>
                <li>{"Disable shadows by default on mobile devices until they can be fixed."}</li>
            </ul>

            <h3>{"12/24/2022"}</h3>

            <ul>
                <li>{"Add Skjold corvette and associated weapon."}</li>
                <li>{"Add directional shading."}</li>
                <li>{"Reduce score gain when spawn-killing."}</li>
                <li>{"Adjust missile guidance."}</li>
                <li>{"Add minimalist HUD (can switch back to old Circle HUD in settings)."}</li>
                <li>{"Fix aiming of Yasen's rocket torpedoes."}</li>
                <li>{"Reduce Visby's torpedo count to 4 for realism."}</li>
                <li>{"Add support for converting text (like :smile:) to emoji in chat."}</li>
                <li>{"Add more chat moderation features."}</li>
                <li>{"Add optional high-contrast setting."}</li>
                <li>{"Highlight mentions of your nickname in chat."}</li>
                <li>{"Mute sounds when the tab is minimized."}</li>
                <li>{"Revise help page and add an open-source software page linked to from the about page."}</li>
                <li>{"Begin accumulating per-ship data to inform re-balancing."}</li>
            </ul>

            <h3>{"7/10/2022"}</h3>

            <ul>
                <li>{"Add torpedoes to Dreadnought and Type 055 where realistic."}</li>
                <li>{"Fix several spawning issues, and try harder to avoid bad spawn locations."}</li>
                <li>{"Make maximum team size depend on number of players online. It now ranges from 4 to 8, instead of always being 6."}</li>
                <li>{"Oil barrels no longer regenerate health."}</li>
                <li>{"Oil rigs de-spawn slightly faster when no players are nearby."}</li>
                <li>{"Fix bug allowing rocket torpedoes to have guidance."}</li>
                <li>{"Add language picker to splash screen."}</li>
            </ul>

            <h3>{"6/4/2022"}</h3>

            <ul>
                <li>{"Improve spawning location algorithm."}</li>
                <li>{"Don't clear server preference on refresh or settings change."}</li>
                <li>{"Submarines can submerge slightly faster (less delay)."}</li>
                <li>{"Reset surfacing animation when respawning."}</li>
                <li>{"Improve player distribution among multiple servers."}</li>
                <li>{"Make bots lose more points when they are sunk, to encourage more low level bots."}</li>
            </ul>

            <h3>{"6/2/2022"}</h3>

            <ul>
                <li>{"Add Iowa battleship."}</li>
                <li>{"Add Tropics biome."}</li>
                <li>{"Icebreaker can destroy snow as well as ice."}</li>
                <li>{"Land repels boats instead of just damaging them."}</li>
                <li>{"Make reversing easier and more visual."}</li>
                <li>{"Homing torpedoes can make a U-turn."}</li>
                <li>{"Add invite, Discord, and GitHub links to splash screen."}</li>
                <li>{"Reduce sensor range of weapons like missiles and torpedoes."}</li>
                <li>{"Fix spawning bias towards border of Arctic biome."}</li>
                <li>{"Make submerging more visually obvious and delay it to avoid abuse."}</li>
                <li>{"Fix unwanted right-click menus."}</li>
                <li>{"Reduce the number of SAMs on Kolkata and Type 055."}</li>
                <li>{"Add aim indicators for non-turreted weapons."}</li>
                <li>{"Pointy-looking ships do more ramming damage."}</li>
                <li>{"Torpedoes out-smart players who try to out-turn them."}</li>
                <li>{"Avoid playing loud sounds when tab is re-opened."}</li>
                <li>{"Improve rocket-torpedo sensing of enemies."}</li>
                <li>{"Mines can now hit submerged subs."}</li>
                <li>{"Oil rigs de-spawn if unused for 15 minutes."}</li>
                <li>{"Surface ships can no longer fire weapons while submerged."}</li>
                <li>{"Fix compatibility issue with lacking Uint64Array support."}</li>
                <li>{"Underwater surface ships can't handle the pressure..."}</li>
                <li>{"Make zooming in and out more proportional."}</li>
                <li>{"Improve wake and smoke trails."}</li>
                <li>{"Make health-bar rounded."}</li>
                <li>{"Prevent closing weapons panel."}</li>
                <li>{"Fix some Dredger issues."}</li>
                <li>{"Make it much less likely to spawn near weapons."}</li>
            </ul>

            <h3>{"4/10/2022"}</h3>

            <ul>
                <li>{"Remove health regeneration when upgrading to below your full potential."}</li>
                <li>{"Fix some rare graphical glitches."}</li>
                <li>{"Fix daily/weekly leaderboard issue (pending database maintenance)."}</li>
                <li>{"Fix bug affecting incognito/private browsing mode."}</li>
                <li>{"Resolve server CPU usage which resulted in lag (part 2)."}</li>
                <li>{"Working towards multiple servers in same region (experimental)."}</li>
            </ul>

            <h3>{"3/12/2022"}</h3>

            <ul>
                <li>{"Fix throttle vibrating at low FPS."}</li>
                <li>{"Resolve server CPU usage which resulted in lag (part 1)."}</li>
                <li>{"Lower client-to-server bandwidth."}</li>
            </ul>

            <h3>{"3/6/2022"}</h3>

            <ul>
                <li>{"Different submarines now cavitate at different speeds/depths."}</li>
                <li>{"Fix mechanics related to leaving teams, such as destruction of mines."}</li>
                <li>{"Fix leaderboard and team GUI issue after switching servers."}</li>
                <li>{"Improve fire rate in the presence of high latency."}</li>
                <li>{"Discontinue Asia server due to lower player counts, until a better server-switching solution is implemented."}</li>
                <li>{"Fix score-retention bug reported by El Pepe and IcyBun."}</li>
                <li>{"Fix crash due to poor WebAudio support."}</li>
                <li>{"Attempt to fix bug that randomly kicks you back to the splash screen."}</li>
            </ul>

            <h3>{"2/27/2022"}</h3>

            <ul>
                <li>{"Reduce Kirov torpedo salvo size from 10 to 8, and surface-to-air missile salvo size from 8 to 6."}</li>
                <li>{"Oil rigs decay after 90 minutes."}</li>
                <li>{"Outside Arctic, upgraded oil rigs will decay to normal oil rigs."}</li>
                <li>{"Otherwise, decaying oil rigs are simply respawned elsewhere (according to the world size)."}</li>
                <li>{"Add admin moderation system to combat abusive behavior."}</li>
                <li>{"Add Asia server region (experimental)."}</li>
                <li>{"Reinstate Bork language."}</li>
            </ul>

            <h3>{"2/6/2022"}</h3>

            <ul>
                <li>{"Icebreakers earn points for destroying ice."}</li>
                <li>{"Make spawn location more random again."}</li>
                <li>{"Lower levels keep more points when respawning."}</li>
                <li>{"Submarines can travel and regenerate at full speed in the Arctic."}</li>
                <li>{"More HQ's spawn in arctic."}</li>
                <li>{"Fix Dredger placing land too close to the Arctic."}</li>
                <li>{"Help page now explains the basics of the Arctic."}</li>
            </ul>

            <h3>{"2/5/2022"}</h3>

            <ul>
                <li>{"Add Arctic biome, in which HQ's spawn."}</li>
                <li>{"Add Terry Fox icebreaker."}</li>
                <li>{"Change Seawolf armament in accordance with Discord vote."}</li>
                <li>{"Redesign user interface."}</li>
                <li>{"Increase size of world."}</li>
                <li>{"Emojis in friendly names display with proper color."}</li>
                <li>{"Don't show internal weapons in cinematic mode."}</li>
                <li>{"Change how many points you keep after being sunk."}</li>
                <li>{"More efficient terrain rendering (if animations setting is unchecked)."}</li>
            </ul>

            <h3>{"1/16/2022"}</h3>

            <ul>
                <li>{"Depth charges now explode when in proximity of a submarine."}</li>
                <li>{"Fix air-dropped homing torpedoes seeking out-of-range targets."}</li>
                <li>{"Rocket torpedoes can be aimed easier."}</li>
                <li>{"Aircraft and rocket torpedoes deploy closer to target."}</li>
                <li>{"Fix automatic nickname trimming."}</li>
                <li>{"Restore support for certain old devices without BigInteger support."}</li>
                <li>{"Can no longer launch aircraft while underwater."}</li>
            </ul>

            <h3>{"1/13/2022"}</h3>

            <ul>
                <li>{"Improve spawning system to reduce spawn-killing."}</li>
                <li>{"Add visual and auditory representation for automatic anti-aircraft gunfire."}</li>
                <li>{"Improve appearance of water, shell trails, and airborne explosions."}</li>
                <li>{"Harden network connection logic against temporary connection loss."}</li>
                <li>{"Guided weapons prioritize hitting targets at similar depths/altitudes."}</li>
                <li>{"Guided missiles will no seek towards submerged submarines."}</li>
                <li>{"Reduce salvo size of missiles on Yasen."}</li>
                <li>{"Trees are now decorative (unaffected by boats)."}</li>
                <li>{"World border and sub vision overlays affect ships, particles, etc."}</li>
                <li>{"Fix team name length limit to allow more emojis."}</li>
            </ul>
        </>
    }
}

#[inline(never)]
fn changelog_2021_part_2() -> Html {
    html! {
        <>
            <h2>{"2021"}</h2>

            <h3>{"12/25/2021"}</h3>

            <ul>
                <li>{"Add Buyan corvette and associated weapons."}</li>
                <li>{"Change Seawolf armaments to include Tomahawk missiles in accordance with Discord vote."}</li>
                <li>{"Add Italian translation (thanks to Bug82)."}</li>
                <li>{"Improve jet aircraft audio."}</li>
                <li>{"Improve cinematic mode."}</li>
                <li>{"All ships will insta-crush trees on contact."}</li>
                <li>{"Disable mouse controls reverse for level 1 boats, for which it is too error-prone."}</li>
                <li>{"When you leave a team, your mines disappear (so they don't pose a threat to former teammates)."}</li>
                <li>{"When you upgrade, your aircraft disappear (so you can't amass too many at once)."}</li>
                <li>{"Prevent nickname from looking too much like a team name."}</li>
            </ul>

            <h3>{"12/19/2021"}</h3>

            <ul>
                <li>{"Add Clemenceau aircraft carrier and associated weapons and aircraft."}</li>
                <li>{"Add Acacia tree."}</li>
                <li>{"Lessened damage reduction for hitting bow or stern."}</li>
                <li>{"Decrease visibility of subs for ships without sonar, if they didn't fire recently."}</li>
                <li>{"Fix some input and scaling bugs."}</li>
                <li>{"Mouse controls now allow going in reverse."}</li>
                <li>{"Collectibles no longer block weapons, except torpedoes."}</li>
                <li>{"Add cinematic mode, accessible via settings menu, that disables GUI."}</li>
                <li>{"Improve movement in the presence of high network latency."}</li>
            </ul>

            <h3>{"12/12/2021"}</h3>

            <ul>
                <li>{"During an update, we may allow you to remain on an old server, but scores achieved after most other players have left are not leaderboard-worthy."}</li>
            </ul>

            <h3>{"12/11/2021"}</h3>

            <ul>
                <li>{"Add Seawolf submarine."}</li>
                <li>{"Add more settings, including FPS counter."}</li>
                <li>{"Optimize particles speed and texture bandwidth."}</li>
            </ul>

            <h3>{"12/4/2021"}</h3>

            <ul>
                <li>{"Add Freccia destroyer."}</li>
                <li>{"Add Oberon submarine."}</li>
                <li>{"Add Dreadnought dreadnought (heh)."}</li>
                <li>{"Add German (thanks to Blackfur), Japanese, and Vietnamese translations."}</li>
                <li>{"Add muzzle flash effect."}</li>
                <li>{"Add levels page."}</li>
                <li>{"Reorganize levels, the respawn level cap is increased to 4."}</li>
                <li>{"Mark 48 torpedo does 33% more damage."}</li>
                <li>
                    {"Allow a "}
                    <Link href="https://raw.githubusercontent.com/finnbear/rustrict/master/src/safe.txt">
                        {"short list of safe messages"}
                    </Link>
                    {" while muted."}
                </li>
                <li>{"Fix bearing display."}</li>
            </ul>

            <h3>{"11/21/2021"}</h3>

            <ul>
                <li>{"Add a settings menu, and some graphics settings that may help on older hardware."}</li>
                <li>{"Fix a graphical issue with terrain border that affected some integrated GPUs."}</li>
                <li>{"Server chat messages have a colored name, to make it more difficult to impersonate the server."}</li>
                <li>{"The server can now send various semi-automated messages."}</li>
            </ul>

            <h3>{"11/11/2021"}</h3>

            <ul>
                <li>{"Fix balancing issue with Zumwalt shell damage."}</li>
                <li>{"Yamato is (realistically) less torpedo-resistant than other battleships."}</li>
                <li>{"Higher levels cost exponentially more score."}</li>
                <li>{"Change bot AI to be a bit better at avoiding obstacles, and less likely to fire shells at level 1 players."}</li>
            </ul>

            <h3>{"11/7/2021"}</h3>

            <ul>
                <li>{"Add Yamato battleship as first level 8."}</li>
            </ul>

            <h3>{"11/4/2021"}</h3>

            <ul>
                <li>{"Change visual appearance of splash screen."}</li>
                <li>{"Add foam at border of water."}</li>
                <li>{"Missiles fired by the Freedom LCS prefer the launcher with the closer angle."}</li>
            </ul>

            <h3>{"11/2/2021"}</h3>

            <ul>
                <li>{"Overhaul ocean and terrain."}</li>
                <li>{"Wake shows when submarines are cavitating (going fast and producing more noise)."}</li>
            </ul>

            <h3>{"10/31/2021"}</h3>

            <ul>
                <li>{"Active sonar, and moving slowly to avoid making too much noise, actually matter now."}</li>
                <li>{"If you are sunk by land, world border, obstacle, or leaving the game, a fraction of your score is dropped as coins."}</li>
                <li>{"Terrain updates more frequently and with less bandwidth use."}</li>
                <li>{"Dredger clamps to maximum depositor range, if nearly within range."}</li>
                <li>{"Fix enter to close chat, and right click name to open mute menu."}</li>
            </ul>

            <h3>{"10/28/2021"}</h3>

            <ul>
                <li>{"Add España dreadnought."}</li>
                <li>{"Oil barrels can go under oil platforms and hq's."}</li>
                <li>{"Sensors no longer switch to active briefly when upgrading."}</li>
                <li>{"Fix crash while using pinch-to-zoom (i.e. on mobile)."}</li>
                <li>{"Server tells you why it blocked your chat messages."}</li>
                <li>{"Advise against new players choosing certain ships (can be overridden by clicking lock 5 times)."}</li>
                <li>{"Tapping 'x' keys stops you even while turning with 'a' or 'd' key."}</li>
                <li>{"Change world border and particle aesthetic."}</li>
                <li>{"Fix sprite border visual glitch."}</li>
                <li>{"Submarines take more damage while ramming, because they are "}<i>{"fragile"}</i>{"."}</li>
                <li>{"Increase visual range of smaller ships, decrease that of bigger ships."}</li>
                <li>{"Change angles at which certain types of weapons fire."}</li>
            </ul>

            <h3>{"10/23/2021"}</h3>

            <ul>
                <li>{"Slightly reduce variation in turning speed due to ship length."}</li>
                <li>{"Fix a few team-related bugs."}</li>
                <li>{"Fix multiple music playing simultaneously."}</li>
            </ul>

            <h3>{"10/20/2021"}</h3>

            <ul>
                <li>{"Bots are less aggressive towards much smaller boats."}</li>
                <li>{"Improve network loading time if not cached."}</li>
                <li>{"Allow single-character player nicknames and team names."}</li>
            </ul>

            <h3>{"10/10/2021"}</h3>

            <ul>
                <li>{"Fix two more bugs related to names and teams."}</li>
                <li>{"Improve boat vs. boat collision resolution."}</li>
                <li>{"Increase sensitivity of CTRL + and CTRL - keys."}</li>
            </ul>

            <h3>{"10/7/2021"}</h3>

            <ul>
                <li>{"Fix several bugs related to names and teams."}</li>
                <li>{"Airborne SAMs can no longer hit submerged torpedoes."}</li>
                <li>{"Visby is no longer on fire when it shouldn't be."}</li>
                <li>{"Bots handle terrain a bit better."}</li>
            </ul>

            <h3>{"10/3/2021"}</h3>

            <ul>
                <li>{"Add smoke particles."}</li>
                <li>{"Add back invite feature, which is now usable regardless of whether you are a team captain."}</li>
                <li>{"Fix upgraded oil rigs drying up after an hour."}</li>
                <li>{"Fix mouse controls in wide aspect ratios."}</li>
                <li>{"Fix holding space or 'c' to shoot or donate repetitively."}</li>
                <li>{"Show number of players below leaderboard."}</li>
                <li>{"Fix rare desync related to team membership."}</li>
                <li>{"Stop showing bots in live leaderboard."}</li>
                <li>{"All weapons (not just rockets) now experience an angle deviation when launched, depending on their type."}</li>
            </ul>

            <h3>{"9/28/2021"}</h3>

            <ul>
                <li>{"Make aircraft more controllable."}</li>
                <li>{"Fix blur issue on mobile devices and high-DPI screens."}</li>
                <li>{"Fix name tag rendering below objects."}</li>
                <li>{"Increase rate of auto-regen for larger ships."}</li>
                <li>{"Allow you to see your team members further outside your sensor range."}</li>
            </ul>

            <h3>{"9/27/2021"}</h3>

            <ul>
                <li>{"Fix the stop key (x) to allow limited turning while stopped."}</li>
                <li>{"Make bots far less aggressive."}</li>
                <li>{"Increase visibility of ships (in multiple ways)."}</li>
                <li>{"Fix aircraft not reloading after being shot down."}</li>
                <li>{"Add back visibility circle for submerged submarines."}</li>
                <li>{"Reduce recoil for Olympias while ramming."}</li>
            </ul>

            <h3>{"9/26/2021"}</h3>

            <ul>
                <li>{"Improve performance of game client and decrease bandwidth requirement."}</li>
                <li>{"Improve lag compensation to reduce perceived network latency."}</li>
                <li>{"Behavior of torpedoes and submarine launched weapons changed to reflect altitude."}</li>
                <li>{"Overhaul all ship, weapon, etc. textures and particles."}</li>
                <li>{"Add firing arc display to HUD, remove confusing and almost completely useless sensor dots."}</li>
                <li>{"Increase intelligence of bots."}</li>
                <li>{"Add auto-reconnect feature to mitigate network issues that last less than 5 seconds."}</li>
                <li>{"Change terrain seed."}</li>
                <li>{"Stop restricting view of terrain with circle around ship."}</li>
                <li>{"Different ships and turrets turn at different speeds."}</li>
                <li>{"Shells and other fast moving projectiles are more likely to hit, among other physics improvements."}</li>
                <li>{"Add ability to mute players by right clicking their name in chat."}</li>
                <li>{"Stop limiting visibility of terrain to a circle, so you can use your whole screen."}</li>
                <li>{"Update privacy policy."}</li>
            </ul>

            <h3>{"8/8/2021"}</h3>

            <ul>
                <li>{"Aircraft can no longer drop weapons through oil platforms."}</li>
            </ul>

            <h3>{"6/29/2021"}</h3>

            <ul>
                <li>{"Add Kirov cruiser and associated weapons and aircraft."}</li>
                <li>{"Money can't buy happiness, and it "}<i>{"definitely"}</i>{" can't stop torpedoes (anymore)."}</li>
            </ul>

            <h3>{"6/28/2021"}</h3>

            <ul>
                <li>{"Begin adding music (so far only during epic moments)."}</li>
            </ul>

            <h3>{"6/27/2021"}</h3>

            <ul>
                <li>{"Add aircraft sound."}</li>
            </ul>

            <h3>{"6/25/2021"}</h3>

            <ul>
                <li>{"Add Type 055 destroyer in level 6, move Zumwalt to level 7."}</li>
                <li>{"Clarify turret angles by limiting turret rotation."}</li>
                <li>{"Fix rocket torpedo reload bug."}</li>
            </ul>

            <h3>{"6/23/2021"}</h3>

            <ul>
                <li>{"Add ability to land aircraft where they took off."}</li>
            </ul>

            <h3>{"6/22/2021"}</h3>

            <ul>
                <li>{"Add offline mode (must enable Beta program at bottom of Help page for now)."}</li>
            </ul>

            <h3>{"6/19/2021"}</h3>

            <ul>
                <li>{"Add support for sound effects."}</li>
            </ul>

            <h3>{"6/17/2021"}</h3>

            <ul>
                <li>{"Add active mode for radar and sonar, which allows better vision at the cost of giving away your position."}</li>
                <li>{"Add shallow water, which reduces the speed and health regeneration of larger boats (and the maximum depth of submarines). Small boats that are not in a team are likely to spawn in these areas."}</li>
            </ul>

            <h3>{"6/14/2021"}</h3>

            <ul>
                <li>{"Add Movska helicopter carrier and associated aircraft/weapons."}</li>
                <li>{"Add Tanker oil tanker."}</li>
                <li>{"Complete UI redesign."}</li>
                <li>{"Add ability deny team join requests."}</li>
                <li>{"Improve weapon guidance."}</li>
            </ul>

            <h3>{"6/12/2021"}</h3>

            <ul>
                <li>{"Players no longer respawn with their team within 10 seconds of being killed by another player."}</li>
            </ul>

            <h3>{"6/11/2021"}</h3>

            <ul>
                <li>{"Add support for translations."}</li>
            </ul>

            <h3>{"6/10/2021"}</h3>

            <ul>
                <li>{"Rebalance various reload speeds."}</li>
                <li>{"Improve player spawning."}</li>
                <li>{"Show health on Ships page."}</li>
            </ul>

            <h3>{"6/9/2021"}</h3>

            <ul>
                <li>{"Add Yasen class submarine (level 7) and associated weapons."}</li>
                <li>{"Add Town class cruiser (level 5) and move Bismarck and Montana battleships one level up."}</li>
                <li>{"Add Igla SAM to submarines where applicable."}</li>
                <li>{"Re-balance number of weapons on various high level ships."}</li>
                <li>{"Restrict turret angles where applicable."}</li>
                <li>{"Make stealth more effective."}</li>
                <li>{"Make land more vulnerable to weapons and erosion."}</li>
            </ul>

            <h3>{"6/5/2021"}</h3>

            <ul>
                <li>{"Add Freedom class LCS (level 6) and associated weapons."}</li>
                <li>{"Reduce Arleigh Burke number of ESSM SAM's from 8 to 4"}</li>
                <li>{"Add back spawn protection (lasts 10 seconds or until you attack)"}</li>
                <li>{"Improve supporting pages appearance and accuracy"}</li>
                <li>{"Improve keyboard input"}</li>
            </ul>

            <h3>{"6/1/2021"}</h3>

            <ul>
                <li>{"Add ASROC rocket-torpedo"}</li>
                <li>{"Add Kingfisher float-plane to battleships in place of airdropped torpedoes"}</li>
                <li>{"Improve loading and rendering efficiency"}</li>
                <li>{"Improve client turret aiming"}</li>
                <li>{"Add indicator to closed fleet panel showing number of pending requests"}</li>
                <li>{"Improve aircraft weapons, and add helicopters to more ships"}</li>
            </ul>
        </>
    }
}

#[inline(never)]
fn changelog_2021_part_1() -> Html {
    html! {
        <>
            <h3>{"5/30/2021"}</h3>

            <ul>
                <li>{"Add Zumwalt destroyer, including associated weapons and aircraft"}</li>
                <li>{"Submarines no longer have to surface to fire missiles"}</li>
                <li>{"Make bots less aggressive"}</li>
                <li>{"Change terrain seed"}</li>
            </ul>

            <h3>{"5/29/2021"}</h3>

            <ul>
                <li>{"Add ability to drop coins with 'c' key"}</li>
                <li>{"Add ability to [REDACTED] oil platforms with [REDACTED]"}</li>
                <li>{"Mines reload even before they detonate, and do not disappear when player dies"}</li>
                <li>{"Some torpedoes can damage land"}</li>
                <li>{"Make bots smarter"}</li>
                <li>{"Published daily/weekly leaderboards"}</li>
            </ul>

            <h3>{"5/28/2021"}</h3>

            <ul>
                <li>{"Improve keyboard controls"}</li>
                <li>{"Fix the speed and range of missiles and rockets"}</li>
            </ul>

            <h3>{"5/27/2021"}</h3>

            <ul>
                <li>{"Add SAMs (surface to air missiles)"}</li>
            </ul>

            <h3>{"5/26/2021"}</h3>

            <ul>
                <li>{"Randomize bot motion to avoid circling behavior"}</li>
                <li>{"Add Discord link to about about and help pages"}</li>
                <li>{"Change loot texture"}</li>
            </ul>

            <h3>{"5/25/2021"}</h3>

            <ul>
                <li>{"Add NPC pirate ship"}</li>
                <li>{"Aircraft can no longer fly over grass"}</li>
            </ul>

            <h3>{"5/24/2021"}</h3>

            <ul>
                <li>{"Add Akula submarine, displacing Ohio submarine to level 6"}</li>
                <li>{"Update help page"}</li>
                <li>{r#"Rename "Team" to "Fleet""#}</li>
            </ul>

            <h3>{"5/23/2021"}</h3>

            <ul>
                <li>{"Increase bot spawning at high player counts"}</li>
                <li>{"Increase reload time for aircraft"}</li>
                <li>{"Fix bugs with keyboard input and missile range"}</li>
                <li>{"Increase strength of automatic anti-aircraft guns"}</li>
            </ul>

            <h3>{"5/22/2021"}</h3>

            <ul>
                <li>{"Add Essex aircraft carrier"}</li>
                <li>{"Fix bugs related to Dredger"}</li>
                <li>{"Minimum team name length reduced to 2"}</li>
            </ul>

            <h3>{"5/21/2021"}</h3>

            <ul>
                <li>{"Reduce particles if low performance"}</li>
                <li>{"Clarify ship and weapon types in tooltip"}</li>
                <li>{"Update help and ships pages"}</li>
            </ul>

            <h3>{"5/20/2021"}</h3>

            <ul>
                <li>{"Add Lublin minelayer"}</li>
                <li>{"Update ships page"}</li>
                <li>{"Change mine reload time"}</li>
                <li>{"Oil barrels no longer reload weapons"}</li>
            </ul>

            <h3>{"5/19/2021"}</h3>

            <ul>
                <li>{"Add team chat, activated with shift+enter while sending"}</li>
                <li>{"Make it less likely for projectiles to go through grass"}</li>
                <li>{"Fix bug that made sonar decoys infinite"}</li>
                <li>{"Add ships page"}</li>
            </ul>

            <h3>{"5/18/2021"}</h3>

            <ul>
                <li>{"Airdropped weapons take longer to regenerate"}</li>
                <li>{"Other changes to reload times"}</li>
                <li>{"Add Kolkata destroyer"}</li>
            </ul>

            <h3>{"5/17/2021"}</h3>

            <ul>
                <li>{"Update HUD and keyboard input more often"}</li>
                <li>{"World border does not kill ships instantly"}</li>
                <li>{"Fix submarine altitude bug"}</li>
                <li>{"Add airdropped torpedoes to battleships"}</li>
                <li>{"Improve keyboard controls"}</li>
            </ul>

            <h3>{"5/16/2021"}</h3>

            <ul>
                <li>{"Add shift+v shortcut to record ingame video (slow)"}</li>
                <li>{"Improve keyboard controls"}</li>
                <li>{"Change how ram ships inflict damage"}</li>
                <li>{"Fix sonar decoy instantly killing ships"}</li>
            </ul>

            <h3>{"5/15/2021"}</h3>

            <ul>
                <li>{"Add MK70 sonar decoy"}</li>
                <li>{"Allow submarines to remain submerged while firing torpedoes"}</li>
                <li>{"Update help page"}</li>
                <li>{"Add Visby corvette"}</li>
                <li>{"Automatically slow down while turning"}</li>
            </ul>

            <h3>{"5/14/2021"}</h3>

            <ul>
                <li>{"Add ability to click name to auto-reply in radio"}</li>
                <li>{"Update terms and about pages"}</li>
                <li>{"All torpedoes can hit submerged submarines"}</li>
                <li>{"Weapon damage lower when further from ship center"}</li>
                <li>{"Improve spawn-protection"}</li>
                <li>{"Add 'x' button to stop ship"}</li>
            </ul>

            <h3>{"5/13/2021"}</h3>

            <ul>
                <li>{"Improve keyboard input"}</li>
            </ul>

            <h3>{"5/12/2021"}</h3>

            <ul>
                <li>{"Only regenerate half of damage while upgrading"}</li>
                <li>{"Fix bugs related to weapons"}</li>
                <li>{"Add Ohio submarine"}</li>
            </ul>

            <h3>{"5/11/2021"}</h3>

            <ul>
                <li>{"Bots avoid land"}</li>
                <li>{"Can use 'Enter' to enter radio"}</li>
                <li>{"Submarines can surface without firing"}</li>
                <li>{"Bots don't cluster near center of world"}</li>
            </ul>

            <h3>{"5/10/2021"}</h3>

            <ul>
                <li>{"Pending radio message is saved after respawning"}</li>
                <li>{"Team members collide but do no damage"}</li>
                <li>{"Hovercrafts can travel on land"}</li>
                <li>{"Fix client crash"}</li>
            </ul>

            <h3>{"5/9/2021"}</h3>

            <ul>
                <li>{"Hint about being rammed in death message"}</li>
                <li>{"Begin recording snapshots of land every 5 seconds"}</li>
                <li>{"Teach bots how to use ram ships"}</li>
                <li>{"Add gun to Type VIIC submarine"}</li>
                <li>{"Homing torpedoes and depth charges can hit submerged submarines"}</li>
                <li>{"Add Leander cruiser"}</li>
            </ul>

            <h3>{"5/8/2021"}</h3>

            <ul>
                <li>{"Obstacles do not instantly kill ships"}</li>
                <li>{"Add Olympias ram"}</li>
                <li>{"Open source the game on GitHub"}</li>
            </ul>

            <h3>{"Before 5/8/2021"}</h3>

            <p>{"Changes before the game was open source are not yet documented"}</p>
        </>
    }
}
