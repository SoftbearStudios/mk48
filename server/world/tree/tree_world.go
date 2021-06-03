// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package tree

import (
	"encoding/json"
	"fmt"
	"github.com/SoftbearStudios/mk48/server/world"
)

const treeNodeMaxEntities = 8

type (
	// TODO not functional yet
	World struct {
		root *treeNode
	}

	treeNode struct {
		world.AABB
		children [4]*treeNode
		entities []world.Entity
	}
)

func New(radius int) *World {
	return &World{
		root: newTreeNode(radiusAABB(world.Vec2f{}, float32(radius))),
	}
}

func (w *World) Count() (count int) {
	return w.root.count()
}

func (w *World) AddEntity(entity *world.Entity) {
	entity.EntityID = world.AllocateEntityID(func(id world.EntityID) bool {
		return false // TODO
	})

	w.root.add(entity)
}

func (w *World) EntityByID(entityID world.EntityID, callback func(entity *world.Entity) (remove bool)) {
	w.root.iterate(func(entity *world.Entity) (stop, remove bool) {
		if entityID == entity.EntityID {
			remove = callback(entity)
			stop = true
		}
		return
	})
}

func (w *World) ForEntities(callback func(entity *world.Entity) (stop, remove bool)) bool {
	return w.root.iterate(callback)
}

func (w *World) ForEntitiesInRadius(position world.Vec2f, radius float32, callback func(r float32, entity *world.Entity) (stop bool)) bool {
	aabb := radiusAABB(position, radius)
	return w.root.iterateAABB(aabb, func(entity *world.Entity) (stop, remove bool) {
		return callback(position.DistanceSquared(entity.Position), entity), false
	})
}

func (w *World) ForEntitiesAndOthers(entityCallback func(entity *world.Entity) (stop bool, radius float32),
	otherCallback func(entity *world.Entity, otherEntity *world.Entity) (stop, remove, removeOther bool)) bool {

	return w.root.iterate(func(entity *world.Entity) (stopFirst, _ bool) {
		var radius float32
		stopFirst, radius = entityCallback(entity)

		if radius <= 0.0 {
			return
		}

		if stopFirst {
			return
		}

		aabb := radiusAABB(entity.Position, radius)
		r2 := radius * radius

		// 'i' can change if entities are removed so lookup with 'i' each time to get entity
		w.root.iterateAABB(aabb, func(other *world.Entity) (stop, _ bool) {
			if entity == other || entity.Position.DistanceSquared(other.Position) > r2 {
				return
			}

			stop, _, _ = otherCallback(entity, other)

			if stop {
				stopFirst = true
			}

			return
		})

		return
	})
}

func (w *World) SetParallel(readOnly bool) bool {
	return true
}

func (w *World) Debug() {
	entityCount := w.Count()
	fmt.Printf("tree world: nodes: %d, entities: %d\n", w.root.nodeCount(), entityCount)
}

func (w *World) Resize(radius float32) {
	// Do nothing
}

func newTreeNode(aabb world.AABB) *treeNode {
	return &treeNode{AABB: aabb}
}

func (node *treeNode) String() string {
	buf, err := json.MarshalIndent(node, "", "\t")
	if err != nil {
		panic(err.Error())
	}
	return string(buf)
}

func (node *treeNode) count() (count int) {
	count += len(node.entities)
	for _, child := range node.children {
		if child == nil {
			continue
		}
		count += child.count()
	}
	return
}

func (node *treeNode) nodeCount() (count int) {
	count = 1
	for _, child := range node.children {
		if child == nil {
			continue
		}
		count += child.nodeCount()
	}
	return
}

func (node *treeNode) iterate(callback func(entity *world.Entity) (stop, remove bool)) bool {
	for i := range node.entities {
		entity := &node.entities[i]
		stop, remove := callback(entity)

		if remove {
			i = node.remove(i)
		}

		if stop {
			return true
		}
	}

	for _, child := range node.children {
		if child == nil {
			continue
		}

		if child.iterate(callback) {
			return true
		}
	}
	return false
}

func (node *treeNode) iterateAABB(aabb world.AABB, callback func(entity *world.Entity) (stop, remove bool)) bool {
	for i := range node.entities {
		entity := &node.entities[i]
		if !entityAABB(entity).Intersects(aabb) {
			continue
		}
		stop, remove := callback(entity)

		if remove {
			i = node.remove(i)
		}

		if stop {
			return true
		}
	}

	for _, child := range node.children {
		if child == nil {
			continue
		}

		if !child.AABB.Intersects(aabb) {
			continue
		}

		if child.iterate(callback) {
			return true
		}
	}
	return false
}

func (node *treeNode) add(entity *world.Entity) {
	node.entities = append(node.entities, *entity)
	if len(node.entities) > treeNodeMaxEntities {
		start := 0
		// Entities already failed subdivision
		if end := len(node.entities) - 1; end > treeNodeMaxEntities {
			start = end
		}
		node.subdivide(start)
	}
}

// Subdivides the node into 4 new nodes and places fitting entities in them
// Starts at the start index
func (node *treeNode) subdivide(start int) {
	quadrants := node.Quadrants()

	j := start
	for i := j; i < len(node.entities); i++ {
		entity := &node.entities[i]
		aabb := entityAABB(entity)

		removed := false
		for k, quad := range quadrants {
			if quad.Contains(aabb) {
				child := node.children[k]
				if child == nil {
					child = newTreeNode(quad)
					node.children[k] = child
				}

				child.add(entity)
				removed = true
				break
			}
		}

		node.entities[i] = node.entities[j]
		if !removed {
			j++
		}
	}
	node.shrink(j)
}

// Removes a node and returns the new iteration index
func (node *treeNode) remove(index int) int {
	end := len(node.entities) - 1
	node.entities[index] = node.entities[end]
	node.entities[end] = world.Entity{} // Clear pointers
	node.shrink(end)
	return index - 1
}

// Re-slices entities length to n and shrinks slice if too much space is remaining
func (node *treeNode) shrink(n int) {
	node.entities = node.entities[:n]
	if len(node.entities) <= cap(node.entities)/3 {
		smallerEntities := make([]world.Entity, len(node.entities))
		copy(smallerEntities, node.entities)
		node.entities = smallerEntities
	}
}

func entityAABB(entity *world.Entity) world.AABB {
	return radiusAABB(entity.Position, entity.Data().Radius)
}

func radiusAABB(position world.Vec2f, radius float32) world.AABB {
	return world.AABB{
		Vec2f:  position,
		Width:  radius * 2,
		Height: radius * 2,
	}.CornerCoordinates()
}
