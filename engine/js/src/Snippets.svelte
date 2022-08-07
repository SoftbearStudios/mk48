<script context="module">
    import {adminRequest} from './util.js';
</script>

<script>
    import Nav from './Nav.svelte';
    import {onMount} from 'svelte';

    let cohort = 'any', referrer, snippet;
    let snippets = [];

    onMount(requestSnippets);

    async function clearSnippet(cohort_id, referrer) {
        let parms = {
            cohort_id,
            referrer
        };
        const response = await adminRequest({ClearSnippet: parms});
        if (response == 'SnippetCleared') {
            // Re-reading the list isn't efficient but it works.
            await requestSnippets();
        } else {
            alert("Could not clear snippet.");
        }
    }

    function parse_cohort(cohort) {
        let trimText = parse_text(cohort);
        let n = trimText == 'any' ? null : parseInt(trimText);
        return n;
    }

    function parse_text(text) {
        let trimText = text ? text.trim() : null;
        return trimText && trimText.length != 0 ? trimText : null;
    }

    async function requestSnippets() {
        const response = await adminRequest('RequestSnippets');
        if (response.SnippetsRequested) {
            snippets = response.SnippetsRequested;
        }
    }

    async function sendSnippet() {
        let parms = {
            cohort_id: parse_cohort(cohort),
            referrer: parse_text(referrer),
            snippet: parse_text(snippet)
        }
        const response = await adminRequest({SetSnippet: parms});
        if (response == 'SnippetSet') {
            // Re-reading the list isn't efficient but it works.
            await requestSnippets();
        } else {
            alert("Could not set snippet.");
        }
    }
</script>

<Nav/>

<main>
    {#await adminRequest('RequestServerId')}
    {:then data}
        <h2>Server: {data.ServerIdRequested ? data.ServerIdRequested : 'localhost'}</h2>
    {:catch err}
    {/await}

    <form on:submit|preventDefault={() => sendSnippet()}>
        <table>
            <tr>
                <th>cohort</th>
                <td>
                  <select bind:value={cohort}>
                      <option default>any</option>
                      <option>1</option>
                      <option>2</option>
                      <option>3</option>
                      <option>4</option>
                </td>
            </tr>
            <tr>
                <th>referrer</th>
                <td>
                    <input type="text" bind:value={referrer}/>
                </td>
            </tr>
            <tr>
                <th>snippet</th>
                <td>
                    <textarea rows="10" type="text" bind:value={snippet}/>
                </td>
            </tr>
        </table>

        <button id="set">Set</button>
    </form>

    <table>
        <thead>
            <tr>
                <th>Cohort</th>
                <th>Referrer</th>
                <th>Snippet</th>
                <th></th>
            </tr>
        </thead>
        <tbody>
        {#each snippets as s}
            <tr>
                <td>{s.cohort_id ? s.cohort_id : "any"}</td>
                <td>{s.referrer ? s.referrer : ""}</td>
                <td>{s.snippet}</td>
                <td><button on:click={() => clearSnippet(s.cohort_id, s.referrer)}>Clear</button></td>
            </tr>
        {/each}
        </tbody>
    </table>
</main>

<style>
    button#set {
        margin-bottom: 4rem;
        margin-top: 1rem;
    }
</style>
