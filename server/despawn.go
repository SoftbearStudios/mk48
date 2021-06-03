package main

import "github.com/SoftbearStudios/mk48/server/world"

// Despawn removes pending players.
// The purpose is to make removing k players O(n + k)
// instead of O(n * k) by removing multiple at once.
func (h *Hub) Despawn() {
	// Uses a set for higher efficiency.
	removals := make(map[*world.Player]struct{}, h.despawn.Len)

	// Iterate and removal all items.
	for client := h.despawn.First; client != nil; client = h.despawn.Remove(client) {
		removals[&client.Data().Player.Player] = struct{}{}
	}

	// Parallelize for lower latency.
	h.world.SetParallel(true)
	h.world.ForEntities(func(entity *world.Entity) (_, remove bool) {
		_, remove = removals[entity.Owner]
		return
	})
	h.world.SetParallel(false)
}
