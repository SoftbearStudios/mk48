<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {fade} from 'svelte/transition';
	import XButton from '../component/XButton.svelte';
	export let title;
</script>

<div class=curtain transition:fade on:click={() => window.location = '#/'}>
	<div class=modal on:click|stopPropagation>
		<div class=header>
			<h2>{title}</h2>
		</div>
		<div class="content scrollbar">
			<slot/>
		</div>
		<XButton on:click={() => window.location = '#/'}/>
	</div>
</div>

<style>
	div.content {
		height: 90%;
		overflow-y: auto;
		padding-right: 1rem;
		text-align: left;
	}

	div.curtain {
		background-color: #0003;
		bottom: 0;
		left: 0;
		position: absolute;
		right: 0;
		top: 0;
	}

	div.header {
		height: 10%;
		left: 0;
		top: 0;
		right: 0;
	}

	div.modal {
		background-color: #174479;
		border-radius: 0.5em;
		bottom: 0px;
		box-shadow: 5px 5px 5px #00000020;
		color: white;
		inset: 10%;
		padding: 15px;
		position: absolute;
		text-align: center;
	}
</style>
