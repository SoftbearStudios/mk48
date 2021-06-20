<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<div class='background'>
	<div class='inset'>
		<div class='page'>
			<slot/>
		</div>
	</div>
</div>

<style>
	div.background {
		background-color: #003574;
		background: url(/background.png);
		background-size: cover;
		color: white;
		font-family: sans-serif;
		overflow-y: scroll;
		position: absolute;
		inset: 0;
	}

	div.inset {
		background-color: transparent;
		left: 10vw;
		right: 10vw;
		top: 10vw;
		position: absolute;
	}

	div.page {
		background-color: #2c3e50;
		border-radius: 1em;
		box-shadow: 0em 0em 4em #00000030;
		margin-bottom: 12em;
		padding: 2em;
	}

	a {
		color: white;
	}

	h1 {
		margin-top: 0;
	}
</style>
