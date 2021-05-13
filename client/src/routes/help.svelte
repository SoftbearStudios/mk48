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
		<li><b>Right click</b> (and optionally hold) outside your ship's inner ring</li>
		<li><b>Tap and hold</b> (on touch screen) outside your ship's inner ring</li>
		<li>Use the <b>WASD</b> or <b>arrow</b> keys.</li>
	</ol>

	<p>If using mouse controls, your ship will turn to point towards your
	mouse. You can control the speed of your ship by using your mouse position
	between the two rings as a throttle. If your mouse is outside the outer
	ring, your ship will increase to maximum speed.</p>

	<h2>Weapons</h2>

	<p>There are multiple different types of weapons available on various
	ships:</p>

	<ol>
		<li><b>Torpedoes</b> are powerful underwater weapons. Once fired, a
		torpedo will turn until it reaches the desired angle. Some torpedoes
		have the ability to track targets automatically, with sonar.</li>

		<li><b>Missiles</b> are airborne and are faster, but less maneuverable than torpedoes.</li>

		<li><b>Rockets</b> are like missiles but lack guidance.</li>

		<li><b>Gun turrets</b> shoot very fast shells that do moderate
		damage. Once fired, a shell is without guidance.</li>

		<li><b>Depth charges</b> are stationary weapons that can be deployed
		against persuing ships.</li>

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
		<li><b>Red</b> dots are enemy (visual or radar).</li>
		<li><b>Blue</b> dots are enemy (sonar).</li>
		<li><b>Yellow</b> dots are collectible crates.</li>
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
