<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<div>
	<h1>Mk48.io Help Guide</h1>

	<h2>Basics</h2>

	<p>Mk48.io is a multiplayer naval combat game. You start off a small ship,
	and consume <b>crates</b> to increase your score (to level up your ship). While a
	small amount of crates spawn naturally, sinking other ships directly
	increases your score and spawns many more crates.</p>

	<h2>Movement</h2>

	<p>There are four ways to move your ship:</p>

	<ol>
		<li><b>Left click and hold</b> outside your ship's inner ring</li>
		<li><b>Right click</b> (and optionally hold) outside your ship's inner
		ring <i>(this is the recommended way to move)</i></li>
		<li><b>Tap and hold</b> (on touch screen) outside your ship's inner ring</li>
		<li>Use the <b>WASD</b> or <b>arrow</b> keys.</li>
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
		<li><b>Destroyers</b> are larger boats that carry more weapons, including
		slightly more powerful guns.</li>
		<li><b>Cruisers</b> are a compromise between destroyers and battleships.</li>
		<li><b>Battleships</b> are very formidable ships, having extremely
		powerful main cannons. They need some help to defend against submarines,
		though.</li>
		<li><b>Submarines</b> travel underwater, making them immune to certain
		types of weapons, but must surface to fire weapons of their own.</li>
		<li><b>Hovercraft</b> can travel on land and water.</li>
		<li><b>Rams</b> are a special ship type that is designed to ram other
		ships. They take no damage in the process, and inflict extra damage to
		the other ship.</li>
		<li><b>Dredgers</b> have the special ability to modify the land. New land
		can be created by clicking in front of them, and old land can be destroyed
		by driving over them.</li>
		<li><b>Aircraft carriers</b> are coming soon!</li>
	</ol>

	<p>Once you earn enough points, you are able to <b>upgrade</b> your ship to the
	next level. The upgrade choices are shown in the top left corner of the
	screen, and are greyed out until they are available.</p>

	<h2>Weapons</h2>

	<p>There are multiple different types of weapons available on various
	ships. In general, weapons are fired by clicking in the appropriate
	direction (although the Space key can also be used).</p>

	<ol>
		<li><b>Torpedoes</b> are powerful underwater weapons. Some torpedoes
		have the ability to track targets automatically, with sonar.</li>

		<li><b>Missiles</b> are airborne and are faster, but less maneuverable than torpedoes.</li>

		<li><b>Rockets</b> are like missiles but lack guidance.</li>

		<li><b>Gun turrets</b> shoot very fast shells that do moderate
		damage. Once fired, a shell is without guidance.</li>

		<li><b>Depth charges</b> are stationary weapons that can be deployed
		against persuing ships or submerged submarines.</li>

		<li><b>Dredger</b>. It creates new land. We'll let you figure out if this
		can be used as a weapon.</li>
	</ol>

	<p>Once fired, all weapons take some time to reload. Each type of weapon
	and each turret reload independently. Consuming crates speeds up reloading.</p>

	<h2>Sensors</h2>

	<p>All ships have some combination of sensors to identify other ships and obstacles:</p>

	<ol>
		<li><b>Visual</b> tracks all targets, with a range that depends on conditions.</li>
		<li><b>Radar</b> tracks targets above water.</li>
		<li><b>Sonar</b> tracks underwater targets.</li>
	</ol>

	<p>To visualize sensor data, look at the dots surrounding the outer circle
	around your ship:</p>

	<ol>
		<li><b>Green</b> dots are friendly.</li>
		<li><b>Red</b> dots are enemies (visual or radar).</li>
		<li><b>Blue</b> dots are enemies (sonar).</li>
		<li><b>Yellow</b> dots are collectibles that increase your health, ammunition, and score.</li>
	</ol>

</div>

<style>
	div {
		background-color: #2c3e50;
		color: white;
		font-family: sans-serif;
		font-size: 20px;
		overflow-y: scroll;
		padding: 10px;
		position: absolute;
		inset: 0;
	}

	a {
		color: white;
	}

	h1 {
		margin-top: 0;
	}
</style>
