<script context="module">
    import {writable} from 'svelte/store';
    import {adminRequest} from './util.js';
</script>


<script>
    import Nav from './Nav.svelte';
    import {onMount} from 'svelte';

    let redirect = 0;
    let servers = [];

    onMount(async () => {
        const response = await adminRequest("RequestRedirect");
        if (response.RedirectRequested) {
            redirect = response.RedirectRequested == null ? 0 : response.RedirectRequested;
        }
    });

    onMount(async () => {
        const response = await adminRequest('RequestServers');
        if (response.ServersRequested) {
            servers = response.ServersRequested;
        }
    });

    async function setRedirect(val) {
        const response = await adminRequest({SetRedirect: val});
        if (response.RedirectSet !== undefined) {
            redirect = response.RedirectSet;
        }
    }

    async function restartHttpServer() {
        alert("Are you sure?");
        const response = await adminRequest("RestartHttpServer");
    }

    function checkmark(bool) {
        return bool ? '✔' : '✗';
    }
</script>

<Nav/>

<main>
    <table>
        <thead>
            <tr>
                <th>ID</th>
                <th>Region</th>
                <th>IP</th>
                <th>Home</th>
                <th>Reachable</th>
                <th>Healthy</th>
                <th>Players</th>
                <th>Client Hash</th>
                <th>Redirecting</th>
                <th>Redirect To</th>
            </tr>
        </thead>
        <tbody>
        {#each servers as server}
            <tr>
                <td>{server.server_id}</td>
                <td>{server.region_id == null ? '?' : server.region_id}</td>
                <td>{server.ip}</td>
                <td>{checkmark(server.home)}</td>
                <td>{checkmark(server.reachable)} ({server.rtt}ms)</td>
                <td>{checkmark(server.healthy)}</td>
                <td>{server.player_count == null ? '?' : server.player_count}</td>
                <td>{server.client_hash == null ? '?' : server.client_hash.toString().substring(0, 10)}</td>
                <td>{server.redirect_server_id == null ? '-' : server.redirect_server_id}</td>
                <td>
                    {#if server.server_id == redirect}
                        <button on:click={setRedirect.bind(null, null)}>Clear</button>
                    {:else}
                        <button on:click={setRedirect.bind(null, server.server_id)}>Set</button>
                    {/if}
                </td>
            </tr>
        {/each}
        </tbody>
    </table>
    <br>
    {#if redirect}
        <button on:click={setRedirect.bind(null, null)}>Clear Redirect {redirect}</button>
    {/if}
    <button on:click={restartHttpServer}>Restart Http Server</button>
</main>

<style>
    input {
        width: 50%;
    }
</style>