<script>
    // SPDX-FileCopyrightText: 2021 Softbear, Inc.
    // SPDX-License-Identifier: AGPL-3.0-or-later

    import Chart from './Chart.svelte';

    if (!localStorage.auth) {
        localStorage.auth = prompt("Enter auth code");
    }

    const headers = {'Content-Type': 'application/json'};
    const params = {auth: localStorage.auth};
    const requestGames = JSON.stringify({params, request: 'RequestGames'});
    const requestReferrers = JSON.stringify({params, request: 'RequestReferrers'});
    const requestUserAgents = JSON.stringify({params, request: 'RequestUserAgents'});
    const summary_blacklist = [
        "arenas_cached", "cpu", "ram", "retention", "sessions_cached", "uptime"
    ];

    function filter_summary_blacklist(list) {
        console.log(list); //@@
        return list.filter(item => summary_blacklist.indexOf(item) < 0);
    }

    function formatDetail(value) {
        let detail = '';
        if (Array.isArray(value)) {
            let rounded = value.map(i => round(i, 4));
            switch (value.length) {
                case 2:
                    detail = "of " + rounded[1];
                    break;
                case 4:
                    detail = "\u03C3=" + rounded[1] + ", min=" + rounded[2] + ", max=" + rounded[3];
                    break;
            }
        }

        return detail;
    }

    function formatTimestamp(timestamp) {
        const date = new Date(timestamp);
        const month = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][date.getMonth()];
        return `${month} ${date.getDate()} ${date.getHours()}:00`;
    }

    function formatValue(value) {
        if (Array.isArray(value)) {
            let rounded = value.map(i => round(i, 4));
            switch (value.length) {
                case 2:
                    return percent(rounded[0]);
                case 4:
                    return rounded[0];
                default:
                    return rounded.join(", ");
            }
        } else if (typeof value === 'number') {
            return round(value, 4);
        } else {
            return value;
        }
    }

    function percent(number) {
        return round(number * 100, 1) + "%";
    }

    function round(number, places) {
        const x = Math.pow(10, places);
        return Math.round(number * x) / x;
    }

    let games = [];
    let game_id;
    async function loadGames() {
        let result = await fetch("/admin/", {method: 'POST', body: requestGames, headers}).then(res => res.json());
        games = result.GamesRequested.games.map(([gameId, usage]) => gameId);
        console.log("Games: ", games);
        if (games.length != 0) {
            game_id = games[0];
        }
    }
    loadGames();

    let periods = [];
    let period_id;
    function initPeriods() {
        periods = ['week', 'month', 'quarter'];
        period_id = periods[0];
    }
    initPeriods();

    let referrers = [];
    let referrer_id;
    async function loadReferrers() {
        let result = await fetch("/admin/", {method: 'POST', body: requestReferrers, headers}).then(res => res.json());
        referrers = result.ReferrersRequested.referrers;
        console.log("Referrers: ", referrers);
        if (referrers.length != 0) {
            referrer_id = referrers[0][0];
        }
    }
    loadReferrers();

    let user_agents = [];
    let user_agent_id;
    async function loadUserAgents() {
        let result = await fetch("/admin/", {method: 'POST', body: requestUserAgents, headers}).then(res => res.json());
        user_agents = result.UserAgentsRequested.user_agents;
        console.log("UserAgents: ", user_agents);
        if (user_agents.length != 0) {
            user_agent_id = user_agents[0][0];
        }
    }
    loadUserAgents();

    let view = "summary";
</script>

<nav>
  <div class="navbtn {view == 'summary' ? 'selected' : ''}" on:click={() => view = 'summary'}>Summary</div>
  <div class="navbtn {view == 'day' ? 'selected' : ''}" on:click={() => view = 'day'}>Day</div>
  <div class="navbtn {view == 'referrers' ? 'selected' : ''}" on:click={() => view = 'referrers'}>Referrers</div>
  <div class="navbtn {view == 'user_agents' ? 'selected' : ''}" on:click={() => view = 'user_agents'}>User Agents</div>
  <div class="navbtn db {view == 'series' ? 'selected' : ''}" on:click={() => view = 'series'}>Series</div>
  <div class="selections">
  {#if !games}
      <div/>
  {:else}
      {#if view == 'day'}
          <select>
              <option value="any">any</option>
          {#each referrers as referrer}
              <option value={referrer[0]}>{referrer[0]}</option>
          {/each}
          </select>

          <select>
              <option value="any">any</option>
          {#each user_agents as user_agent}
              <option value={user_agent[0]}>{user_agent[0]}</option>
          {/each}
          </select>
      {/if}

      {#if view == 'series'}
          <select>
          {#each periods as period}
              <option value={period}>{period}</option>
          {/each}
          </select>
      {/if}

      <select>
      {#each games as game}
          <option value={game}>{game}</option>
      {/each}
      </select>
  {/if}
  </div>
</nav>

<main>
{#if !game_id}
    <h2>No data</h2>

{:else if view === 'day'}
    <h2>Day (Cache)</h2>
    {#await fetch("/admin/", {method: 'POST', body: JSON.stringify({params, request: {'RequestDay': {game_id}}}), headers}).then(res => res.json())}
    {:then data}
        <div class="charts">
            {#each filter_summary_blacklist(Object.keys(data.DayRequested.series[0][1])) as key}
                <div class="chart">
                    <p>{key}</p>
                    <Chart
                        data={data.DayRequested.series}
                        filterBounds={false}
                        logarithmic={false}
                        points={true}
                        x={point => point[0]}
                        y={(typeof data.DayRequested.series[0][1][key] === 'number') ? [point => point[1][key]] : (data.DayRequested.series[0][1][key].length === 2 ? [point => point[1][key][0]] : data.DayRequested.series[0][1][key].map((ignored, i) => (point => point[1][key][i])))}
                        fmtX={formatTimestamp}
                    />
                </div>
            {/each}
        </div>
    {:catch err}
        <p>{err}</p>
    {/await}

{:else if view === 'referrers'}
    <h2>Referrers (Cache)</h2>
    {#await fetch("/admin/", {method: 'POST', body: requestReferrers, headers}).then(res => res.json())}
    {:then data}
        <table>
            {#each data.ReferrersRequested.referrers as referrer}
            <tr>
                <th>{referrer[0]}</th>
                <td>{percent(referrer[1])}</td>
            </tr>
            {/each}
        </table>
    {:catch err}
        <p>{err}</p>
    {/await}

{:else if view === 'series'}
    <h2>Series</h2>
    {#await fetch("/admin/", {method: 'POST', body: JSON.stringify({params, request: {'RequestSeries': {game_id}}}), headers}).then(res => res.json())}
    {:then data}
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
    {:catch err}
        <p>{err}</p>
    {/await}

{:else if view === 'summary'}
    <h2>Summary (Cache)</h2>
    {#await fetch("/admin/", {method: 'POST', body: JSON.stringify({params, request: {'RequestSummary': {game_id, period_start: null, period_stop: null}}}), headers}).then(res => res.json())}
    {:then data}
        <table>
            {#each Object.entries(data.SummaryRequested.metrics) as [key, value]}
                <tr>
                    <th>{key}</th>
                    <td class="value">{formatValue(value)}</td>
                    <td class="detail">{formatDetail(value)}</td>
                </tr>
            {/each}
        </table>
    {:catch err}
        <p>{err}</p>
    {/await}

{:else if view === 'user_agents'}
    <h2>User Agents (Cache)</h2>
    {#await fetch("/admin/", {method: 'POST', body: requestUserAgents, headers}).then(res => res.json())}
    {:then data}
        <table>
            {#each data.UserAgentsRequested.user_agents as user_agent}
            <tr>
                <th>{user_agent[0]}</th>
                <td>{percent(user_agent[1])}</td>
            </tr>
            {/each}
        </table>
    {:catch err}
        <p>{err}</p>
    {/await}
{/if}

</main>

<style>
    div.chart {
        #background-color: black;
    }

    div.db {
        text-decoration: underline;
    }

    div.charts {
        display: grid;
        grid-gap: 10px 10px;
        grid-template-columns: repeat(4, 1fr);
    }

    div.navbtn {
        line-height: 2.1rem;
        padding-left: 1rem;
    }

    div.selected {
        color: white;
    }

    div.selections {
        position: absolute;
        right: 0;
    }

    h1 {
        text-transform: uppercase;
    }

    main {
        color: black;
        font-family: Verdana;
        margin: 0 auto;
        max-width: 240px;
        padding: 1em;
        text-align: center;
    }

    nav {
        background: darkslategrey;
        color: #ddd;
        display: flex;
        height: 2.1rem;
        left: 50%;
        position: relative;
        transform: translate(-50%, 0%);
        width: 80%;
    }

    select {
        background: darkslategrey;
        color: white;
    }

    table {
        border: 1px solid gray;
        margin: auto;
    }

    td {
        padding-left: 1em;
        padding-right: 1em;
        text-align: left;
    }

    td.detail {
        font-style: italic;
    }

    td.value {
        font-weight: bold;
    }

    th {
        background-color: darkslategrey;
        color: white;
        font-weight: normal;
        padding-left: 1em;
        padding-right: 1em;
        text-align: left;
    }

    @media (min-width: 640px) {
        main {
            max-width: none;
        }
    }
</style>
