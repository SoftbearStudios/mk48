<script>
    import {adminRequest} from './util.js';
    import Nav from './Nav.svelte';

    let alias = "Server";
    let message = "";

    async function sendChat() {
        if (!alias || alias.length == 0 || !message || message.length == 0) {
            return;
        }
        const response = await adminRequest({SendChat: {alias, message}});
        if (response.ChatSent.sent) {
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
</script>

<Nav/>

<main>
    <form on:submit={sendChat}>
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
        <button>Send (to all arenas)</button>
    </form>
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
</style>