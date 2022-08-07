<script context="module">
    import {writable} from 'svelte/store';
    import {adminRequest, percent} from './util.js';

    export const regions = writable([]);

    async function load() {
        const result = await adminRequest('RequestRegions');
        regions.set(result.RegionsRequested);
    }

    load();
</script>

<script>
    import Nav from './Nav.svelte';
</script>

<Nav/>

<main>
    {#await adminRequest('RequestServerId')}
    {:then data}
        <h2>Server: {data.ServerIdRequested ? data.ServerIdRequested : 'localhost'}</h2>
    {:catch err}
    {/await}

    <table>
        {#each $regions as region}
            <tr>
                <th>{region[0]}</th>
                <td>{percent(region[1])}</td>
            </tr>
        {/each}
    </table>
</main>
