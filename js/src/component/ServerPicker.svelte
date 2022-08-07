<script>
    export let state;
    export let settingsStyle = false;
</script>

{#if state && state.servers}
    {#key JSON.stringify(state.servers) + state.serverId}
        <select class:settingsStyle value={state.serverId} on:change={e => window.rust && window.rust.handleChooseServerId(e.target.value == "null" ? null : parseInt(e.target.value))}>
            {#if state.serverId == null}
                <option value="null">Unknown Server</option>
            {/if}
            {#each state.servers as {serverId, region, players}}
                <option value={serverId}>Server {serverId} - {region} ({players} players)</option>
            {/each}
        </select>
    {/key}
{/if}

<style>
    select.settingsStyle {
	    background-color: #0075ff;
	    display: block;
	}
</style>