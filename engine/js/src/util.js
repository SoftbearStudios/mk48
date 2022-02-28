import {writable} from 'svelte/store';

while (localStorage.auth == null || localStorage.auth == "null" || localStorage.auth === "") {
    localStorage.auth = prompt("Enter auth code");
}

export const DAY = 1000 * 60 * 60 * 24;
export const auth = localStorage.auth;
export const headers = {'Content-Type': 'application/json'};

// Cache of promises.
const disableCache = true;
const cache = {};

export const adminRequest = async function(request) {
    const body = JSON.stringify({auth, request});
    if (disableCache || !(body in cache)) {
        cache[body] = fetch("/admin/", {method: 'POST', body, headers}).then(res => res.json());
    }
    return await cache[body];
}

export const games = writable([]);
export const game = writable(null);

async function loadGames() {
    const result = await adminRequest('RequestGames');
    const gameIds = result.GamesRequested.map(([gameId, usage]) => gameId);
    games.set(gameIds);
    if (gameIds.length > 0) {
        game.set(gameIds[0]);
    }
}
loadGames();

export const formatTimestamp = function(timestamp) {
    const date = new Date(timestamp);
    const month = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][date.getMonth()];
    return `${month} ${date.getDate()} ${date.getHours()}:00`;
}

export const percent = function(number) {
    return round(number * 100, 1) + "%";
}

export const round = function(number, places) {
    const x = Math.pow(10, places);
    return Math.round(number * x) / x;
}

export const wildcardToUndefined = function(param) {
     if (!param || param === '*') {
         return undefined;
     }
     return param;
 }