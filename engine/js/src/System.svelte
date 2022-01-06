<script>
    import {adminRequest} from './util.js';
    import Nav from './Nav.svelte';
    import {onMount} from 'svelte';

    let redirect = 0;

    onMount(async () => {
        const response = await adminRequest("RequestRedirect");
        if (response.RedirectRequested) {
            redirect = response.RedirectRequested.server_id == null ? 0 : response.RedirectRequested.server_id;
        }
    });

    async function setRedirect() {
        if (typeof redirect !== 'number') {
            return;
        }
        const response = await adminRequest({SetRedirect: {server_id: redirect === 0 ? null : redirect}});
        if (response.RedirectSet) {
            redirect = response.RedirectSet.server_id == null ? 0 : response.RedirectSet.server_id;
        }
    }
</script>

<Nav/>

<main>
    <form on:submit|preventDefault={setRedirect}>
        <p>0 means no redirect, anything else is a server id.</p>
        <input type="number" min="0" max="5" placeholder="Redirect" bind:value={redirect}/>
        <br/>
        <button>Set</button>
    </form>
</main>

<style>
    input {
        width: 50%;
    }
</style>