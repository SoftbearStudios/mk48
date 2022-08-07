<script>
    // SPDX-FileCopyrightText: 2021 Softbear, Inc.
    // SPDX-License-Identifier: AGPL-3.0-or-later

    import Router from 'svelte-spa-router'
    import {game, games} from './util.js';
    import Summary from './Summary.svelte';
    import Chat from './Chat.svelte';
    import Snippets from './Snippets.svelte';
    import System from './System.svelte';
    import Day from './Day.svelte';
    import Referrers, {referrers} from './Referrers.svelte';
    import Regions, {regions} from './Regions.svelte';
    import UserAgents, {userAgents} from './UserAgents.svelte';
    import Series, {periods} from './Series.svelte';

    const routes = {
        '/': Summary,
        '/summary/:filter': Summary,
        '/day/:filter': Day,
        '/referrers': Referrers,
        '/regions': Regions,
        '/userAgents': UserAgents,
        '/series/:period/:resolution/:filter': Series,
        '/chat': Chat,
        '/system': System,
        '/snippets': Snippets,
    }
</script>

{#if !$game}
    <h2>No data</h2>
{:else}
    <Router {routes}/>
{/if}

<style>
    :global(div.chart) {
        #background-color: black;
    }

    :global(div.db) {
        text-decoration: underline;
    }

    :global(div.charts) {
        display: grid;
        grid-gap: 10px 10px;
        grid-template-columns: repeat(4, 1fr);
    }

    :global(a.navbtn) {
        color: white;
        line-height: 2.1rem;
        padding-left: 1rem;
        filter: opacity(0.75);
    }

    :global(a.navbtn.active) {
        filter: opacity(1.0);
    }

    :global(div.selections) {
        position: absolute;
        right: 0;
    }

    :global(h1) {
        text-transform: uppercase;
    }

    :global(main) {
        color: black;
        font-family: Verdana;
        margin: 0 auto;
        max-width: 240px;
        padding: 1em;
        text-align: center;
    }

    :global(nav) {
        background: darkslategrey;
        color: #ddd;
        display: flex;
        height: 2.1rem;
        left: 50%;
        position: relative;
        transform: translate(-50%, 0%);
        width: 95%;
    }

    :global(select) {
        background: darkslategrey;
        color: white;
    }

    :global(table) {
        border: 1px solid gray;
        margin: auto;
    }

    :global(td) {
        padding-left: 1em;
        padding-right: 1em;
        text-align: left;
    }

    :global(td.detail) {
        font-style: italic;
    }

    :global(td.value) {
        font-weight: bold;
    }

    :global(th) {
        background-color: darkslategrey;
        color: white;
        font-weight: normal;
        padding-left: 1em;
        padding-right: 1em;
        text-align: left;
    }

    @media (min-width: 640px) {
        :global(main) {
            max-width: none;
        }
    }
</style>
