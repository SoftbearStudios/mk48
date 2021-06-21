<!--
	SPDX-FileCopyrightText: 2021 Softbear, Inc.
	SPDX-License-Identifier: AGPL-3.0-or-later
-->

<script>
	import {hasWebP, getMouseButton, isMobile} from '../util/compatibility.js';
	import {addTransforms, angleDiff, clamp, clampMagnitude, dist, mapRanges} from '../util/math.js';
	import Ship, {getArmamentType} from '../lib/Ship.svelte';
	import Chat from '../lib/Chat.svelte';
	import Instructions from '../lib/Instructions.svelte';
	import Leaderboard from '../lib/Leaderboard.svelte';
	import Status from '../lib/Status.svelte';
	import SplashScreen from '../lib/SplashScreen.svelte';
	import Hint from '../lib/Hint.svelte';
	import Sidebar from '../lib/Sidebar.svelte';
	import Teams from '../lib/Teams.svelte';
	import t from '../lib/translation.js';
	import Upgrades, {canUpgrade} from '../lib/Upgrades.svelte';
	import {drawHud, THROTTLE_END, THROTTLE_START} from '../lib/hud.js';
	import {recycleParticle, updateParticles} from '../lib/particles.js';
	import {applyVelocity} from '../lib/physics.js';
	import {connect, connected, disconnect, send, contacts as socketContacts, entityID as socketEntityID, terrain, leaderboard, worldRadius} from '../lib/socket.js';
	import backgroundShader from '../lib/background.js';
	import {startRecording, stopRecording} from '../lib/recording.js';
	import {volume} from '../lib/settings.js';
	import {onMount, onDestroy} from 'svelte'

	// Spritesheet data
	import entitiesTPS from '../data/entities.tps.json';
	import extrasTPS from '../data/extras.tps.json';

	// Entity/sound/etc. Data
	import entityData from '../data/entities.json';
	import soundData from '../data/sounds.json';

	let canvas, chatRef, shipRef, heightFract, widthFract, viewport;
	$: height = Math.floor(heightFract);
	$: width = Math.floor(widthFract);

	let mouse = {x: 0, y: 0, touch: false, leftDown: 0, rightDown: false, click: false};
	let keyboard = {shoot: false, forward: false, backward: false, right: false, left: false, stop: false}; // what keys are down
	let keyEvent = false;
	let overlay = {};
	let viewportPositionCache = {x: 0, y: 0};
	let armamentSelection;
	let active; // active sensors
	let altitudeTarget;
	let lastActive; // last active sent to server
	let lastAltitudeTarget; // last altitudeTarget sent to server
	let lastSend = 0; // secondsTotal of last manual/aim
	let perf = 0.5; // performance level in interval [0,1]

	// Global leaderboard
	let globalLeaderboard = null;

	// To debug global leaderboard locally
	// globalLeaderboard = JSON.parse(`{"single/all":[{"name":"test1","score":1234}], "single/week":[{"name":"test1","score":1234}], "single/day":[{"name":"test1","score":1234}]}`);

	// Tutorial status
	let timesMoved = 0;
	let weaponsFired = 0;
	let instructZoom = true; // when player figures out how to zoom, set to false
	let recording = false;

	const MOUSE_CLICK_MILLIS = 180;
	const DEFAULT_ZOOM = 2.5;
	const SECONDS_PER_SEND = 0.1;
	const NAME_SCALE = 6;

	function mouseDownNotClick() {
		return (mouse.leftDown > 0 && (Date.now() - mouse.leftDown) > MOUSE_CLICK_MILLIS) || mouse.rightDown;
	}

	let contacts = {};

	// make this a local variable, not a store, for convenience
	$: localEntityID = $socketEntityID;

	function onStart(data) {
		connect(() => {
			send('spawn', data);
		});
	}

	function setOverlay(name, value) {
		overlay[name] = value;
		overlay = overlay; // reactivity
	}

	// May be called anywhere (not just component init)
	const customOnDestroyFuncs = [];
	let destroying = false;
	function customOnDestroy(func) {
		if (destroying) {
			func();
		} else {
			customOnDestroyFuncs.push(func);
		}
	}

	onDestroy(() => {
		console.log(`on destroy (${customOnDestroyFuncs.length})`);
		destroying = true;
		for (const func of customOnDestroyFuncs) {
			func();
		}
	});

	onMount(async () => {
		const PIXI = await import('pixi.js');
		const {Viewport} = await import('pixi-viewport');
		const {sound: Sounds} = await import('@pixi/sound');

		if (destroying) {
			return;
		}

		PIXI.settings.MIPMAP_TEXTURES = PIXI.MIPMAP_MODES.ON;

		const app = new PIXI.Application({
			view: canvas,
			width,
			height,
			antialias: true,
			resolution: 1,
			sharedTicker: true,
			backgroundColor: 0x10ccff
		});

		// Relatively meaningless, as does not seem to limit size
		const WORLD_SIZE = 10;

		viewport = new Viewport({
			screenWidth: width,
			screenHeight: height,
			worldWidth: WORLD_SIZE,
			worldHeight: WORLD_SIZE,
			interaction: app.renderer.plugins.interaction
		});
		app.stage.addChild(viewport);

		// Zoom in a bit
		viewport.scale.set(DEFAULT_ZOOM);
		// Enable scroll and pinch zoom
		viewport.wheel({smooth: true});
		viewport.pinch();
		const zoomHandler = viewport.on('zoomed', () => {
			instructZoom = false;
			viewport.off('zoomed', zoomHandler);
		});

		// Load spritesheets (synchronously)
		const spritesheetExt = hasWebP() ? 'webp' : 'png';
		const entitiesTexture = PIXI.Texture.from(`/entities.${spritesheetExt}`);
		const extrasTexture = PIXI.Texture.from(`/extras.${spritesheetExt}`);
		const entitiesSpritesheet = new PIXI.Spritesheet(entitiesTexture, entitiesTPS);
		const extrasSpritesheet = new PIXI.Spritesheet(extrasTexture, extrasTPS);
		entitiesSpritesheet.parse(() => {});
		extrasSpritesheet.parse(() => {});

		// Load sounds
		for (const name of soundData) {
			Sounds.add(name, {
				url: `/sounds/${name}.mp3`,
				preload: name !== 'ocean'
			});
		}

		// Calling play twice before a sound is loaded will crash (see
		// https://github.com/pixijs/sound/issues/71)
		function playSoundSafe(name, options) {
			const sound = Sounds.find(name);
			if (!sound.isPlayable) {
				console.warn('sound not playable')
				return;
			}
			sound.play(options);
		}

		// Only playing this once, so no need for playSoundSafe
		Sounds.play('ocean', {loop: true, volume: 0.25});

		volume.subscribe(val => {
			if (val > 0) {
				Sounds.unmuteAll();
				Sounds.volumeAll = val;
			} else {
				Sounds.muteAll();
			}
		});

		// Background (water + land)
		const background = new PIXI.Filter(null, backgroundShader);
		const backgroundContainer = new PIXI.Container();
		viewport.addChild(backgroundContainer);
		backgroundContainer.filterArea = app.screen;
		backgroundContainer.filters = [background];
		customOnDestroy(worldRadius.subscribe(newRadius => {
			background.uniforms.iBorderRange = newRadius;
		}));

		// For unknown reasons, PIXI.js does not render things in the order
		// they were added to viewport. Some things must be added in reverse.
		const smokeParticles = new PIXI.ParticleContainer(16384, {
			scale: true,
			position: true,
			alpha: true,
			autoResize: true
		});
		viewport.addChild(smokeParticles);

		const explosions = new PIXI.Container();
		viewport.addChild(explosions);

		const hud = new PIXI.Graphics();
		viewport.addChild(hud);

		const wakeParticles = new PIXI.ParticleContainer(16384, {
			scale: true,
			position: true,
			alpha: true,
			autoResize: true
		});
		viewport.addChild(wakeParticles);

		const entityContainer = new PIXI.Container();
		viewport.addChild(entityContainer);

		const splashes = new PIXI.Container();
		viewport.addChild(splashes);

		// Keep a map of entityID to sprite
		const entitySprites = {};

		// Removes an entity sprite, destroying its children where applicable
		function removeSprite(entityID, sprite) {
			// The commented lines were observed to cause issues in the past

			if (sprite.nameText) {
				viewport.removeChild(sprite.nameText);
				//sprite.nameText.destroy();
			}
			if (sprite.healthBar) {
				viewport.removeChild(sprite.healthBar);
				sprite.healthBar.destroy();
			}
			if (sprite.triangle) {
				viewport.removeChild(sprite.triangle);
				sprite.triangle.destroy();
			}
			entityContainer.removeChild(sprite);
			//entitySprites[entityID].destroy();
			delete(entitySprites[entityID]);
		}

		function volumeAt(point) {
			const distance = dist(viewport.center, point);
			return 1 / (1 + 0.05 * distance);
		}

		function reconcileContacts(newContacts) {
			contacts = newContacts;

			for (const entityID of Object.keys(newContacts)) {
				const entity = newContacts[entityID];

				// Can be undefined (for unkown contacts)
				const currentEntityData = entityData[entity.type];

				let sprite = entitySprites[entityID];
				const isNew = !sprite || entity.type != sprite.type;

				if (isNew) {
					if (sprite) {
						removeSprite(entityID, sprite);
					}

					if (entityID === localEntityID) {
						playSoundSafe('upgrade');
					}

					const texture = entity.type && entity.type in entitiesSpritesheet.textures ? entitiesSpritesheet.textures[entity.type] : extrasSpritesheet.textures.contact;
					sprite = PIXI.Sprite.from(texture);
					entitySprites[entityID] = sprite;

					sprite.type = entity.type;
					sprite.uncertainty = entity.uncertainty;

					sprite.anchor.set(0.5);
					sprite.height = currentEntityData ? currentEntityData.width : 15;
					sprite.width = currentEntityData ? currentEntityData.length : 15;
					entityContainer.addChild(sprite);

					if (currentEntityData) {
						// Sounds
						const volume = volumeAt(entity.position);
						const direction = Math.atan2(entity.position.y - viewport.center.y, entity.position.x - viewport.center.x);
						const inbound = Math.abs(angleDiff(entity.direction, direction + Math.PI)) < Math.PI / 2;
						switch (currentEntityData.kind) {
							case 'boat':
								if (!entity.friendly && inbound && localEntityID) {
									playSoundSafe('alarmSlow', {volume: 0.25 * Math.max(volume, 0.5)});
								}
							case 'weapon':
								switch (currentEntityData.subkind) {
									case 'torpedo':
										if (entity.friendly) {
											playSoundSafe('torpedoLaunch', {volume: Math.min(volume, 0.5)});
											setTimeout(() => {
												playSoundSafe('splash', {volume});
											}, 100);
										}
										if (currentEntityData.sensors && currentEntityData.sensors.sonar && currentEntityData.sensors.sonar.range) {
											setTimeout(() => {
												playSoundSafe('sonar3', {volume});
											}, entity.friendly ? 1000 : 0);
										}
										break;
									case 'missile':
									case 'rocket':
										if (!entity.friendly && inbound && localEntityID) {
											playSoundSafe('alarmFast', {volume: Math.max(volume, 0.5)});
										}
										// Fallthrough
									case 'sam':
										playSoundSafe('rocket', {volume});
										break;
									case 'depthCharge':
									case 'mine':
										playSoundSafe('splash', {volume});
										if (!entity.friendly && localEntityID) {
											playSoundSafe('alarmSlow', {volume: Math.max(volume, 0.5)});
										}
										break;
									case 'shell':
										playSoundSafe(`shell`, {volume: volume * mapRanges(currentEntityData.length, 0.5, 1.5, 0.5, 1, true)});
										break;
								}
								break;
							case 'aircraft':
								if (!entity.friendly && inbound && localEntityID) {
									playSoundSafe('alarmSlow', {volume: 0.1 * Math.max(volume, 0.5)});
								}
								break;
							case 'decoy':
								playSoundSafe('sonar3', {volume});
								break;
						}

						const turrets = currentEntityData.turrets;

						if (turrets) {
							sprite.turrets = [];

							for (let t = 0; t < turrets.length; t++) {
								const turret = turrets[t];

								let turretContainer;
								if (turret.type) {
									turretContainer = PIXI.Sprite.from(entitiesSpritesheet.textures[turret.type]);
									//turretContainer.anchor.set(entityData[turret.type].positionForward / entityData[turret.type].width, 0.5);
									turretContainer.height = entityData[turret.type].width / sprite.scale.y;
									turretContainer.width = entityData[turret.type].length / sprite.scale.x;
									turretContainer.anchor.set(0.5 - (entityData[turret.type].positionForward || 0) / entityData[turret.type].length, 0.5 - (entityData[turret.type].positionSide || 0) / entityData[turret.type].width);
								} else {
									turretContainer = new PIXI.Container();
								}
								turretContainer.position.set(turret.positionForward / sprite.scale.x, turret.positionSide / sprite.scale.y);
								turretContainer.rotation = turret.angle || 0;
								sprite.addChild(turretContainer);

								sprite.turrets[t] = turretContainer;
							}
						}

						const armaments = currentEntityData.armaments;

						if (armaments) {
							sprite.armaments = [];

							for (let a = 0; a < armaments.length; a++) {
								const armament = armaments[a];

								// For now, vertically-launched armaments are hidden
								// TODO: Create top-down sprites
								if (armament.hidden || armament.vertical || !(entity.external || entity.friendly)) {
									continue;
								}

								const armamentSprite = PIXI.Sprite.from(entitiesSpritesheet.textures[armament.type]);
								armamentSprite.position.set((armament.positionForward || 0) / sprite.scale.x, (armament.positionSide || 0) / sprite.scale.y);
								armamentSprite.anchor.set(0.5);
								armamentSprite.rotation = armament.angle || 0;
								if (armament.turret != undefined) {
									sprite.turrets[armament.turret].addChild(armamentSprite);
								} else {
									sprite.addChild(armamentSprite);
								}

								armamentSprite.height = entityData[armament.type].width / sprite.scale.y;
								armamentSprite.width = entityData[armament.type].length / sprite.scale.x;

								sprite.armaments[a] = armamentSprite;
							}
						}
					}
				}

				// Markers/nametags
				if (currentEntityData) {
					let oldColor = null, newColor = null;

					switch (currentEntityData.kind) {
					case 'aircraft':
					case 'decoy':
					case 'weapon':
						newColor = entity.friendly ? 0x3aff8c : 0xe74c3c;

						if (sprite.triangle) {
							oldColor = sprite.triangle.tint;
						}

						if (newColor !== oldColor) {
							if (!sprite.triangle) {
								sprite.triangle = new PIXI.Sprite(extrasSpritesheet.textures.triangle);
								sprite.triangle.anchor.set(0.5);
								viewport.addChild(sprite.triangle);
							}

							sprite.triangle.tint = newColor;
						}
						break;
					case 'boat':
						let newName = null;
						if (entity.name) {
							newName = entity.team ? `[${entity.team}] ${entity.name}` : entity.name;
						}
						newColor = entity.friendly ? 0x3aff8c : 0xffffff;

						let oldName = null;
						if (sprite.nameText) {
							oldName = sprite.nameText.text;
							oldColor = sprite.nameText.fillColor;
						}

						if (newName !== oldName || newColor !== oldColor) {
							if (sprite.nameText) {
								sprite.removeChild(sprite.nameText);
								sprite.nameText.destroy();
								delete sprite.nameText;
							}

							if (newName) {
								sprite.nameText = new PIXI.Text(newName, {fontFamily: 'Arial', fontSize: 32, align: 'center', fill: newColor});
								sprite.nameText.fillColor = newColor; // for our purposes, not PIXI
								sprite.nameText.anchor.set(0.5);
								sprite.nameText.alpha = 0.75;
								sprite.nameText.scale.set(0.1);
								viewport.addChild(sprite.nameText);
							}
						}

						if (sprite.nameText) {
							// Quantize health to avoid frequent GUI updates
							const health = Math.ceil((1 - (entity.damage || 0)) * 10) / 10;

							if (currentEntityData.kind === 'boat' && (!sprite.healthBar || sprite.healthBar.health !== health || newColor !== oldColor)) {
								if (!sprite.healthBar) {
									sprite.healthBar = new PIXI.Graphics();
									viewport.addChild(sprite.healthBar);
								} else if (entityID === localEntityID && health < sprite.healthBar.health) {
									playSoundSafe('damage');
								}

								sprite.healthBar.health = health;

								const BAR_LENGTH = 15;
								const BAR_HEIGHT = 1;

								sprite.healthBar.clear();
								sprite.healthBar.beginFill(0xaaaaaa, 0.5);
								sprite.healthBar.drawRect(-BAR_LENGTH / 2, -BAR_HEIGHT / 2, BAR_LENGTH, BAR_HEIGHT);
								sprite.healthBar.endFill();

								sprite.healthBar.beginFill(newColor, 0.5);
								sprite.healthBar.drawRect(-BAR_LENGTH / 2, -BAR_HEIGHT / 2, health * BAR_LENGTH, BAR_HEIGHT);
								sprite.healthBar.endFill();

								sprite.healthBar.visible = entity.damage ? true : false;
							}
						} else if (sprite.healthBar) {
							viewport.removeChild(sprite.healthBar);
							sprite.healthBar.destroy();
							delete sprite.healthBar;
						}

						if (entityID === localEntityID) {
							setOverlay('score', entity.score);

							// If player has a good score, prompt them before leaving page
							if (entity.score >= 50) {
								window.onbeforeunload = function() {
									return true;
								};
							} else {
								window.onbeforeunload = null;
							}
						}
						break;
					}
				}

				if (entity.altitude != undefined) {
					sprite.alpha = clamp(entity.altitude + 1, 0.25, 1);
				}

				// Selective snapping
				if (isNew || dist(sprite.position, entity.position) > 15) {
					sprite.position.set(entity.position.x, entity.position.y);
				}
				sprite.velocity = entity.velocity;
				if (isNew || Math.abs(angleDiff(sprite.rotation, entity.direction)) > Math.PI / 4) {
					sprite.rotation = entity.direction;
				}

				// update armament consumption
				if (sprite.armaments) {
					for (let i = 0; i < sprite.armaments.length; i++) {
						if (!sprite.armaments[i]) {
							continue;
						}
						const consumption = (entity.armamentConsumption || [])[i] || 0;
						sprite.armaments[i].alpha = consumption === 0 ? 1 : 0.5;
					}
				}

				if (sprite.turrets && entity.turretAngles) {
					for (let i = 0; i < sprite.turrets.length; i++) {
						sprite.turrets[i].rotation = entity.turretAngles[i] || 0;
					}
				}
			}

			for (const entityID of Object.keys(entitySprites)) {
				const entity = newContacts[entityID];
				const sprite = entitySprites[entityID];

				if (!entity || !entity.type) {
					const spriteData = entityData[sprite.type];

					// Spawn destruction effect
					if (!entity && spriteData && !(sprite.uncertainty > 0.75)) {
						const volume = Math.min(0.25, volumeAt(sprite.position));
						if (spriteData.kind === 'collectible') {
							playSoundSafe('collect', {volume});
						} else {
							let animation;
							let group;
							let spriteSize;

							if (['sam', 'shell', 'rocket', 'missile'].includes(spriteData.subkind)) {
								animation = extrasSpritesheet.animations.explosion;
								group = splashes;
								spriteSize = 5;
							} else {
								animation = extrasSpritesheet.animations.splash;
								group = explosions;
								spriteSize = 2;
							}

							if (spriteData.kind === 'boat') {
								playSoundSafe('explosionLong', {volume});
							} else {
								playSoundSafe('explosionShort', {volume});
							}

							const destruction = new PIXI.AnimatedSprite(animation);
							destruction.position.set(sprite.position.x, sprite.position.y);
							destruction.anchor.set(0.5);
							destruction.rotation = Math.random() * Math.PI * 2;

							const size = clamp(sprite.width * spriteSize, 5, 15);
							destruction.width = size;
							destruction.height = size;
							destruction.loop = false;
							destruction.animationSpeed = 0.5;
							group.addChild(destruction);

							destruction.gotoAndPlay(0);

							destruction.onComplete = () => {
								group.removeChild(destruction);
								destruction.destroy();
							}
						}
					}

					removeSprite(entityID, sprite);
				}
			}
		}

		// Update sprites whenever contacts change
		customOnDestroy(socketContacts.subscribe(reconcileContacts));

		// Terrain
		let terrainTexture = null;
		let terrainDimensions = [0, 0, 0, 0]; // x, y, width, height

		customOnDestroy(terrain.subscribe(data => {
			if (!data) {
				return;
			}

			let width = data.stride;
			let height = data.data.length / 4 / data.stride;

			let buffer = data.data;
			if (width === 0 || height === 0) {
				width = 1;
				height = 1;
				buffer = new Uint8Array([0,0,0,0]);
			}

			if (terrainTexture) {
				const sizeChanged = width != terrainTexture.baseTexture.resource.width || height != terrainTexture.baseTexture.resource.height;
				if (sizeChanged) {
					terrainTexture.destroy();
					terrainTexture = null;
				}
			}

			if (terrainTexture) {
				terrainTexture.baseTexture.resource.data = buffer;
				terrainTexture.update();
			} else {
				terrainTexture = PIXI.Texture.fromBuffer(buffer, width, height, {
					scaleMode: PIXI.SCALE_MODES.LINEAR,
				});
			}

			terrainDimensions[0] = data.x;
			terrainDimensions[1] = data.y;
			terrainDimensions[2] = data.width || 1;
			terrainDimensions[3] = data.height || 1;
		}));

		async function updateGlobalLeaderboard() {
			try {
				const response = await fetch('/leaderboard.json');
				if (!response.ok) {
					throw new Error('NOK');
				}
				globalLeaderboard = await response.json();
			} catch (err) {
				console.log('could not fetch leaderboard');
			}
		}

		let secondsTotal = 0;
		let frames = 0;
		let traceCounter = 0;

		updateGlobalLeaderboard();

		const frame = delta => {
			if (destroying) {
				console.log('ticker fired after destroying');
				return;
			}

			// Update canvas/renderer size
			app.renderer.resize(width, height);
			viewport.resize(width, height, WORLD_SIZE, WORLD_SIZE)

			const seconds = app.ticker.elapsedMS / 1000;
			frames++;
			const FPS_INTERVAL = 4; // seconds
			if (Math.floor((secondsTotal + seconds) / FPS_INTERVAL) > Math.floor(secondsTotal / FPS_INTERVAL)) {
				const fps = frames / FPS_INTERVAL;
				perf = mapRanges(fps, 20, 55, 0, 1, true);

				// Trace the first FPS calculation and every ~15 subsequent ones
				if (traceCounter == 0 || traceCounter >= 15) {
					console.log(`fps: ${fps}, perf: ${perf}`);
					send('trace', {fps});
					updateGlobalLeaderboard();
					traceCounter = 1;
				}

				frames = 0;
				traceCounter++;
			}
			secondsTotal += seconds;

			const localEntity = contacts[localEntityID];
			const localSprite = entitySprites[localEntityID];
			const localEntityData = localEntity ? entityData[localEntity.type] : null;

			if (localEntity && localSprite) {
				const mousePositionScreen = app.renderer.plugins.interaction.mouse.global;
				const mousePosition = viewport.toWorld(mouse);
				const mouseDistance = dist(mousePosition, localSprite.position);
				const mouseAngle = Math.atan2(mousePosition.y - localSprite.position.y, mousePosition.x - localSprite.position.x);

				keyEvent |= keyboard.forward || keyboard.backward || keyboard.right || keyboard.left || keyboard.stop;
				let angVelTarget = undefined;
				if (mouseDownNotClick() || keyEvent || active != lastActive || altitudeTarget != lastAltitudeTarget) {
					if (mouseDownNotClick()) {
						localSprite.directionTarget = mouseAngle;
						localSprite.velocityTarget = mapRanges(mouseDistance / localSprite.width, THROTTLE_START, THROTTLE_END, 0, entityData[localEntity.type].speed, true);
					} else {
						if (typeof localSprite.velocityTarget !== 'number') {
							localSprite.velocityTarget = localSprite.velocity || 0;
						}

						if (typeof localSprite.directionTarget !== 'number') {
							localSprite.directionTarget = localSprite.rotation;
						}
					}

					if (keyEvent) {
						const forwardBackward = keyboard.forward || keyboard.backward;
						const turnSpeed = 150 / (150 + localEntityData.length);

						if (forwardBackward) {
							if (keyboard.forward) {
								localSprite.velocityTarget += 50 * seconds;
							}
							if (keyboard.backward) {
								localSprite.velocityTarget -= 50 * seconds;
							}

							// Straighten out
							angVelTarget = 0;

							// This turn will be added to the current course,
							// not the current desired course.
							// If too low, relative to tick rate, oscillations
							// will occur.
							if (keyboard.right) {
								angVelTarget = turnSpeed * Math.PI;
							}
							if (keyboard.left) {
								angVelTarget = -turnSpeed * Math.PI;
							}

							localSprite.directionTarget = localSprite.rotation + angVelTarget * seconds;
						} else {
							let newDirectionTarget = localSprite.directionTarget;

							if (keyboard.right) {
								newDirectionTarget += turnSpeed * Math.PI * seconds;
							}
							if (keyboard.left) {
								newDirectionTarget -= turnSpeed * Math.PI * seconds;
							}

							// Limit turn target to 90 degrees from current bearing
							if (Math.abs(angleDiff(newDirectionTarget, localSprite.rotation)) < 0.5 * Math.PI) {
								localSprite.directionTarget = newDirectionTarget;
							}
						}

						localSprite.velocityTarget = clamp(localSprite.velocityTarget, -0.33 * localEntityData.speed, localEntityData.speed);
					}

					if (keyboard.stop) {
						localSprite.velocityTarget = 0;
					}

					timesMoved++;
				}

			 	if (secondsTotal - lastSend >= SECONDS_PER_SEND) {
					lastSend = secondsTotal;

					setOverlay('speed', localSprite.velocity);
					setOverlay('positionX', localSprite.position.x);
					setOverlay('positionY', localSprite.position.y);
					setOverlay('direction', localSprite.rotation);

					// Fire weapons when sending as a form of rate limiting
					if ((mouse.click || keyboard.shoot) && localEntityData.armaments && armamentSelection) {
						let directionTarget = mouseAngle;
						if (keyboard.shoot) {
							directionTarget = localSprite.rotation;
						}

						let bestArmamentIndex = -1;
						let bestArmamentAngleDiff = Infinity;

						function hasArmament(consumption, index) {
							return !consumption || consumption.length <= index || consumption[index] < 0.001;
						}

						for (let index = 0; index < localEntityData.armaments.length; index++) {
							const armament = localEntityData.armaments[index];

							const type = getArmamentType(armament);

							if (type !== armamentSelection) {
								continue;
							}

							if (!hasArmament(localEntity.armamentConsumption, index)) {
								continue;
							}

							let armamentAngle = armament.angle || 0;
							let armamentPosX = armament.positionForward || 0;
							let armamentPosY = armament.positionSide || 0;

							if (armament.turret != null) {
								const turretData = localEntityData.turrets[armament.turret];
								const turretAngle = localEntity.turretAngles[armament.turret] || (turretData.angle || 0);

								const azimuthF = angleDiff((turretData.angle || 0) + Math.PI, turretAngle);
								if (turretData.azimuthFL != null && turretData.azimuthFL - Math.PI > azimuthF) {
									continue;
								}
								if (turretData.azimuthFR != null && Math.PI - turretData.azimuthFR < azimuthF) {
									continue;
								}

								const azimuthB = angleDiff(turretData.angle || 0, turretAngle);
								if (turretData.azimuthBL != null && turretData.azimuthBL - Math.PI > azimuthB) {
									continue;
								}
								if (turretData.azimuthBR != null && Math.PI - turretData.azimuthBR < azimuthB) {
									continue;
								}

								armamentAngle += turretAngle;

								const newArmamentPos = addTransforms(turretData.positionForward || 0, turretData.positionSide || 0, armamentPosX, armamentPosY, turretAngle);
								armamentPosX = newArmamentPos.x;
								armamentPosY = newArmamentPos.y;
							}

							const armamentEntityData = entityData[armament.type];

							const newArmamentPos = addTransforms(localSprite.position.x, localSprite.position.y, armamentPosX, armamentPosY, localSprite.rotation);
							armamentPosX = newArmamentPos.x;
							armamentPosY = newArmamentPos.y;

							// Each armament has a slightly different angle to the target
							// and this must be taken into acount for large ships
							// with spaced-out armaments
							let armamentDirectionTarget = Math.atan2(mousePosition.y - armamentPosY, mousePosition.x - armamentPosX);
							if (keyboard.shoot) {
								// Unless the direction came from keyboard
								armamentDirectionTarget = directionTarget;
							}

							let diff = Math.abs(angleDiff(localEntity.direction + armamentAngle, armamentDirectionTarget));

							if (armament.vertical || ['aircraft'].includes(armamentEntityData.kind) || ['depositor', 'depthCharge', 'mine'].includes(armamentEntityData.subkind)) {
								// Vertically-launched armaments can fire in any horizontal direction
								diff = 0;
							}
							if (diff < bestArmamentAngleDiff) {
								bestArmamentIndex = index;
								bestArmamentAngleDiff = diff;
							}
						}

						if (bestArmamentIndex != -1) {
							let requiredAngle = 1.1 * Math.PI / 2;
							if (armamentSelection === 'weapon/shell') {
								requiredAngle = 0.1 * Math.PI;
							}

							if (bestArmamentAngleDiff <= requiredAngle || keyboard.shoot) {
								send('fire', {
									entityID: localEntityID,
									index: bestArmamentIndex,
									positionTarget: mousePosition
								});

								weaponsFired++;
							}
						}
					}

					if (mouseDownNotClick() || keyEvent || active != lastActive || altitudeTarget != lastAltitudeTarget) {
						send('manual', {
							entityID: localEntityID,
							velocityTarget: localSprite.velocityTarget,
							angVelTarget,
							directionTarget: localSprite.directionTarget,
							active,
							altitudeTarget: localEntityData.subkind === 'submarine' ? altitudeTarget : undefined,
							aimTarget: mousePosition,
						});

						if (localEntityData.subkind === 'submarine') {
							if (lastAltitudeTarget == 0 && altitudeTarget < 0) {
								playSoundSafe('dive');
							} else if (lastAltitudeTarget < 0 && altitudeTarget == 0) {
								playSoundSafe('surface');
							}
						}
						if (!lastActive && active) {
							if (localEntityData && localEntityData.sensors && localEntityData.sensors.sonar && localEntityData.sensors.sonar.range) {
								playSoundSafe('sonar1');
							}
						}

						lastActive = active;
						lastAltitudeTarget = altitudeTarget;
					} else {
						// TODO: Some ships don't need to aim turrets, in which
						// case this is a (wasteful) no-op
						// NOTE: until the bug is fixed, must consider the case
						// of a ship with an airborne aircraft upgrading into a
						// ship without turrets/aircraft
						send('aim', {
							target: mousePosition
						});
					}

					// Reset input
					if (keyboard.pay) {
						send('pay', {
							position: mousePosition
						});
						keyboard.pay = false;
					}
					mouse.click = false;
					keyEvent = false;
				}

				drawHud(hud, localEntity, localSprite, contacts);
			}

			let maxVisualRange = Math.min(terrainDimensions[2], terrainDimensions[3]) / 2;
			if (maxVisualRange === 0) {
				maxVisualRange = 500;
			}
			let visualRange = maxVisualRange - 50; // don't show texture borders
			if (localEntityData && localEntityData.sensors) {
				for (const sensorType in localEntityData.sensors) {
					const sensor = localEntityData.sensors[sensorType];
					if (sensorType === 'visual') {
						maxVisualRange = Math.max(maxVisualRange, sensor.range);
						break;
					}
				}
			}

			// Bigger boats can zoom out wider
			// Use max range to not zoom in and out when diving sub
			viewport.clampZoom({minScale: 600 / maxVisualRange, maxScale: 6000 / maxVisualRange});
			Sounds.volume('ocean', 0.15 * viewport.scale.x)

			for (const entityID of Object.keys(entitySprites)) {
				const entity = contacts[entityID];
				const sprite = entitySprites[entityID];
				const spriteData = entityData[entity.type];

				if (!entity) {
					console.warn('no entity for sprite', sprite);
					continue;
				}

				const interpolateAngle = angleDiff(sprite.rotation, entity.direction);
				sprite.rotation += mapRanges(seconds, 0, 0.25, 0, interpolateAngle, true);

				// Direction target of 0 may be invalid
				if (spriteData && (entity.friendly || entity.directionTarget)) {
					let maxTurnSpeed = Math.PI / 4; // per second
					if (spriteData.subkind === 'heli') {
						maxTurnSpeed = Math.PI / 2;
					}

					let angleDifference = angleDiff(sprite.rotation, entity.directionTarget || 0);
					let maxSpeed = spriteData.speed || 20;
					if (spriteData.kind !== 'aircraft') {
						maxSpeed /= Math.max(Math.pow(angleDifference, 2), 1);
						maxTurnSpeed *= Math.max(0.25, 1 - Math.abs(sprite.velocity || 0) / (maxSpeed + 1));
					}
					sprite.rotation += clampMagnitude(angleDifference, seconds * maxTurnSpeed);

					angleDifference = angleDiff(entity.direction, entity.directionTarget || 0);
					entity.direction += clampMagnitude(angleDifference, seconds * maxTurnSpeed);
				}

				sprite.position.x = mapRanges(seconds, 0, 0.25, sprite.position.x, entity.position.x, true);
				sprite.position.y = mapRanges(seconds, 0, 0.25, sprite.position.y, entity.position.y, true);

				applyVelocity(entity.position, entity.velocity, entity.direction, seconds);
				applyVelocity(sprite.position, sprite.velocity, sprite.rotation, seconds);

				const spriteDistance = dist(sprite.position, viewport.center);

				if (spriteData && spriteDistance <= visualRange) {
					let amount =  0.03 * sprite.width * Math.log(sprite.velocity) * perf;
					let wakeAngle = 2 * Math.atan(sprite.height / (Math.max(1, sprite.velocity)));
					if (spriteData.kind === 'aircraft') {
						amount *= 0.25;
						wakeAngle *= 0.2;
					}
					for (let i = 0; i < amount; i++) {
						let wakeParticle = recycleParticle(wakeParticles);
						if (!wakeParticle) {
							wakeParticle = new PIXI.Sprite(extrasSpritesheet.textures.wake);
							wakeParticle.anchor.set(0.5);
							wakeParticles.addChild(wakeParticle);
						}

						const r = sprite.width * 0.5 - 2;
						wakeParticle.position.set(sprite.position.x - Math.cos(sprite.rotation) * r, sprite.position.y - Math.sin(sprite.rotation) * r);
						const direction = sprite.rotation + Math.sign(Math.random() - 0.5) * wakeAngle * (0.75 * Math.random() + 0.25);
						wakeParticle.sinDirection = Math.sin(direction);
						wakeParticle.cosDirection = Math.cos(direction);
						wakeParticle.maxAlpha = sprite.alpha;
						wakeParticle.velocity = sprite.velocity * Math.random();
						const scale = 0.1 * (1 + 0.25 * (Math.random() - 0.5));
						wakeParticle.scale.x = scale;
						wakeParticle.scale.y = scale;
					}
				}

				if (sprite.nameText) {
					sprite.nameText.position.x = sprite.position.x;
					sprite.nameText.position.y = sprite.position.y - sprite.width * (THROTTLE_START + THROTTLE_END) / 2 - 3 * NAME_SCALE / viewport.scale.x;
					sprite.nameText.scale.set(0.1 * NAME_SCALE / viewport.scale.x);
				}

				if (sprite.healthBar) {
					sprite.healthBar.position.x = sprite.position.x;
					sprite.healthBar.position.y = sprite.position.y - sprite.width * (THROTTLE_START + THROTTLE_END) / 2;
					sprite.healthBar.scale.set(NAME_SCALE / viewport.scale.x);
				}

				if (sprite.triangle) {
					sprite.triangle.position.x = sprite.position.x;
					sprite.triangle.position.y = sprite.position.y - sprite.width - 2 * NAME_SCALE / viewport.scale.x;
					sprite.triangle.scale.set(0.2 * NAME_SCALE / viewport.scale.x);
				}
			}

			updateParticles(wakeParticles, seconds);
			updateParticles(smokeParticles, seconds);

			background.uniforms.iScale = [1 / viewport.scale.x, 1 / viewport.scale.x];
			background.uniforms.iTime = secondsTotal;
			background.uniforms.iVisualRange = visualRange;
			background.uniforms.iTerrain = terrainTexture;
			background.uniforms.iTerrainDimensions = terrainDimensions;

			function setViewPosition(position, interpolate) {
				if (interpolate) {
					// Interpolate
					position.x = mapRanges(seconds, 0, 0.5, viewportPositionCache.x, position.x, true);
					position.y = mapRanges(seconds, 0, 0.5, viewportPositionCache.y, position.y, true);
				}

				viewportPositionCache = position;
				hud.position.set(position.x, position.y);
				viewport.moveCenter(position.x, position.y);
				background.uniforms.iMiddle = [position.x, position.y];
				background.uniforms.iOffset = [position.x - app.screen.width / viewport.scale.y / 2, position.y + app.screen.height / viewport.scale.y / 2];
			}

			if (localEntity && localSprite) {
				hud.visible = true;
				setViewPosition(localSprite.position, true);
			} else {
				hud.visible = false;
				const newCenter = {x: terrainDimensions[0] + terrainDimensions[2] / 2, y: terrainDimensions[1] + terrainDimensions[3] / 2};
				const distance = dist(viewportPositionCache, newCenter);
				if (distance < 50) {
					setViewPosition(viewportPositionCache, false);
				} else {
					setViewPosition(newCenter, true);
				}
			}
		};

		app.ticker.add(frame);

		// start receiving leaderboard, background, etc.
		// do this after we are actually ready for data, to be safe
		connect();

		// Cannot use 'return' method of setting onDestroy, since this is an
		// async function
		customOnDestroy(() => {
			disconnect();
			console.log('destroying app')
			Sounds.removeAll();
			// Cannot delete system/shared ticker since they're protected
			// so settle for removing frame handler
			app.ticker.remove(frame);
			app.loader.reset();
			app.destroy(false, {children: true, texture: true, baseTexture: true});
			PIXI.utils.destroyTextureCache();
		});
	});

	function updateMouseLeftDown(leftDown) {
		if (leftDown) {
			mouse.leftDown = Date.now();
		} else {
			if ((Date.now() - mouse.leftDown) <= MOUSE_CLICK_MILLIS) {
				mouse.click = true;
			}
			mouse.leftDown = 0;
		}
	}

	function handleMouseMove(event) {
		if (event.touches && event.touches.length > 0) {
			mouse.x = event.touches[0].pageX;
			mouse.y = event.touches[0].pageY;
		} else if (typeof event.pageX === 'number') {
			mouse.x = event.pageX;
			mouse.y = event.pageY;
		}

		mouse.touch |= event.type.startsWith('touch');
		mouse = mouse; // reactivity
	}

	function handleMouseButton(event) {
		event.preventDefault();
		event.stopPropagation();

		chatRef && chatRef.blur && chatRef.blur();

		const button = getMouseButton(event);

		const down = {mousedown: true, mouseup: false}[event.type];

		switch (button) {
			case 0:
				updateMouseLeftDown(down);
				break;
			case 2:
				// Right button never translates into click
				mouse.rightDown = down;
				break;
		}

		handleMouseMove(event);
	}

	function handleTouch(event) {
		event.preventDefault();

		const button = getMouseButton(event);

		if (['touchstart', 'touchend'].includes(event.type)) {
			updateMouseLeftDown(event.type === 'touchstart');
		}

		handleMouseMove(event);
	}

	function handleKey(event) {
		const {ctrlKey, keyCode, preventDefault, shiftKey, target, type} = event;

		const down = {keydown: true, keyup: false}[type];

		if (down && target && (ctrlKey || (target instanceof HTMLInputElement))) {
			return;
		}

		if (down !== undefined) {
			const keys = {
				32: 'shoot', // space
				67: () => {
					keyboard.pay = true; // only once per keypress
				}, // c (coin)
				69: 'shoot', // e
				88: 'stop', // x
				86: () => {
					if (recording) {
						stopRecording();
						recording = false;
					} else if (event.shiftKey) { // See #80
						startRecording(canvas);
						recording = true;
					}
				}, // v

				// WASD
				65: 'left',
				87: 'forward',
				68: 'right',
				83: 'backward',

				// arrows
				37: 'left',
				38: 'forward',
				39: 'right',
				40: 'backward',
			};

			if (chatRef && chatRef.focus) {
				// enter
				keys[13] = chatRef.focus.bind(chatRef);
			}

			// Last 3 checks to prevent https://github.com/SoftbearStudios/mk48/issues/26
			if (shipRef && shipRef.toggleActive && shipRef.toggleAltitudeTarget && shipRef.incrementSelection && shipRef.setSelectionIndex) {
				// tab
				keys[9] = shipRef.incrementSelection.bind(shipRef);

				// z
				keys[90] = shipRef.toggleActive.bind(shipRef);

				// r
				keys[82] = shipRef.toggleAltitudeTarget.bind(shipRef);

				// numbers
				for (let i = 0; i < 5; i++) {
					keys[49 + i] = shipRef.setSelectionIndex.bind(shipRef, i);
				}
			}

			const key = keys[keyCode];

			if (key) {
				if (typeof key === 'function') {
					if (down) {
						key();
					}
				} else {
					keyboard[key] = down;
					//event.preventDefault();
					keyboard = keyboard; // reactivity
				}

				event.preventDefault();
				event.stopPropagation();
			}
		}
	}

	$: spawned = localEntityID && contacts[localEntityID];
</script>

<main bind:clientWidth={widthFract} bind:clientHeight={heightFract}>
	<canvas
		bind:this={canvas}
		{width} {height}
		tabindex={0}
		on:contextmenu|preventDefault
		on:mousedown={handleMouseButton} on:mouseup={handleMouseButton} on:mousemove={handleMouseMove}
		on:touchstart={handleTouch} on:touchend={handleTouch} on:touchmove={handleMouseMove}></canvas>

	<div class='top bar'>
		{#if spawned}
			<Teams {contacts}/>
			{#if canUpgrade(contacts[localEntityID].type, contacts[localEntityID].score)}
				<Upgrades
					score={contacts[localEntityID].score}
					type={contacts[localEntityID].type}
					callback={type => send('upgrade', {type})}
				/>
			{:else}
				<Instructions touch={mouse.touch} instructBasics={timesMoved < 100 || weaponsFired < 2} {instructZoom}/>
			{/if}
		{:else}
			<!-- Render this div even without contents, as it causes the flex
			box to shift the other contents to the right side -->
			<div>
				{#if globalLeaderboard}
					{#if globalLeaderboard['single/all']}
						<Leaderboard label={$t('panel.leaderboard.type.single/all')} leaderboard={globalLeaderboard['single/all']} headerAlign='left'/>
						<br/>
					{/if}
					{#if globalLeaderboard['single/week']}
						<Leaderboard label={$t('panel.leaderboard.type.single/week')} open={false} leaderboard={globalLeaderboard['single/week']} headerAlign='left'/>
						<br/>
					{/if}
					{#if globalLeaderboard['single/day']}
						<Leaderboard label={$t('panel.leaderboard.type.single/day')} open={false} leaderboard={globalLeaderboard['single/day']} headerAlign='left'/>
					{/if}
				{/if}
			</div>
		{/if}
		{#if $leaderboard}
			<Leaderboard leaderboard={$leaderboard} headerAlign='right'/>
		{/if}
	</div>
	<div class='bottom bar'>
		{#if spawned}
			<Ship type={contacts[localEntityID].type} consumption={contacts[localEntityID].armamentConsumption} altitude={contacts[localEntityID].altitude} bind:active bind:altitudeTarget bind:selection={armamentSelection} bind:this={shipRef}/>
			<Status {overlay} {recording} type={contacts[localEntityID].type}/>
			<Chat callback={data => send('sendChat', data)} bind:this={chatRef}/>
		{/if}
	</div>
	{#if spawned}
		<Hint type={contacts[localEntityID].type}/>
	{:else}
		<SplashScreen callback={onStart} connectionLost={$connected === false}/>
	{/if}
	<Sidebar zoom={amount => viewport && viewport.setZoom(viewport.scale.x + amount, true)}/>
</main>

<svelte:window on:keydown={handleKey} on:keyup={handleKey}/>

<style>
	:root {
		font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell,
			'Open Sans', 'Helvetica Neue', sans-serif;
	}

	canvas {
		height: 100%;
		margin: 0;
		width: 100%;
	}

	div.bar {
		position: absolute;
		left: 0;
		right: 0;
		height: min-content;
		pointer-events: none;
		display: flex;
		justify-content: space-between;
	}

	div.bar.top {
		top: 0;
	}

	div.bar.bottom {
		bottom: 0;
		align-items: flex-end;
	}

	div.bar > :global(div) {
		height: min-content;
		pointer-events: all;
		margin: 1em;
	}

	main {
		background-color: #003574;
		color: white;
		left: 0;
		right: 0;
		top: 0;
		bottom: 0;
		margin: 0;
		overflow: hidden;
		padding: 0;
		position: absolute !important;
	}

	:global(input), :global(select) {
		border-radius: 0.25em;
		box-sizing: border-box;
		cursor: pointer;
		font-size: 1em;
		font-weight: bold;
		outline: 0;
		padding: 0.7em;
		pointer-events: all;
		white-space: nowrap;
		margin-top: 0.25em;

		background-color: #00000025;
		border: 0;
		color: white;
	}

	:global(input::placeholder) {
		opacity: 0.75;

		color: white;
	}

	:global(button) {
		background-color: #2980b9;
		border: 1px solid #2980b9;
		border-radius: 0.25em;
		box-sizing: border-box;
		color: white;
		cursor: pointer;
		font-size: 1em;
		margin-top: 0.5em;
		padding: 0.5em 0.6em;
		text-decoration: none;
		white-space: nowrap;
		width: 100%;
	}

	:global(button:disabled) {
		filter: opacity(0.6);
	}

	:global(button:hover:not(:disabled)) {
		filter: brightness(0.95);
	}

	:global(button:active:not(:disabled)) {
		filter: brightness(0.9);
	}

	:global(html) {
		font-size: 1.4vmin;
		font-size: calc(5px + 0.9vmin);
	}
</style>
