<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Dialog from './Dialog.svelte';
	import ServerPicker from '../component/ServerPicker.svelte';
	import t from '../util/translation.js';
	import strings from '../data/strings.json';
	import storage from '../util/storage.js';

    import {animations, antialias, chatShown, cinematic, fpsShown, leaderboardShown, resolution, waveQuality} from '../util/settings.js';

    // Passed via router props.
    export let state;

    let pendingAntialias = undefined;
    let pendingAnimations = undefined;
    let pendingWaveQuality = undefined;
    const simpleSettings = true;

    function applyChanges() {
        if (pendingAntialias !== undefined) {
            antialias.set(pendingAntialias);
        }
        if (pendingAnimations !== undefined) {
            animations.set(pendingAnimations);
        }
        if (pendingWaveQuality !== undefined) {
            waveQuality.set(pendingWaveQuality);
        }
        location.reload(true);
    }
</script>

<Dialog title={$t('page.settings.title')}>
    <h3>General</h3>

    <label>
        <input type="checkbox" bind:checked={$fpsShown}/>
        Show FPS Counter
    </label>

    {#if !simpleSettings}
        <label>
            <input type="checkbox" bind:checked={$leaderboardShown}/>
            Show Leaderboard
        </label>
    {/if}

    <label>
        <input type="checkbox" bind:checked={$chatShown}/>
        Show Radio
    </label>

    <label>
        <input type="checkbox" bind:checked={$cinematic}/>
        Cinematic Mode
    </label>

    <ServerPicker state={$state} settingsStyle={true}/>

    <h3>Graphics</h3>

    <label>
        <input type="checkbox" checked={pendingAnimations || $animations} on:change={e => pendingAnimations = e.target.checked}/>
        Animations
    </label>

    <label>
        <input type="checkbox" checked={pendingAntialias || $antialias} on:change={e => pendingAntialias = e.target.checked}/>
        Antialiasing
    </label>

    <select value={pendingWaveQuality || $waveQuality} on:change={e => pendingWaveQuality = parseInt(e.target.value)}>
        <option value={0}>No Waves</option>
        <option value={1}>Good Waves</option>
        <option value={2}>Great Waves</option>
        <option value={3}>Fantastic Waves</option>
    </select>

    <select value={$resolution} on:change={e => resolution.set(parseFloat(e.target.value))}>
        {#each [1.0, 0.5] as res}
            <option value={res}>{res * 100}% Resolution</option>
        {/each}
    </select>

    {#if (pendingWaveQuality !== undefined && pendingWaveQuality != $waveQuality) || (pendingAnimations !== undefined && pendingAnimations != $animations) || (pendingAntialias !== undefined && pendingAntialias != $antialias)}
        <button on:click={applyChanges}>Apply Changes</button>
    {/if}
</Dialog>

<style>
	label {
	    display: block;
		user-select: none;
		margin-bottom: 0.4em;
	}

	select {
	    background-color: #0075ff;
	    display: block;
	}

	button {
	    width: min-content;
	}
</style>
