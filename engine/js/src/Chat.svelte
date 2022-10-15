<script>
    import {adminRequest} from './util.js';
    import Nav from './Nav.svelte';
    import {onMount} from 'svelte';

    let alias = "Server";
    let message = "";

    let players = [];
    onMount(async () => {
        const response = await adminRequest('RequestPlayers');
        if (response.PlayersRequested) {
            players = response.PlayersRequested;
        }
    });

    async function overrideAlias(playerId, previousAlias) {
        const alias = prompt("Override alias to? (ok to confirm)", previousAlias);
        const response = await adminRequest({OverridePlayerAlias: {player_id: playerId, alias}});
        if (response.PlayerAliasOverridden) {
            const player = players.find(p => p.player_id == playerId);
            if (player != null) {
                player.alias = response.PlayerAliasOverridden;

                // Reactivity
                players = players;
            }
        }
    }

    async function overrideModerator(playerId, moderator) {
        const response = await adminRequest({OverridePlayerModerator: {player_id: playerId, moderator}});
        if (typeof response.PlayerModeratorOverridden === 'boolean') {
            const player = players.find(p => p.player_id == playerId);
            if (player != null) {
                player.moderator = response.PlayerModeratorOverridden;

                // Reactivity
                players = players;
            }
        }
    }

    async function mute(playerId, minutes) {
        const response = await adminRequest({MutePlayer: {player_id: playerId, minutes}});
        if (typeof response.PlayerMuted === 'number') {
            const player = players.find(p => p.player_id == playerId);
            if (player != null) {
                player.mute = response.PlayerMuted;

                // Reactivity
                players = players;
            }
        }
    }

    async function restrict(playerId, minutes) {
        const response = await adminRequest({RestrictPlayer: {player_id: playerId, minutes}});
        if (typeof response.PlayerRestricted === 'number') {
            const player = players.find(p => p.player_id == playerId);
            if (player != null) {
                player.restriction = response.PlayerRestricted;

                // Reactivity
                players = players;
            }
        }
    }

    async function sendChat(player_id) {
        if (!alias || alias.length == 0 || !message || message.length == 0) {
            return;
        }
        const response = await adminRequest({SendChat: {alias, message, player_id}});
        if (response == "ChatSent") {
            message = "";
        }
    }

    const PRESETS = {
        "Custom Message": "",
        "Phasing Out": "Attention players: This server is being phased out. You may continue playing as long as you want, but no new players will join.",
        "Phasing Out Reminder": "Attention players: This is a reminder that this server is being phased out. You may continue playing as long as you want, but no new players will join.",
        "Discord Tip": "Tip: You can join the game's discord at https://discord.gg/YMheuFQWTX",
        "GitHub Tip": "Tip: You can view the game's code at https://github.com/SoftbearStudios/mk48",
        "Mute Tip": "Tip: You can mute another player by right-clicking their name in chat, and pressing the mute button.",
        "Bot Name Tip": "Tip: Bots only use names of famous robots, however nothing stops real players from using those same names.",
        "Update ETA": "Tip: The developers, despite other obligations, are working hard on improving the game. Suggestions are always appreciated. However, there are no ETA's for updates.",
        "Level 10 Plan": "Tip: The plan is for there to be 10 levels. Getting there will take some time, so please be patient!",
        "Fake Developer Warning": "Tip: Beware of anyone claiming to be a developer or admin. There are only two (real) developers.",
        "Profanity Warning": "Please refrain from profanity. Patching the filter takes developer time away from adding content.",
        "Disrespect Warning": "Please keep the chat respectful. Moderating takes developer time away from adding content.",
        "Toxicity Warning": "Please do your best to welcome new players (by not sinking them right after they spawn).",
        "Privacy Warning": "Never share your full name, age, or other personal information if you are under 13 years old.",
        "Thank You": "Thank you for playing Mk48.io!",
        "Be Patient": "Thank you for your patience while the developers patch any remaining bugs.",
        "Be Specific": "Please be specific when reporting issues, so the developers have a chance of fixing them!",
        "Aware Issue": "The developers are aware of an ongoing issue and are looking into a solution. Please be patient!",
        "Feedback Noted": "Your feedback has been noted!",
        "Server Clarification": "The developers may or may not be here, but they asked me to send an automated message."
    }

    /// Replaces null with question mark.
    function maybe(val) {
        return val == null ? '?' : val;
    }

    function checkmark(bool) {
        return bool ? '✔' : '✗';
    }
</script>

<Nav/>

<main>
    <form on:submit|preventDefault={() => sendChat()}>
        <input type="text" minlength="1" placeholder="Alias" bind:value={alias}/>
        <br/>
        <select on:change={event => message = PRESETS[event.target.value]}>
            {#each Object.entries(PRESETS) as [name, value]}
                <option>{name}</option>
            {/each}
        </select>
        <br/>
        <textarea type="text" minlength="1" placeholder="Message" bind:value={message}/>
        <br/>
        <button>Send (to all players)</button>
    </form>

    <br>
    <br>

    <table>
        <thead>
            <tr>
                <th>ID</th>
                <th>Alias</th>
                <th>Team ID</th>
                <th>Discord ID</th>
                <th>Mod</th>
                <th>Score</th>
                <th>Plays</th>
                <th>Region ID</th>
                <th>IP</th>
                <th>FPS</th>
                <th>RTT</th>
                <th>Msgs.</th>
                <th>Inapp.</th>
                <th>Reports</th>
                <th>Restrict</th>
                <th>Mute</th>
                <th>Chat</th>
                <th>Zeus</th>
            </tr>
        </thead>
        <tbody>
            {#each players as player}
                <tr>
                    <td>{player.player_id}</td>
                    <td class='clickable' on:click={() => overrideAlias(player.player_id, player.alias)}>{player.alias}</td>
                    <td>{player.team_id == null ? '-' : player.team_id}</td>
                    <td>{player.discord_id == null ? '-' : player.discord_id}</td>
                    <td class='clickable' on:click={() => overrideModerator(player.player_id, player.moderator ? false : true)}>{checkmark(player.moderator)}</td>
                    <td>{player.score}</td>
                    <td>{player.plays}</td>
                    <td>{maybe(player.region_id)}</td>
                    <td>{player.ip_address}</td>
                    <td>{maybe(player.fps)}</td>
                    <td>{maybe(player.rtt)}</td>
                    <td>{player.messages}</td>
                    <td>{player.inappropriate_messages}</td>
                    <td>{player.abuse_reports}</td>
                    <td>
                        <select class="mod" on:change|preventDefault={e => restrict(player.player_id, parseInt(e.target.value))} value={player.restriction}>
                            <option disabled>{player.restriction}</option>
                            <option>0</option>
                            <option>5</option>
                            <option>10</option>
                            <option>30</option>
                            <option>60</option>
                            <option>360</option>
                        </select>
                    </td>
                    <td>
                        <select class="mod" on:change|preventDefault={e => mute(player.player_id, parseInt(e.target.value))} value={player.mute}>
                            <option disabled>{player.mute}</option>
                            <option>0</option>
                            <option>5</option>
                            <option>10</option>
                            <option>30</option>
                            <option>60</option>
                            <option>360</option>
                        </select>
                    </td>
                    <td>
                        <button on:click={() => sendChat(player.player_id)}>Send</button>
                    </td>
                    <td>
                        <button>Smite</button>
                    </td>
                </tr>
            {/each}
        </tbody>
    </table>
</main>

<style>
    input, select, textarea {
        width: 50%;
    }

    textarea {
        height: 100px;
    }

    select {
        background: initial;
        color: initial;
    }

    select.mod {
        width: max-content;
    }

    .clickable {
        cursor: pointer;
    }
</style>