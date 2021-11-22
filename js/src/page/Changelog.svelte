<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Page from './Page.svelte';
	import t from '../lib/translation.js';
</script>

<Page title={$t('page.changelog.title')}>
	<p>Warning: This changelog may not always be fully up to date</p>

	<h2>2021</h2>

	<h3>11/21/2021</h3>

	<ul>
		<li>Add a settings menu, and some graphics settings that may help on older hardware.</li>
		<li>Fix a graphical issue with terrain border that affected some integrated GPUs.</li>
		<li>Server chat messages have a colored name, to make it more difficult to impersonate the server.</li>
		<li>The server can now send various semi-automated messages.</li>
	</ul>

	<h3>11/11/2021</h3>

	<ul>
		<li>Fix balancing issue with "Zumwalt" shell damage.</li>
		<li>"Yamato" is (realistically) less torpedo-resistant than other battleships.</li>
		<li>Higher levels cost exponentially more score.</li>
		<li>Change bot AI to be a bit better at avoiding obstacles, and less likely to fire shells at level 1 players.</li>
	</ul>

	<h3>11/7/2021</h3>

	<ul>
		<li>Add "Yamato" battleship as first level 8.</li>
	</ul>

	<h3>11/4/2021</h3>

	<ul>
		<li>Change visual appearance of splash screen.</li>
		<li>Add foam at border of water.</li>
		<li>Missiles fired by the "Freedom" LCS prefer the launcher with the closer angle.</li>
	</ul>

	<h3>11/2/2021</h3>

	<ul>
		<li>Overhaul ocean and terrain.</li>
		<li>Wake shows when submarines are cavitating (going fast and producing more noise).</li>
	</ul>

	<h3>10/31/2021</h3>

	<ul>
		<li>Active sonar, and moving slowly to avoid making too much noise, actually matter now.</li>
		<li>If you are sunk by land, world border, obstacle, or leaving the game,
		a fraction of your score is dropped as coins.</li>
		<li>Terrain updates more frequently and with less bandwidth use.</li>
		<li>Dredger clamps to maximum depositor range, if nearly within range.</li>
		<li>Fix enter to close chat, and right click name to open mute menu.</li>
	</ul>

	<h3>10/28/2021</h3>

	<ul>
		<li>Add "España" dreadnought.</li>
		<li>Oil barrels can go under oil platforms and hq's.</li>
		<li>Sensors no longer switch to active briefly when upgrading.</li>
		<li>Fix crash while using pinch-to-zoom (i.e. on mobile).</li>
		<li>Server tells you why it blocked your chat messages.</li>
		<li>Advise against new players choosing certain ships (can be overridden by clicking lock 5 times).</li>
		<li>Tapping 'x' keys stops you even while turning with 'a' or 'd' key.</li>
		<li>Change world border and particle aesthetic.</li>
		<li>Fix sprite border visual glitch.</li>
		<li>Submarines take more damage while ramming, because they are <i>fragile</i>.</li>
		<li>Increase visual range of smaller ships, decrease that of bigger ships.</li>
		<li>Change angles at which certain types of weapons fire.</li>
	</ul>

	<h3>10/23/2021</h3>

	<ul>
		<li>Slightly reduce variation in turning speed due to ship length.</li>
		<li>Fix a few team-related bugs.</li>
		<li>Fix multiple music playing simultaneously.</li>
	</ul>

	<h3>10/20/2021</h3>

	<ul>
		<li>Bots are less aggressive towards much smaller boats.</li>
		<li>Improve network loading time if not cached.</li>
		<li>Allow single-character player nicknames and team names.</li>
	</ul>

	<h3>10/10/2021</h3>

	<ul>
		<li>Fix two more bugs related to names and teams.</li>
		<li>Improve boat vs. boat collision resolution.</li>
		<li>Increase sensitivity of CTRL + and CTRL - keys.</li>
	</ul>

	<h3>10/7/2021</h3>

	<ul>
		<li>Fix several bugs related to names and teams.</li>
		<li>Airborne SAMs can no longer hit submerged torpedoes.</li>
		<li>"Visby" is no longer on fire when it shouldn't be.</li>
		<li>Bots handle terrain a bit better.</li>
	</ul>

	<h3>10/3/2021</h3>

	<ul>
		<li>Add smoke particles.</li>
		<li>Add back invite feature, which is now usable regardless of whether you are a team captain.</li>
		<li>Fix upgraded oil rigs drying up after an hour.</li>
		<li>Fix mouse controls in wide aspect ratios.</li>
		<li>Fix holding space or 'c' to shoot or donate repetitively.</li>
		<li>Show number of players below leaderboard.</li>
		<li>Fix rare desync related to team membership.</li>
		<li>Stop showing bots in live leaderboard.</li>
		<li>All weapons (not just rockets) now experience an angle deviation when launched, depending on their type.</li>
	</ul>

	<h3>9/28/2021</h3>

	<ul>
		<li>Make aircraft more controllable.</li>
		<li>Fix blur issue on mobile devices and high-DPI screens.</li>
		<li>Fix name tag rendering below objects.</li>
		<li>Increase rate of auto-regen for larger ships.</li>
		<li>Allow you to see your team members further outside your sensor range.</li>
	</ul>

	<h3>9/27/2021</h3>

	<ul>
		<li>Fix the stop key (x) to allow limited turning while stopped.</li>
		<li>Make bots far less aggressive.</li>
		<li>Increase visibility of ships (in multiple ways).</li>
		<li>Fix aircraft not reloading after being shot down.</li>
		<li>Add back visibility circle for submerged submarines.</li>
		<li>Reduce recoil for Olympias while ramming.</li>
	</ul>

	<h3>9/26/2021</h3>

	<ul>
		<li>Improve performance of game client and decrease bandwidth requirement.</li>
		<li>Improve lag compensation to reduce perceived network latency.</li>
		<li>Behavior of torpedoes and submarine launched weapons changed to reflect altitude.</li>
		<li>Overhaul all ship, weapon, etc. textures and particles.</li>
		<li>Add firing arc display to HUD, remove confusing and almost completely useless sensor dots.</li>
		<li>Increase intelligence of bots.</li>
		<li>Add auto-reconnect feature to mitigate network issues that last less than 5 seconds.</li>
		<li>Change terrain seed.</li>
		<li>Stop restricting view of terrain with circle around ship.</li>
		<li>Different ships and turrets turn at different speeds.</li>
		<li>Shells and other fast moving projectiles are more likely to hit, among other physics improvements.</li>
		<li>Add ability to mute players by right clicking their name in chat.</li>
		<li>Stop limiting visibility of terrain to a circle, so you can use your whole screen.</li>
		<li>Update privacy policy.</li>
	</ul>

	<h3>8/8/2021</h3>

	<ul>
		<li>Aircraft can no longer drop weapons through oil platforms.</li>
	</ul>

	<h3>6/29/2021</h3>

	<ul>
		<li>Add "Kirov" cruiser and associated weapons and aircraft.</li>
		<li>Money can't buy happiness, and it <i>definitely</i> can't stop torpedoes (anymore).</li>
	</ul>

	<h3>6/28/2021</h3>

	<ul>
		<li>Begin adding music (so far only during epic moments).</li>
	</ul>

	<h3>6/27/2021</h3>

	<ul>
		<li>Add aircraft sound.</li>
	</ul>

	<h3>6/25/2021</h3>

	<ul>
		<li>Add "Type 055" destroyer in level 6, move "Zumwalt" to level 7.</li>
		<li>Clarify turret angles by limiting turret rotation.</li>
		<li>Fix rocket torpedo reload bug.</li>
	</ul>

	<h3>6/23/2021</h3>

	<ul>
		<li>Add ability to land aircraft where they took off.</li>
	</ul>

	<h3>6/22/2021</h3>

	<ul>
		<li>Add offline mode (must enable Beta program at bottom of Help page for now).</li>
	</ul>

	<h3>6/19/2021</h3>

	<ul>
		<li>Add support for sound effects.</li>
	</ul>

	<h3>6/17/2021</h3>

	<ul>
		<li>Add active mode for radar and sonar, which allows better vision at
		the cost of giving away your position.</li>
		<li>Add shallow water, which reduces the speed and health regeneration of
		larger boats (and the maximum depth of submarines). Small boats that are
		not in a team are likely to spawn in these areas.</li>
	</ul>

	<h3>6/14/2021</h3>

	<ul>
		<li>Add "Movska" helicopter carrier and associated aircraft/weapons.</li>
		<li>Add "Tanker" oil tanker.</li>
		<li>Complete UI redesign.</li>
		<li>Add ability deny team join requests.</li>
		<li>Improve weapon guidance.</li>
	</ul>

	<h3>6/12/2021</h3>

	<ul>
		<li>Players no longer respawn with their team within 10 seconds of being
		killed by another player.</li>
	</ul>

	<h3>6/11/2021</h3>

	<ul>
		<li>Add support for translations.</li>
	</ul>

	<h3>6/10/2021</h3>

	<ul>
		<li>Rebalance various reload speeds.</li>
		<li>Improve player spawning.</li>
		<li>Show health on Ships page.</li>
	</ul>

	<h3>6/9/2021</h3>

	<ul>
		<li>Add "Yasen" class submarine (level 7) and associated weapons.</li>
		<li>Add "Town" class cruiser (level 5) and move "Bismarck" and "Montana" battleships one level up.</li>
		<li>Add "Igla" SAM to submarines where applicable.</li>
		<li>Rebalance number of weapons on various high level ships.</li>
		<li>Restrict turret angles where applicable.</li>
		<li>Make stealth more effective.</li>
		<li>Make land more vulnerable to weapons and erosion.</li>
	</ul>

	<h3>6/5/2021</h3>

	<ul>
		<li>Add "Freedom" class LCS (level 6) and associated weapons.</li>
		<li>Reduce "Arleigh Burke" number of "ESSM" SAM's from 8 to 4</li>
		<li>Add back spawn protection (lasts 10 seconds or until you attack)</li>
		<li>Improve supporting pages appearance and accuracy</li>
		<li>Improve keyboard input</li>
	</ul>

	<h3>6/1/2021</h3>

	<ul>
		<li>Add "ASROC" rocket-torpedo</li>
		<li>Add "Kingfisher" floatplane to battleships in place of airdropped torpedoes</li>
		<li>Improve loading and rendering efficiency</li>
		<li>Improve client turret aiming</li>
		<li>Add indicator to closed fleet panel showing number of pending requests</li>
		<li>Improve aircraft weapons, and add helicopters to more ships</li>
	</ul>

	<h3>5/30/2021</h3>

	<ul>
		<li>Add "Zumwalt" destroyer, including associated weapons and aircraft</li>
		<li>Submarines no longer have to surface to fire missiles</li>
		<li>Make bots less aggressive</li>
		<li>Change terrain seed</li>
	</ul>

	<h3>5/29/2021</h3>

	<ul>
		<li>Add ability to drop coins with 'c' key</li>
		<li>Add ability to [REDACTED] oil platforms with [REDACTED]</li>
		<li>Mines reload even before they detonate, and do not disappear when player dies</li>
		<li>Some torpedoes can damage land</li>
		<li>Make bots smarter</li>
		<li>Published daily/weekly leaderboards</li>
	</ul>

	<h3>5/28/2021</h3>

	<ul>
		<li>Improve keyboard controls</li>
		<li>Fix the speed and range of missiles and rockets</li>
	</ul>

	<h3>5/27/2021</h3>

	<ul>
		<li>Add SAMs (surface to air missiles)</li>
	</ul>

	<h3>5/26/2021</h3>

	<ul>
		<li>Randomize bot motion to avoid circling behavior</li>
		<li>Add Discord link to about about and help pages</li>
		<li>Change loot texture</li>
	</ul>

	<h3>5/25/2021</h3>

	<ul>
		<li>Add NPC priate ship</li>
		<li>Aircraft can no longer fly over grass</li>
	</ul>

	<h3>5/24/2021</h3>

	<ul>
		<li>Add "Akula" submarine, displacing "Ohio" submarine to level 6</li>
		<li>Update help page</li>
		<li>Rename "Team" to "Fleet"</li>
	</ul>

	<h3>5/23/2021</h3>

	<ul>
		<li>Increase bot spawning at high player counts</li>
		<li>Increase reload time for aircraft</li>
		<li>Fix bugs with keyboard input and missile range</li>
		<li>Increase strength of automatic anti-aircraft guns</li>
	</ul>

	<h3>5/22/2021</h3>

	<ul>
		<li>Add "Essex" aircraft carrier</li>
		<li>Fix bugs related to "Dredger"</li>
		<li>Minimum team name length reduced to 2</li>
	</ul>

	<h3>5/21/2021</h3>

	<ul>
		<li>Reduce particles if low performance</li>
		<li>Clarify ship and weapon types in tooltip</li>
		<li>Update help and ships pages</li>
	</ul>

	<h3>5/20/2021</h3>

	<ul>
		<li>Add "Lublin" minelayer</li>
		<li>Update ships page</li>
		<li>Change mine reload time</li>
		<li>Oil barrels no longer reload weapons</li>
	</ul>

	<h3>5/19/2021</h3>

	<ul>
		<li>Add team chat, activated with shift+enter while sending</li>
		<li>Make it less likely for projectiles to go through grass</li>
		<li>Fix bug that made sonar decoys infinite</li>
		<li>Add ships page</li>
	</ul>

	<h3>5/18/2021</h3>

	<ul>
		<li>Airdropped weapons take longer to regenerate</li>
		<li>Other changes to reload times</li>
		<li>Add "Kolkata" destroyer</li>
	</ul>

	<h3>5/17/2021</h3>

	<ul>
		<li>Update HUD and keyboard input more often</li>
		<li>World border does not kill ships instantly</li>
		<li>Fix submarine altitude bug</li>
		<li>Add airdropped torpedoes to battleships</li>
		<li>Improve keyboard controls</li>
	</ul>

	<h3>5/16/2021</h3>

	<ul>
		<li>Add shift+v shortcut to record ingame video (slow)</li>
		<li>Improve keyboard controls</li>
		<li>Change how ram ships inflict damage</li>
		<li>Fix sonar decoy instantly killing ships</li>
	</ul>

	<h3>5/15/2021</h3>

	<ul>
		<li>Add "mk70" sonar decoy</li>
		<li>Allow submarines to remain submerged while firing torpedoes</li>
		<li>Update help page</li>
		<li>Add "Visby" corvette</li>
		<li>Automatically slow down while turning</li>
	</ul>

	<h3>5/14/2021</h3>

	<ul>
		<li>Add ability to click name to auto-reply in radio</li>
		<li>Update terms and about pages</li>
		<li>All torpedoes can hit submerged submarines</li>
		<li>Weapon damage lower when further from ship center</li>
		<li>Improve spawn-protection</li>
		<li>Add 'x' button to stop ship</li>
	</ul>

	<h3>5/13/2021</h3>

	<ul>
		<li>Improve keyboard input</li>
	</ul>

	<h3>5/12/2021</h3>

	<ul>
		<li>Only regenerate half of damage while upgrading</li>
		<li>Fix bugs related to weapons</li>
		<li>Add "Ohio" submarine</li>
	</ul>

	<h3>5/11/2021</h3>

	<ul>
		<li>Bots avoid land</li>
		<li>Can use 'Enter' to enter radio</li>
		<li>Submarines can surface without firing</li>
		<li>Bots don't cluster near center of world</li>
	</ul>

	<h3>5/10/2021</h3>

	<ul>
		<li>Pending radio message is saved after respawning</li>
		<li>Team members collide but do no damage</li>
		<li>Hovercrafts can travel on land</li>
		<li>Fix client crash</li>
	</ul>

	<h3>5/9/2021</h3>

	<ul>
		<li>Hint about being rammed in death message</li>
		<li>Begin recording snapshots of land every 5 seconds</li>
		<li>Teach bots how to use ram ships</li>
		<li>Add gun to "Type VIIC" submarine</li>
		<li>Homing torpedoes and depth charges can hit submerged submarines</li>
		<li>Add "Leander" cruiser</li>
	</ul>

	<h3>5/8/2021</h3>

	<ul>
		<li>Obstacles do not instantly kill ships</li>
		<li>Add "Olympias" ram</li>
		<li>Open source project on GitHub</li>
	</ul>

	<h3>Before 5/8/2021</h3>

	<p>Changes before the game was open source are not yet documented</p>
</Page>

<style>
	a {
		color: white;
	}

	h1 {
		margin-top: 0;
	}

	table {
		border-spacing: 1em;
		text-align: left;
	}
</style>
