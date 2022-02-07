<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import Dialog from './Dialog.svelte';
	import entityData from '../data/entities.json';
	import Sprite from '../component/Sprite.svelte';
	import {levelToScore, summarizeType} from '../util/warship.js';
	import t from '../util/translation.js';

	const levels = [];
	for (const entityType of Object.keys(entityData)) {
		const data = entityData[entityType];
		if (data.kind === 'boat' && !data.npc) {
			if (!Array.isArray(levels[data.level - 1])) {
				levels[data.level - 1] = [];
			}
			levels[data.level - 1].push(entityType);
		}
	}
	for (const boats of levels) {
		boats.sort((a, b) => {
			let one = entityData[a].subkind;
			let two = entityData[b].subkind;
			if (one < two) {
				return -1;
			}
			if (one > two) {
				return 1;
			}
			return 0;
		});
	}
</script>

<Dialog title="Mk48.io Levels">
	{#each levels as boats, i}
		<div>
			<h3>Level {i + 1} <i>({levelToScore(i + 1)} {$t('panel.status.scorePlural')})</i></h3>
			{#each boats as boatType}
				<div class="sprite">
					<Sprite
							title={`${entityData[boatType].label} (${summarizeType($t, boatType)})`}
							name={boatType}
					/>
				</div>
			{/each}
		</div>
	{/each}
</Dialog>

<style>
	div {
		text-align: center;
	}

	h1, h2, h3 {
		text-align: center;
	}

	div.sprite {
		display: inline-block;
		margin: 0.5em;
	}
</style>
