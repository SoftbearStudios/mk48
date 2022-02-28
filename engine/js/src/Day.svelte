<script context="module">
    import {adminRequest, formatTimestamp, game, wildcardToUndefined} from './util.js';
    import {filterSummaryBlacklist} from './Summary.svelte';
    import {referrers} from './Referrers.svelte';
    import {userAgents} from './UserAgents.svelte';
</script>

<script>
    import Chart from './Chart.svelte';
    import Nav from './Nav.svelte';
    import {replace} from 'svelte-spa-router';

    export let params;
</script>

<Nav>
    <select on:change={e => replace(`/day/${e.target.value == "*" ? "*" : JSON.stringify({Referrer: e.target.value})}`)} value={'*'}>
        <option value={'*'}>Any</option>
        {#each $referrers as r}
            <option value={r[0]}>{r[0]}</option>
        {/each}
    </select>

    <select on:change={e => replace(`/day/${e.target.value == "*" ? "*" : JSON.stringify({UserAgentId: e.target.value})}`)} value={'*'}>
        <option value={'*'}>Any</option>
        {#each $userAgents as u}
            <option value={u[0]}>{u[0]}</option>
        {/each}
    </select>
</Nav>

{#await adminRequest({'RequestDay': {game_id: $game, filter: !params.filter || params.filter === '*' ? undefined : JSON.parse(params.filter)}})}
{:then data}
    <div class="charts">
        {#each filterSummaryBlacklist(Object.keys(data.DayRequested[0][1])) as key}
        <div class="chart">
            <p>{key}</p>
            <Chart
                data={data.DayRequested}
                filterBounds={false}
                logarithmic={false}
                points={true}
                x={point => point[0]}
                y={(typeof data.DayRequested[0][1][key] === 'number') ? [point => point[1][key]] : (data.DayRequested[0][1][key].length === 2 ? [point => point[1][key][0]] : data.DayRequested[0][1][key].map((ignored, i) => (point => point[1][key][i])))}
                fmtX={formatTimestamp}
            />
        </div>
        {/each}
    </div>
{:catch err}
    <p>{err}</p>
{/await}