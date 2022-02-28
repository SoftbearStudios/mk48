<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {onMount} from 'svelte';
	import t, {getLanguage, getLanguageList, setLanguage, translate} from '../util/translation.js';
	import BorkFlag from '../flags/BorkFlag.svelte';
	import ChineseFlag from '../flags/ChineseFlag.svelte';
	import EnglishFlag from '../flags/EnglishFlag.svelte';
	import FrenchFlag from '../flags/FrenchFlag.svelte';
	import GermanFlag from '../flags/GermanFlag.svelte';
	import ItalianFlag from '../flags/ItalianFlag.svelte';
	import JapaneseFlag from '../flags/JapaneseFlag.svelte';
	import RussianFlag from '../flags/RussianFlag.svelte';
	import SpanishFlag from '../flags/SpanishFlag.svelte';
	import VietnameseFlag from '../flags/VietnameseFlag.svelte';

	let language;
	let menuOpen;
	let timeout = null;

	onMount(() => {
		language = getLanguage();
	});

	function handleLanguageChange(lang) {
		language = lang;
		setLanguage(language);
		menuOpen = false;
		if (timeout !== null) {
			clearTimeout(timeout);
		}
	}

	function handleOpen() {
		if (!menuOpen) {
			menuOpen = true;
			if (timeout !== null) {
				clearTimeout(timeout);
			}
			timeout = setTimeout(() => {
				timeout = null;
				menuOpen = false;
			}, 10000);
		}
	}

</script>

<div id="language_selector" on:click={handleOpen}>
	{#if menuOpen}
		<select value={language} on:click|stopPropagation on:change={e => handleLanguageChange(e.target.value)}>
			{#each getLanguageList() as lang}
				<option value={lang}>{translate(lang, 'label')}</option>
			{/each}
		</select>
	{:else}
		{#if language == 'xx-bork'}
			<BorkFlag/>
		{:else if language == 'de'}
			<GermanFlag/>
		{:else if language == 'es'}
			<SpanishFlag/>
		{:else if language == 'fr'}
			<FrenchFlag/>
		{:else if language == 'it'}
			<ItalianFlag/>
		{:else if language == 'ja'}
			<JapaneseFlag/>
		{:else if language == 'ru'}
			<RussianFlag/>
		{:else if language == 'vi'}
			<VietnameseFlag/>
		{:else if language == 'zh'}
			<ChineseFlag/>
		{:else}
			<EnglishFlag/>
		{/if}
	{/if}
</div>

<style>
	div {
		padding-top: 7px;
		position: relative;
		width: 2rem;
	}

	select {
		background-color: #CCC;
		color: black;
		position: absolute;
		right: 0;
		top: 0;
		width: min-content;
	}
</style>
