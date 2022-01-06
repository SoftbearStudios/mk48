<script context="module">
    import {adminRequest, game, round, wildcardToUndefined} from './util.js';

    const summaryBlacklist = [
        "arenas_cached", "connections", "cpu", "fps", "ram", "retention", "sessions_cached", "ups", "uptime"
    ];

    export const filterSummaryBlacklist = function(list) {
        return list.filter(item => summaryBlacklist.indexOf(item) < 0);
    }
</script>

<script>
    import Nav from './Nav.svelte';
    import {referrers} from './Referrers.svelte';
    import {userAgents} from './UserAgents.svelte';
    import {replace} from 'svelte-spa-router';

    export let params = {};
</script>

<Nav>
    <select on:change={event => replace(`/summary/${params.userAgent}/${event.target.value}`)} value={params.referrer || '*'}>
        <option value={'*'}>Any</option>
        {#each $referrers as r}
            <option value={r[0]}>{r[0]}</option>
        {/each}
    </select>

    <select on:change={event => replace(`/summary/${event.target.value}/${params.referrer}`)} value={params.userAgent || '*'}>
        <option value={'*'}>Any</option>
        {#each $userAgents as u}
            <option value={u[0]}>{u[0]}</option>
        {/each}
    </select>
</Nav>

<main>
    {#await adminRequest({'RequestSummary': {game_id: $game, referrer: wildcardToUndefined(params.referrer), user_agent_id: wildcardToUndefined(params.userAgent), period_start: null, period_stop: null}})}
    {:then data}
        <table>
            {#each Object.entries(data.SummaryRequested.metrics) as [key, value]}
                <tr>
                    <th>{key}</th>
                    <td class="value">
                        {#if typeof value.percent === 'number'}
                            {round(value.percent, 0)}%
                        {:else if typeof value.total === 'number'}
                            {value.total}
                        {:else if typeof value.average === 'number'}
                            {round(value.average, 3)}
                            {#if value.standard_deviation != 0}
                                ± {round(value.standard_deviation, 3)}
                            {/if}
                        {:else if Array.isArray(value.buckets)}
                            {JSON.stringify(value.buckets.map(n => round(n, 1)))}
                        {:else}
                            {JSON.stringify(value)}
                        {/if}
                    </td>
                    <td class="detail">
                        {#if typeof value.min === 'number' && typeof value.max === 'number' && (!('standard_deviation' in value) || value.standard_deviation != 0)}
                            (min: {round(value.min, 2)}, max: {round(value.max, 2)})
                        {:else if typeof value.total === 'number'}
                            (total: {value.total})
                        {:else if typeof value.underflow === 'number' && typeof value.overflow === 'number'}
                            (underflow: {round(value.underflow, 1)}, overflow: {round(value.overflow, 1)})
                        {/if}
                    </td>
                </tr>
            {/each}
        </table>
    {:catch err}
        <p>{err}</p>
    {/await}
</main>
