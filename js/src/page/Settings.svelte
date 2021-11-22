<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Page from './Page.svelte';
	import t from '../lib/translation.js';

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

    <p>Warning: Graphics settings take effect after <b>refreshing the page</b>.</p>

    <p>Note: None of these settings are intended to improve performance. They are for personal preferences or hardware support.</p>
</Page>

<style>
	label {
	    display: block;
		user-select: none;
		margin-bottom: 0.4em;
	}
</style>
