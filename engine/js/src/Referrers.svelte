<script context="module">
    import {writable} from 'svelte/store';
    import {adminRequest, percent} from './util.js';
    export const referrers = writable([]);

    async function load() {
        const result = await adminRequest('RequestReferrers');
        referrers.set(result.ReferrersRequested);
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
        {#each $referrers as referrer}
            <tr>
                <th>{referrer[0]}</th>
                <td>{percent(referrer[1])}</td>
            </tr>
        {/each}
    </table>
</main>
