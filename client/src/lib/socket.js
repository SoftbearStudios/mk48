// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

import {get, writable} from 'svelte/store';
import storage from '../util/storage.js';
import {getInvite} from './teams.js';

export const connected = writable(null); // null = never, true = yes, false = not any more
export const entityID = writable(null);
export const contacts = writable({});
export const chats = writable([]);
export const leaderboard = writable([]);
export const playerID = writable(null);
export const teamInvite = writable(null);
export const teamMembers = writable([]);
export const teamJoinRequests = writable([]);
export const worldRadius = writable(500);
export const deathMessage = writable(null);
export const terrain = writable(null);
export const serverID = writable(null);

let socket = null;
let connecting = false;

// Connect opens a websocket if needed, and calls the callback when a socket is sopen
export async function connect(callback) {
	if (connecting || (socket && socket.readyState === WebSocket.CONNECTING)) {
		return;
	}

	if (!socket || socket.readyState !== WebSocket.OPEN) {
		connecting = true;
		let server = 'ws://localhost:8192/ws';
		if (typeof storage.server === 'string') {
			server = storage.server;
		} else if (!location.host.startsWith('localhost') || storage.noLocalServer) {
			server = null;
			const region = 'us-east-1';
			const slots = 4;

			const inviteServerIndex = parseInviteServerIndex(getInvite());

			// -1 stands for inviteServerIndex
			for (let iter = inviteServerIndex ? -1 : 0; iter < slots; iter++) {
				const i = iter == -1 ? inviteServerIndex : iter;
				const httpServer = `https://cf-${region}-${i}.mk48.io`;
				try {
					const response = await fetch(httpServer);
					if (!response.ok) {
						throw new Error('response not ok');
					}
					server = `wss://cf-${region}-${i}.mk48.io/ws`;
					serverID.set(`cf-${region}-${i}`);
					const json = await response.json();
					console.log(`server ${region} #${i}: ${json}`);
					if (json.players > 40) {
						console.log(`server ${region} #${i} is full, looking for others`);
					} else {
						break;
					}
				} catch (err) {
					console.log(`Could not connect to ${httpServer}`);
				}
			}
		}
		connecting = false;

		socket = new WebSocket(server);

		socket.onopen = () => {
			console.log("socket - Connected to server.");
			connected.set(true);
			callback && callback();
		};

		socket.onclose = event => {
			socket = null;
			connected.set(false);
			contacts.set({});
			chats.set([]);
			entityID.set(null);
			serverID.set(null);
			console.log(`socket - Disconnected from server with code ${event.code} due to '${event.reason}'.`);
		};

		socket.onmessage = messageRaw => {
			const message = JSON.parse(messageRaw.data);
			switch (message.type) {
				case "update":
					entityID.set(message.data.entityID);
					playerID.set(message.data.playerID);
					contacts.set(message.data.contacts);
					if (message.data.teamInvite) {
						teamInvite.set(`${get(serverID)}/${message.data.teamInvite}`);
					} else {
						teamInvite.set(null);
					}
					teamMembers.set(message.data.teamMembers);
					teamJoinRequests.set(message.data.teamJoinRequests);
					if (message.data.chats || message.data.teamChats) {
						chats.update(cs => {
							if (message.data.chats) {
								cs = cs.concat(message.data.chats);
							}
							if (message.data.teamChats) {
								cs = cs.concat(message.data.teamChats.map(chat => ({...chat, teamOnly: true})));
							}
							if (cs.length > 10) {
								cs = cs.slice(cs.length - 10);
							}
							return cs;
						});
					}
					worldRadius.set(message.data.worldRadius)
					deathMessage.set(message.data.deathMessage);
					if (message.data.terrain) {
						message.data.terrain.data = readTerrain(message.data.terrain.data, message.data.terrain.length);

						terrain.set(message.data.terrain);
					}
					break;
				case "leaderboard":
					leaderboard.set(message.data.leaderboard);
					break;
			}
		};
	} else if (callback) {
		callback();
	}
}

export function disconnect() {
	if (socket && socket.readyState === WebSocket.CONNECTING || socket.readyState === WebSocket.OPEN) {
		socket.close();
	}
}

export function send(type, data = {}) {
	if (!socket || socket.readyState !== WebSocket.OPEN) {
		return;
	}
	socket.send(JSON.stringify({type, data}));
}

function readTerrain(base64, length) {
	const str = window.atob(base64);
	const len = str.length;

	const bytes = new Uint8Array(length);
	for (let i = 0, j = 0; i < len; i++) {
		let b = str.charCodeAt(i);
		while (true) {
			const count = b & 0b00001111
			bytes[j] = b & 0b11110000
			j++
			if (count > 0) {
				b--
			} else {
				break
			}
		}
	}

	const imageBytes = new Uint8Array(length * 4);
	for (let i = 0; i < length; i++) {
		imageBytes[i * 4 + 3] = bytes[i];
	}
	return imageBytes;
}

function parseInviteServerIndex(invite) {
	try {
		const segments = invite.split('/');
		const serverID = segments[0];
		const serverIDSegments = serverID.split('-');
		return parseInt(serverIDSegments[serverIDSegments.length - 1]);
	} catch (err) {
		return null;
	}
}
