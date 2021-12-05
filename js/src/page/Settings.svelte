<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Page from './Page.svelte';
	import t, {setLanguage, translateAs} from '../lib/translation.js';
	import strings from '../data/strings.json';
	import storage from '../util/storage.js';

    import {chatOpen, renderFoam, renderTerrainTextures, renderWaves} from '../util/settings.js';
</script>

<Page title={$t('page.settings.title')}>
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
        <input type="checkbox" bind:checked={$chatOpen}/>
        Show Chat
    </label>

    <select class='language' value={storage.language} on:change={e => setLanguage(e.target.value)}>
        {#each Object.keys(strings) as lang}
            {#if Object.keys(strings[lang]).length > 0}
                <option value={lang}>{translateAs(lang, 'label')}</option>
            {/if}
        {/each}
    </select>

    <p>Warning: Graphics settings take effect after <b>refreshing the page</b>.</p>

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
	}
</style>
