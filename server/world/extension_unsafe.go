// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"reflect"
	"unsafe"
)

type (
	// unsafeData is allocated with extra space for armaments and angles
	// basics documented in extension_safe
	unsafeExtension struct {
		data            *uint16
		target          Vec2f
		alt             float32
		altTarget       float32
		spawnProtection Ticks
		typ             EntityType
	}
)

var _ = extension(&unsafeExtension{})

func init() {
	if unsafe.Sizeof(Angle(0)) != 2 || unsafe.Sizeof(Ticks(0)) != 2 {
		panic("unsafe extension must be updated")
	}
}

func unsafeDataLen(data *EntityTypeData) int {
	return len(data.Armaments) + len(data.Turrets)
}

// setEntityType initializes to a size defined by entityType
func (ext *unsafeExtension) setType(entityType EntityType) {
	data := entityType.Data()

	// Allocate enough space for armaments, and turret angles
	size := unsafeDataLen(data)
	if size == 0 {
		ext.data = nil
	} else {
		ext.data = &make([]uint16, size)[0]
	}

	ext.typ = entityType
	angles := ext.turretAngles()
	for i, turret := range data.Turrets {
		angles[i] = turret.Angle
	}
}

func (ext *unsafeExtension) copiesAll() bool {
	return true
}

// copy reallocates data of same size
func (ext *unsafeExtension) copy() {
	data := ext.typ.Data()
	size := unsafeDataLen(data)

	var src []uint16
	header := (*reflect.SliceHeader)(unsafe.Pointer(&src))
	header.Data = uintptr(unsafe.Pointer(ext.data))
	header.Len = size
	header.Cap = size

	dst := make([]uint16, len(src))
	copy(dst, src)

	ext.data = &dst[0]
}

func (ext *unsafeExtension) armamentConsumption() (slice []Ticks) {
	if n := len(ext.typ.Data().Armaments); n != 0 {
		header := (*reflect.SliceHeader)(unsafe.Pointer(&slice))
		header.Data = uintptr(unsafe.Pointer(ext.data))
		header.Len = n
		header.Cap = n
	}
	return
}

func (ext *unsafeExtension) copyArmamentConsumption() {
	ext.copy()
}

func (ext *unsafeExtension) turretAngles() (slice []Angle) {
	data := ext.typ.Data()
	if n := len(data.Turrets); n != 0 {
		header := (*reflect.SliceHeader)(unsafe.Pointer(&slice))
		header.Data = uintptr(unsafe.Pointer(ext.data)) + uintptr(len(data.Armaments))*unsafe.Sizeof(Ticks(0))
		header.Len = n
		header.Cap = n
	}
	return
}

func (ext *unsafeExtension) copyTurretAngles() {
	ext.copy()
}

func (ext *unsafeExtension) altitude() float32 {
	return ext.alt
}

func (ext *unsafeExtension) setAltitude(a float32) {
	ext.alt = a
}

func (ext *unsafeExtension) altitudeTarget() float32 {
	return ext.altTarget
}

func (ext *unsafeExtension) setAltitudeTarget(a float32) {
	ext.altTarget = a
}

func (ext *unsafeExtension) aimTarget() Vec2f {
	return ext.target
}

func (ext *unsafeExtension) setAimTarget(target Vec2f) {
	ext.target = target
}

func (ext *unsafeExtension) getSpawnProtection() Ticks {
	return ext.spawnProtection
}

func (ext *unsafeExtension) setSpawnProtection(val Ticks) {
	ext.spawnProtection = val
}
