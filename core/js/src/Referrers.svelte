<script context="module">
    import {writable} from 'svelte/store';
    import {adminRequest, percent} from './util.js';
    export const referrers = writable([]);

    async function load() {
        const result = await adminRequest('RequestReferrers');
        referrers.set(result.ReferrersRequested.referrers);
    }

    load();
</script>

<script>
    import Nav from './Nav.svelte';
</script>

<Nav/>

<main>
    <table>
        {#each $referrers as referrer}
            <tr>
                <th>{referrer[0]}</th>
                <td>{percent(referrer[1])}</td>
            </tr>
        {/each}
    </table>
</main>
