<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {fade} from 'svelte/transition';

	const transition = {duration: 500, delay: 500};
</script>

<div id="progress_spinner" in:fade={transition}>
	<div id="animation"/>
</div>

<style>
	#progress_spinner {
		left: 50%;
		position: absolute;
		top: 50%;
		transform: translate(-50%, -50%);
	}

	#animation {
		animation: spin 2s linear infinite;
		border: 5px solid #f3f3f3;
		border-radius: 50%;
		border-top: 5px solid #3b99fc;
		box-sizing: border-box;
		height: 20vh;
		width: 20vh;
	}

	@keyframes spin {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}
</style>
