<script context="module">
    import {writable} from 'svelte/store';
    import {adminRequest, DAY, formatTimestamp, game} from './util.js';
    export const periods = ['week', 'month', 'quarter'];
    export const resolutions = ['hour', '6 hours', 'day', 'week'];
</script>

<script>
    import Chart from './Chart.svelte';
    import Nav from './Nav.svelte';
    import {replace} from 'svelte-spa-router';
    import {referrers} from './Referrers.svelte';
    import {regions} from './Regions.svelte';
    import {userAgents} from './UserAgents.svelte';

    export let params = {};
</script>

<Nav>
    <select on:change={e => replace(`/series/${params.period}/${params.resolution}/${e.target.value == "*" ? "*" : JSON.stringify({CohortId: parseInt(e.target.value)})}`)} value={'*'}>
        <option value={'*'}>Any</option>
        <option>1</option>
        <option>2</option>
        <option>3</option>
        <option>4</option>
    </select>

    <select on:change={e => replace(`/series/${params.period}/${params.resolution}/${e.target.value == "*" ? "*" : JSON.stringify({Referrer: e.target.value})}`)} value={'*'}>
        <option value={'*'}>Any</option>
        {#each $referrers as r}
            <option value={r[0]}>{r[0]}</option>
        {/each}
    </select>

    <select on:change={e => replace(`/series/${params.period}/${params.resolution}/${e.target.value == "*" ? "*" : JSON.stringify({RegionId: e.target.value})}`)} value={'*'}>
        <option value={'*'}>Any</option>
        {#each $regions as r}
            <option value={r[0]}>{r[0]}</option>
        {/each}
    </select>


    <select on:change={e => replace(`/series/${params.period}/${params.resolution}/${e.target.value == "*" ? "*" : JSON.stringify({UserAgentId: e.target.value})}`)} value={'*'}>
        <option value={'*'}>Any</option>
        {#each $userAgents as u}
            <option value={u[0]}>{u[0]}</option>
        {/each}
    </select>

    <select on:change={event => replace(`/series/${event.target.value}/${params.resolution}/${params.filter}`)} value={params.period}>
        {#each periods as p}
            <option value={p}>{p}</option>
        {/each}
    </select>

    <select on:change={event => replace(`/series/${params.period}/${event.target.value}/${params.filter}`)} value={params.resolution}>
        {#each resolutions as r}
            <option value={r}>{r}</option>
        {/each}
    </select>
</Nav>

<main>
    {#await adminRequest({'RequestSeries': {game_id: $game, filter: !params.filter || params.filter === '*' ? undefined : JSON.parse(params.filter), period_start: Date.now() - {week: 7 * DAY, month: 30 * DAY, quarter: 90 * DAY}[params.period], resolution: {hour: 1, '6 hours': 6, day: 24, week: 24 * 7}[params.resolution]}})}
    {:then data}
        {#if data.SeriesRequested.length > 0}
            <div class="charts">
                {#each Object.keys(data.SeriesRequested[0][1]) as key}
                <div class="chart">
                    <p>{key}</p>
                    <Chart
                        data={data.SeriesRequested}
                        filterBounds={false}
                        logarithmic={false}
                        points={true}
                        x={point => point[0]}
                        y={(typeof data.SeriesRequested[0][1][key] === 'number') ? [point => point[1][key]] : (data.SeriesRequested[0][1][key].length === 2 ? [point => point[1][key][0]] : data.SeriesRequested[0][1][key].map((ignored, i) => (point => point[1][key][i])))}
                        fmtX={formatTimestamp}
                    />
                </div>
                {/each}
            </div>
        {:else}
            <h2>No data.</h2>
        {/if}
    {:catch err}
        <p>{err}</p>
    {/await}
</main>