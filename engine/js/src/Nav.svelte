<script>
    import {periods, resolutions} from './Series.svelte';
    import {game, games} from './util.js';


    import {link} from 'svelte-spa-router'
    import active from 'svelte-spa-router/active'
</script>

<nav>
    <a class="navbtn" href="/summary/*" use:link use:active>Summary</a>
    <a class="navbtn" href="/day/*" use:link use:active>Day</a>
    <a class="navbtn" href="/referrers" use:link use:active>Referrers</a>
    <a class="navbtn" href="/regions" use:link use:active>Regions</a>
    <a class="navbtn" href="/userAgents" use:link use:active>User Agents</a>
    <a class="navbtn" href="/series/{periods[0]}/{resolutions[0]}/*" use:link use:active={'/series/*/*/*'}>Series</a>
    <a class="navbtn" href="/chat" use:link use:active>Chat</a>
    <a class="navbtn" href="/system" use:link use:active>System</a>
    <a class="navbtn" href="/snippets" use:link use:active>Snippets</a>
    <div class="selections">
        <slot/>
        {#if !$games}
            <div/>
        {:else}
            <select on:change={event => game_id = event.target.value} value={$game}>
        {#each $games as g}
            <option value={g}>{g}</option>
        {/each}
            </select>
        {/if}
    </div>
</nav>
