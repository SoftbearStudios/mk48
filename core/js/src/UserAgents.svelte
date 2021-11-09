<script context="module">
    import {writable} from 'svelte/store';
    import {adminRequest, percent} from './util.js';

    export const userAgents = writable([]);

    async function load() {
        const result = await adminRequest('RequestUserAgents');
        userAgents.set(result.UserAgentsRequested.user_agents);
    }

    load();
</script>

<script>
    import Nav from './Nav.svelte';
</script>

<Nav/>

<main>
    <table>
        {#each $userAgents as userAgent}
            <tr>
                <th>{userAgent[0]}</th>
                <td>{percent(userAgent[1])}</td>
            </tr>
        {/each}
    </table>
</main>
