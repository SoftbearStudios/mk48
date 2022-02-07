<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
    export let message = "warning";

    async function tryRefresh() {
    	try {
    		await fetch("/");
    		location.reload();
		} catch (err) {
			// No-op.
		}
    }
</script>

<div id="warning_panel">
	{message}
	<!--
	<ul>
		<li>If you opened another tab, it may have taken over your connection.</li>
		<li>Otherwise, press refresh to try connecting again.</li>
	</ul>
	-->
	<button on:click={tryRefresh}>Refresh</button>
</div>

<style>
	div {
		background-color: #f6f6f6;
		border-radius: 1rem;
		box-shadow: 0em 0.25rem 0 #cccccc;
		color: #000000;
		font-size: 2rem;
		left: 50%;
		max-width: 60%;
		padding: 2rem;
		position: absolute;
		text-align: center;
		top: 50%;
		transform: translate(-50%, -50%);
		word-break: break-word;
	}
</style>
