<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Link from '../lib/Link.svelte';
	import Page from './Page.svelte';
	import t from '../lib/translation.js';
</script>

<Page title={$t('page.help.title')}>
	<h2>{$t('page.help.basics.title')}</h2>

	<p>{@html $t('page.help.basics.html')}</p>

	<p>You can <Link href='https://discord.gg/YMheuFQWTX'>join the Discord server</Link> to
	get help from the community!</p>

	<h2>Movement</h2>

	<p>There are four ways to move your ship:</p>

	<ol>
		<li><b>Left click and hold</b> outside your ship's inner ring</li>
		<li><b>Right click</b> (and optionally hold) outside your ship's inner
		ring <i>(this is the recommended way to move)</i></li>
		<li><b>Tap and hold</b> (on touch screen) outside your ship's inner ring</li>
		<li>Use the <b>WASD</b> or <b>arrow</b> keys to move, and 'x' key to stop</li>
	</ol>

	<p>Once your ship is moving in a direction, it will keep moving, to let you
	focus on using your weapons.</p>

	<p>If using mouse controls, your ship will turn to point towards your
	mouse. You can control the speed of your ship by using your mouse position
	between the two rings as a throttle. If your mouse is outside the outer
	ring, your ship will increase to maximum speed.</p>

	<h2>Ships</h2>

	<p>Just like in real life, there are many different types of ships, each
	with different advantages.</p>

	<ol>
		<li><b>Motor-torpedo boats</b> are the type of most level one ships,
		generally having multiple torpedoes and possibly guns.</li>
		<li><b>Corvettes</b> and <b>Destroyers</b> are larger boats that carry more weapons, including
		slightly more powerful guns.</li>
		<li><b>Cruisers</b> are a compromise between destroyers and battleships.</li>
		<li><b>Battleships</b> are very formidable ships, having extremely
		powerful main cannons. They may carry a minimal amount of planes for submarine defense.</li>
		<li><b>Submarines</b> travel underwater, making them immune to certain
		types of weapons, but must surface to fire certain types of weapons and regenerate others at a normal rate.</li>
		<li><b>Hovercraft</b> can travel on land and water.</li>
		<li><b>Rams</b> are a special ship type that is designed to ram other
		ships. They take very little damage in the process, and inflict extra damage to
		the other ship.</li>
		<li><b>Dredgers</b> have the special ability to modify the land. New land
		can be created by clicking in front of them, and old land can be destroyed
		by driving over it.</li>
		<li><b>Minelayers</b> dispense long lasting mines that can help guard a
		small area.</li>
		<li><b>Aircraft carriers</b> command a squadron of planes to bomb enemy ships! Planes will follow your mouse cursor.</li>
	</ol>

	<p>Once you earn enough points, you are able to <b>upgrade</b> your ship to the
	next level. The upgrade choices are shown in top-middle of  the screen when an
	upgrade is available. Be careful when upgrading, as becoming a larger
	ship may lead to crashing into land or a reduction in mobility if the water is too shallow.</p>

	<p>Enemies are more likely to detect larger ships. However, some modern ships have
	a property known as <b>stealth</b> to help you evade detection.</p>

	<p>Here is a full list of ships: <a href='#/ships'>Mk48.io Ships</a></p>

	<h2>Weapons</h2>

	<p>There are multiple different types of weapons available on various
	ships. In general, weapons are fired by clicking in the appropriate
	direction (although the Space key can also be used).</p>

	<ol>
		<li><b>Torpedoes</b> are powerful underwater weapons. Some torpedoes
		have the ability to track targets automatically, with sonar.</li>

		<li><b>Missiles</b> are airborne and are faster, but less maneuverable than torpedoes.</li>

		<li><b>Rockets</b> are like missiles but lack guidance.</li>

		<li><b>SAMs</b> (surface-to-air missiles) can shoot down aircraft and missiles.</li>

		<li><b>Gun turrets</b> shoot very fast shells that do moderate
		damage. Once fired, a shell is without guidance.</li>

		<li><b>Depth charges</b> are stationary weapons that can be deployed
		against persuing ships or submerged submarines.</li>

		<li><b>Mines</b> are like depth charges but last much longer and are
		more powerful.</li>

		<li><b>Aircraft</b> fly towards your mouse cursor, and automatically deploy weapons of their own.</li>

		<li><b>Depositor</b> creates new land. We'll let you figure out if this
		can be used as a weapon.</li>
	</ol>

	<p>Once fired, all weapons take some time to <b>reload</b>. Each type of weapon
	and each turret reload independently. Consuming crates speeds up reloading,
	while using weapons underwater slows it down.</p>

	<h2>Sensors</h2>

	<p>All ships have some combination of sensors to identify other ships and obstacles:</p>

	<ol>
		<li><b>Visual</b> tracks all targets, with a range that depends on conditions.</li>
		<li><b>Radar</b> tracks targets above water.</li>
		<li><b>Sonar</b> tracks underwater targets.</li>
	</ol>

	<p>There are two modes for sensors, <b>active</b> and <b>passive</b>. Passive
	mode listens for emissions (e.g. sound in the case of sonar) from other entities.
	Active mode emits a signal and resolves contacts based on the signals that bounce
	back. In general, active mode allows you to see more, but has the potential
	to give away your position.</p>

	<!--
	<p>To visualize sensor data, look at the dots surrounding the outer circle
	around your ship:</p>

	<ol>
		<li><b>Green</b> dots are friendly.</li>
		<li><b>Red</b> dots are enemies (visual or radar).</li>
		<li><b>Blue</b> dots are enemies (sonar).</li>
		<li><b>Yellow</b> dots are collectibles that increase your health, ammunition, and score.</li>
	</ol>
	-->

	<p>If a contact is on the border of your sensor range, it will
	appear as an arrow. In this case, you know something is there, but not what
	it is.</p>

	<h2>Fleets</h2>

	<p>You can join up to five other players to form a fleet, using the panel
	in the top left corner of the screen. Once you have a fleet, other players
	can request to join it. You can then approve or deny them. Members of a fleet
	cannot hurt each other with weapons. Importantly, you cannot request to join
	a fleet until you are close enough to see one of its members, and the fleet
	has slots remaining.</p>
</Page>

<style>
	a {
		color: white;
	}

	h1 {
		margin-top: 0;
	}
</style>
