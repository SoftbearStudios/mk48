// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

// Updates a container of container, where each particle has a velocity and
// direction in addition to normal fields
export function updateParticles(container, seconds) {
	if (container.deadParticles) {
		for (const deadParticle of container.deadParticles) {
			container.removeChild(deadParticle);
			deadParticle.destroy();
		}
	}

	container.deadParticles = [];

	for (const particle of container.children) {
		particle.position.x += particle.cosDirection * particle.velocity * seconds;
		particle.position.y += particle.sinDirection * particle.velocity * seconds;

		particle.velocity *= 0.95;
		particle.velocity *= 0.95;
		particle.alpha = (particle.maxAlpha || 1) * particle.velocity;

		if (particle.velocity < 0.1) {
			container.deadParticles.push(particle);
		}
	}
}

// Deletions are postponed one cycle to allow recycle
export function recycleParticle(container) {
	if (container.deadParticles) {
		return container.deadParticles.pop();
	}
	return undefined;
}
