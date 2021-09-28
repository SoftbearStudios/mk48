<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {fade} from 'svelte/transition';
	export let title;
</script>

<div class=modal transition:fade>
	<div class=header>
		<h2>{title}</h2>
	</div>
	<div class="content scrollbar">
		<slot/>
	</div>
	<button on:click={() => window.location = '#/'}>Close</button>
</div>

<style>
	button {
		font-size: 1rem;
		position: absolute;
		top: 0;
		right: 0.5em;
		width: min-content;
	}

	div.modal {
		background-color: #174479;
		bottom: 0px;
		color: white;
		padding: 15px;
		position: absolute;
		text-align: center;
		inset: 10%;
		box-shadow: 5px 5px 5px #00000020;
		border-radius: 0.5em;
	}

	div.header {
		height: 10%;
		left: 0;
		top: 0;
		right: 0;
	}

	div.content {
		height: 90%;
		overflow-y: auto;
		padding-right: 1rem;
		text-align: left;
	}
</style>
