<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import t from '../util/translation.js';
</script>

<span id="help_links">
	<a href='#/help'>{$t('panel.splash.action.help.label')}</a>
	<a href='#/about'>{$t('panel.splash.action.about.label')}</a>
	<a href='#/privacy'>{$t('panel.splash.action.privacy.label')}</a>
	<a href='#/terms'>{$t('panel.splash.action.terms.label')}</a>
</span>

<style>
	a {
		color: #FFFC;
		font-size: 1.2rem;
		margin-left: 0.25rem;
		margin-right: 0.25rem;
		padding-top: 4rem;
		white-space: nowrap;
	}

	span {
		bottom: 0.5rem;
		display: flex;
		font-size: 1rem;
		gap: 1rem;
		justify-content: center;
		left: 0;
		position: absolute;
		right: 0;
	}
</style>
