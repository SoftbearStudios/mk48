<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Page from './Page.svelte';
	import t, {setLanguage, translateAs} from '../lib/translation.js';
	import strings from '../data/strings.json';
	import storage from '../util/storage.js';

    import {antialias, chatOpen, fpsCounter, renderFoam, renderTerrainTextures, renderWaves, resolution} from '../util/settings.js';

    // Serializes settings that require a restart.
    function serializeRefreshSettings(antialias, renderFoam, renderTerrainTextures, renderWaves) {
        return JSON.stringify({antialias, renderFoam, renderTerrainTextures, renderWaves});
    }

    const initialRefreshSettings = serializeRefreshSettings($antialias, $renderFoam, $renderTerrainTextures, $renderWaves);
</script>

<Page title={$t('page.settings.title')}>
    <h3>General</h3>

    <label>
        <input type="checkbox" bind:checked={$chatOpen}/>
        Show Chat
    </label>

    <label>
        <input type="checkbox" bind:checked={$fpsCounter}/>
        Show FPS Counter
    </label>

    <select value={storage.language} on:change={e => setLanguage(e.target.value)}>
        {#each Object.keys(strings) as lang}
            {#if Object.keys(strings[lang]).length > 0}
                <option value={lang}>{translateAs(lang, 'label')}</option>
            {/if}
        {/each}
    </select>

    <h3>Graphics</h3>

    <label>
        <input type="checkbox" bind:checked={$renderWaves}/>
        Render Waves
    </label>

    <label>
        <input type="checkbox" bind:checked={$renderFoam}/>
        Render Foam
    </label>

    <label>
        <input type="checkbox" bind:checked={$renderTerrainTextures}/>
        Render Terrain Textures
    </label>

    <label>
        <input type="checkbox" bind:checked={$antialias}/>
        Antialiasing
    </label>

    <select value={$resolution} on:change={e => resolution.set(parseFloat(e.target.value))}>
        {#each [1.0, 0.5] as res}
            <option value={res}>{res * 100}% Resolution</option>
        {/each}
    </select>

    {#if initialRefreshSettings !== serializeRefreshSettings($antialias, $renderFoam, $renderTerrainTextures, $renderWaves)}
        <button on:click={() => location.reload(true)}>Apply Changes</button>
    {/if}

    <p>Note: None of these settings are intended to improve performance. They are for personal preferences or hardware support.</p>
</Page>

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
