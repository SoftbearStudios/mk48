<script context="module">
    import {writable} from 'svelte/store';
    import {adminRequest, DAY, formatTimestamp, game} from './util.js';
    export const periods = ['week', 'month', 'quarter'];
</script>

<script>
    import Chart from './Chart.svelte';
    import Nav from './Nav.svelte';
    import {replace} from 'svelte-spa-router';

    export let params = {};
</script>

<Nav>
    <select on:change={event => replace(`/series/${event.target.value}`)} value={params.period}>
        {#each periods as p}
            <option value={p}>{p}</option>
        {/each}
    </select>
</Nav>

<main>
    {#await adminRequest({'RequestSeries': {game_id: $game, period_start: Date.now() - {week: 7 * DAY, month: 30 * DAY, quarter: 90 * DAY}[params.period]}})}
    {:then data}
        {#if data.SeriesRequested.series.length > 0}
            <div class="charts">
                {#each Object.keys(data.SeriesRequested.series[0][1]) as key}
                <div class="chart">
                    <p>{key}</p>
                    <Chart
                        data={data.SeriesRequested.series}
                        filterBounds={false}
                        logarithmic={false}
                        points={true}
                        x={point => point[0]}
                        y={(typeof data.SeriesRequested.series[0][1][key] === 'number') ? [point => point[1][key]] : (data.SeriesRequested.series[0][1][key].length === 2 ? [point => point[1][key][0]] : data.SeriesRequested.series[0][1][key].map((ignored, i) => (point => point[1][key][i])))}
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