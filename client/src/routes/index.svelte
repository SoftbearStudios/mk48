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
	import Teams from '../lib/Teams.svelte';
	import Upgrades from '../lib/Upgrades.svelte';
	import {drawHud, THROTTLE_END, THROTTLE_START} from '../lib/hud.js';
	import {recycleParticle, updateParticles} from '../lib/particles.js';
	import {applyVelocity} from '../lib/physics.js';
	import {connect, connected, disconnect, send, contacts as socketContacts, entityID as socketEntityID, terrain, leaderboard, worldRadius} from '../lib/socket.js';
	import backgroundShader from '../lib/background.js';
	import {startRecording, stopRecording} from '../lib/recording.js';
	import {onMount} from 'svelte'

	// Spritesheet data
	import entitiesTPS from '../data/entities.tps.json';
	import extrasTPS from '../data/extras.tps.json';

	// Entity Data
	import entityData from '../data/entities.json';

	let canvas, chatRef, shipRef, heightFract, widthFract;
	$: height = Math.floor(heightFract);
	$: width = Math.floor(widthFract);

	let mouse = {x: 0, y: 0, touch: false, leftDown: 0, rightDown: false, click: false};
	let keyboard = {shoot: false, forward: false, backward: false, right: false, left: false, stop: false}; // what keys are down
	let keyEvent = false;
	let overlay = {};
	let viewportPositionCache = {x: 0, y: 0};
	let armamentSelection;
	let altitudeTarget;
	let lastAltitudeTarget; // last altitudeTarget sent to server
	let lastSend = 0; // secondsTotal of last manual/aim
	let perf = 0.5; // performance level in interval [0,1]

	// Global leaderboard
	let globalLeaderboard = null;

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

	onMount(async () => {
		const PIXI = await import('pixi.js');
		const {Viewport} = await import('pixi-viewport');

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

		const viewport = new Viewport({
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

		// Background (water + land)
		const background = new PIXI.Filter(null, backgroundShader);
		const backgroundContainer = new PIXI.Container();
		viewport.addChild(backgroundContainer);
		backgroundContainer.filterArea = app.screen;
		backgroundContainer.filters = [background];
		const unsubscribeWorldRadius = worldRadius.subscribe(newRadius => {
			background.uniforms.iBorderRange = newRadius;
		});

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

		const entityContainer = new PIXI.Container();
		viewport.addChild(entityContainer);

		const splashes = new PIXI.Container();
		viewport.addChild(splashes);

		const wakeParticles = new PIXI.ParticleContainer(16384, {
			scale: true,
			position: true,
			alpha: true,
			autoResize: true
		});
		viewport.addChild(wakeParticles);

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

		function reconcileContacts(newContacts) {
			contacts = newContacts;

			for (const entityID of Object.keys(newContacts)) {
				const entity = newContacts[entityID];

				if (!entity.type) {
					continue; // radar contacts, etc.
				}

				const currentEntityData = entityData[entity.type];

				let sprite = entitySprites[entityID];
				const isNew = !sprite || entity.type != sprite.type;

				if (isNew) {
					if (sprite) {
						removeSprite(entityID, sprite);
					}

					sprite = PIXI.Sprite.from(entitiesSpritesheet.textures[entity.type]);
					entitySprites[entityID] = sprite;

					sprite.type = entity.type;

					sprite.anchor.set(0.5);
					sprite.height = currentEntityData.width;
					sprite.width = currentEntityData.length;
					entityContainer.addChild(sprite);

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

							const armamentSprite = PIXI.Sprite.from(entitiesSpritesheet.textures[armament.default]);
							armamentSprite.position.set((armament.positionForward || 0) / sprite.scale.x, (armament.positionSide || 0) / sprite.scale.y);
							armamentSprite.anchor.set(0.5);
							armamentSprite.rotation = armament.angle || 0;
							if (armament.turret != undefined) {
								sprite.turrets[armament.turret].addChild(armamentSprite);
							} else {
								sprite.addChild(armamentSprite);
							}

							armamentSprite.height = entityData[armament.default].width / sprite.scale.y;
							armamentSprite.width = entityData[armament.default].length / sprite.scale.x;

							sprite.armaments[a] = armamentSprite;
						}
					}
				}

				// Markers/nametags
				let oldColor = null, newColor = null;
				switch (currentEntityData.type) {
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

						if (currentEntityData.type === 'boat' && (!sprite.healthBar || sprite.healthBar.health !== health || newColor !== oldColor)) {
							if (!sprite.healthBar) {
								sprite.healthBar = new PIXI.Graphics();
								viewport.addChild(sprite.healthBar);
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
					if (!entity && spriteData && spriteData.type !== 'collectible') {
						let animation;
						let group;
						let spriteSize;

						if (['sam', 'shell', 'rocket', 'missile'].includes(spriteData.subtype)) {
							animation = extrasSpritesheet.animations.explosion;
							group = splashes;
							spriteSize = 5;
						} else {
							animation = extrasSpritesheet.animations.splash;
							group = explosions;
							spriteSize = 2;
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

					removeSprite(entityID, sprite);
				}
			}
		}

		// Update sprites whenever contacts change
		const unsubscribeSocketContacts = socketContacts.subscribe(reconcileContacts);

		// Terrain
		let terrainTexture = null;
		let terrainDimensions = [0, 0, 0, 0]; // x, y, width, height

		const unsubscribeTerrain = terrain.subscribe(data => {
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
		});

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

		app.ticker.add(delta => {
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
				if (mouseDownNotClick() || keyEvent || altitudeTarget != lastAltitudeTarget) {
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
						let newTarget = localSprite.directionTarget;
						let forwardBackward = keyboard.forward || keyboard.backward;

						if (forwardBackward) {
							if (keyboard.forward) {
								localSprite.velocityTarget += 50 * seconds;
							}
							if (keyboard.backward) {
								localSprite.velocityTarget -= 50 * seconds;
							}

							// Straighten out (and damp oscillation by using the entity ground truth)
							newTarget = localEntity.direction;
						}

						let turnSpeed = 100 / (100 + localEntityData.length);
						const sign = 1; // localSprite.velocity >= -0.1 ? 1 : -1;

						if (forwardBackward) {
							// This turn will be added to the current course,
							// not the current desired course.
							// If too low, relative to tick rate, oscillations
							// will occur.
							turnSpeed *= 5;
						}

						if (keyboard.right) {
							newTarget += turnSpeed * sign * Math.PI * seconds;
						}
						if (keyboard.left) {
							newTarget -= turnSpeed * sign * Math.PI * seconds;
						}
						if (Math.abs(angleDiff(newTarget, localSprite.rotation)) < 0.5 * Math.PI) {
							localSprite.directionTarget = newTarget;
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
								const turretAngle = (localEntity.turretAngles[armament.turret] || turretData.angle);
								armamentAngle += turretAngle;

								const newArmamentPos = addTransforms(turretData.positionForward || 0, turretData.positionSide || 0, armamentPosX, armamentPosY, turretAngle);
								armamentPosX = newArmamentPos.x;
								armamentPosY = newArmamentPos.y;
							}

							const armamentEntityData = entityData[armament.default];

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

							if (armament.airdrop || armament.vertical || ['aircraft', 'depositor', 'depthCharge', 'mine'].includes(armamentEntityData.subtype)) {
								// Air-dropped or vertically-launched armaments can fire in any horizontal direction
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

					if (mouseDownNotClick() || keyEvent || altitudeTarget != lastAltitudeTarget) {
						send('manual', {
							entityID: localEntityID,
							velocityTarget: localSprite.velocityTarget,
							directionTarget: localSprite.directionTarget,
							altitudeTarget: localEntityData.subtype === 'submarine' ? altitudeTarget : undefined,
							turretTarget: mousePosition,
						});

						lastAltitudeTarget = altitudeTarget;
					} else if (Array.isArray(localEntityData.turrets) && localEntityData.turrets.length > 0) {
						send('aimTurrets', {
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
				for (const sensor of localEntityData.sensors) {
					if (sensor.type === 'visual') {
						maxVisualRange = Math.max(maxVisualRange, sensor.range);
						break;
					}
				}
			}

			// Bigger boats can zoom out wider
			// Use max range to not zoom in and out when diving sub
			viewport.clampZoom({minScale: 600 / maxVisualRange, maxScale: 6000 / maxVisualRange});

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
				if (entity.friendly || entity.directionTarget) {
					let maxTurnSpeed = Math.PI / 4; // per second
					if (entity.type === 'seahawk') {
						maxTurnSpeed = Math.PI / 2;
					}

					let angleDifference = angleDiff(sprite.rotation, entity.directionTarget || 0);
					let maxSpeed = (spriteData.speed || 20)
					if (spriteData.subtype !== 'aircraft') {
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

				if (spriteDistance <= visualRange) {
					let amount =  0.03 * sprite.width * Math.log(sprite.velocity) * perf;
					let wakeAngle = 2 * Math.atan(sprite.height / (Math.max(1, sprite.velocity)));
					if (spriteData.subtype === 'aircraft') {
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
		});

		// start receiving leaderboard, background
		// do this after we are actually ready for data, to be safe
		connect();

		return () => {
			disconnect();
			unsubscribeSocketContacts();
			unsubscribeTerrain();
			unsubscribeWorldRadius();
			app.destroy.bind(app);
		};
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
		const {keyCode, preventDefault, shiftKey, target, type} = event;

		const down = {keydown: true, keyup: false}[type];

		if (down && target && (target instanceof HTMLInputElement)) {
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
			if (shipRef &&  shipRef.toggleAltitudeTarget && shipRef.incrementSelection && shipRef.setSelectionIndex) {
				// tab
				keys[9] = shipRef.incrementSelection.bind(shipRef);

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
</script>

<main bind:clientWidth={widthFract} bind:clientHeight={heightFract}>
	<canvas
		bind:this={canvas}
		{width} {height}
		tabindex={0}
		on:contextmenu|preventDefault
		on:mousedown={handleMouseButton} on:mouseup={handleMouseButton} on:mousemove={handleMouseMove}
		on:touchstart={handleTouch} on:touchend={handleTouch} on:touchmove={handleMouseMove}></canvas>
	{#if $leaderboard}
		<div class='leaderboard'>
			<Leaderboard leaderboard={$leaderboard}/>
		</div>
	{/if}
	{#if localEntityID && contacts[localEntityID]}
		<Instructions touch={mouse.touch} instructBasics={timesMoved < 100 || weaponsFired < 2} {instructZoom}/>
		<Status {overlay} {recording}/>
		<Ship type={contacts[localEntityID].type} consumption={contacts[localEntityID].armamentConsumption} bind:altitudeTarget bind:selection={armamentSelection} bind:this={shipRef}/>
		<Upgrades
			score={contacts[localEntityID].score}
			type={contacts[localEntityID].type}
			callback={type => send('upgrade', {type})}
		/>
		<Teams {contacts}/>
		<Chat callback={data => send('sendChat', data)} bind:this={chatRef}/>
	{:else}
		<SplashScreen callback={onStart} connectionLost={$connected === false}/>
		{#if globalLeaderboard}
			<div class='globalLeaderboard'>
				{#if globalLeaderboard['single/all']}
					<Leaderboard label='All-time Leaderboard' leaderboard={globalLeaderboard['single/all']}/>
				{/if}
				{#if globalLeaderboard['single/week']}
					<Leaderboard label='Weekly Leaderboard' open={false} leaderboard={globalLeaderboard['single/week']}/>
				{/if}
				{#if globalLeaderboard['single/day']}
					<Leaderboard label='Daily Leaderboard' open={false} leaderboard={globalLeaderboard['single/day']}/>
				{/if}
			</div>
		{/if}
	{/if}
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

	div.leaderboard {
		position: absolute;
		right: 0;
		top: 0;
	}

	div.globalLeaderboard {
		position: absolute;
		left: 0;
		top: 0;
	}

	main {
		background-color: #00487d;
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
		border: 1px solid gray;
		border-radius: 5px;
		box-sizing: border-box;
		color: black;
		cursor: pointer;
		font-weight: bold;
		margin-top: 5px;
		min-width: 200px;
		outline: 0px;
		padding: 8px;
		pointer-events: all;
		white-space: nowrap;
		width: 100%;
	}

	:global(input::placeholder) {
		color: black;
		opacity: 0.75;
	}

	:global(button) {
		background-color: #2980b9;
		border: 1px solid #2980b9;
		border-radius: 5px;
		box-sizing: border-box;
		color: white;
		cursor: pointer;
		font-size: 18px;
		margin-top: 5px;
		padding: 5px 7px;
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
</style>
