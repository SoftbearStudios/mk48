// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package world

import (
	"reflect"
	"unsafe"
)

type (
	unsafeExtension struct {
		data *unsafeData
	}

	// unsafeData is allocated with extra space for armaments and angles
	// basics documented in extension_safe
	unsafeData struct {
		target    Vec2f
		alt       float32
		altTarget float32
		time      Ticks
		first     [0]float32
		// armaments [?]float32
		// angles    [?]float32
	}
)

var _ = extension(&unsafeExtension{})

func unsafeDataSize(data *EntityTypeData) int {
	return int(unsafe.Sizeof(unsafeData{}) + uintptr(len(data.Armaments))*unsafe.Sizeof(float32(0)) + uintptr(len(data.Turrets))*unsafe.Sizeof(Angle(0)))
}

// setEntityType initializes to a size defined by entityType
func (ext *unsafeExtension) setType(entityType EntityType) {
	data := entityType.Data()
	oldExt := ext.data

	// Allocate enough space for target, time, armaments, and angles
	size := unsafeDataSize(data)
	ext.data = (*unsafeData)(unsafe.Pointer(&make([]byte, size)[0]))

	// Only keep target and target time
	if oldExt != nil {
		ext.data.target = oldExt.target
		ext.data.time = oldExt.time
		ext.data.altTarget = oldExt.altTarget
	}

	angles := ext.turretAngles(entityType)
	for i, turret := range data.Turrets {
		angles[i] = turret.Angle
	}
}

func (ext *unsafeExtension) copiesAll() bool {
	return true
}

// copy reallocates data of same size
func (ext *unsafeExtension) copy(entityType EntityType) {
	data := entityType.Data()
	size := unsafeDataSize(data)

	var src []byte
	header := (*reflect.SliceHeader)(unsafe.Pointer(&src))
	header.Data = uintptr(unsafe.Pointer(ext.data))
	header.Len = size
	header.Cap = size

	dst := make([]byte, len(src))
	copy(dst, src)

	ext.data = (*unsafeData)(unsafe.Pointer(&dst[0]))
}

func (ext *unsafeExtension) armamentConsumption(entityType EntityType) (slice []float32) {
	if n := len(entityType.Data().Armaments); n != 0 {
		header := (*reflect.SliceHeader)(unsafe.Pointer(&slice))
		header.Data = uintptr(unsafe.Pointer(&ext.data.first))
		header.Len = n
		header.Cap = n
	}
	return
}

func (ext *unsafeExtension) copyArmamentConsumption(entityType EntityType) {
	ext.copy(entityType)
}

func (ext *unsafeExtension) turretAngles(entityType EntityType) (slice []Angle) {
	data := entityType.Data()
	if n := len(data.Turrets); n != 0 {
		header := (*reflect.SliceHeader)(unsafe.Pointer(&slice))
		header.Data = uintptr(unsafe.Pointer(&ext.data.first)) + uintptr(len(data.Armaments))*unsafe.Sizeof(float32(0))
		header.Len = n
		header.Cap = n
	}
	return
}

func (ext *unsafeExtension) copyTurretAngles(entityType EntityType) {
	ext.copy(entityType)
}

func (ext *unsafeExtension) altitude() float32 {
	if ext.data == nil {
		return 0
	}
	return ext.data.alt
}

func (ext *unsafeExtension) setAltitude(a float32) {
	ext.data.alt = a
}

func (ext *unsafeExtension) altitudeTarget() float32 {
	if ext.data == nil {
		return 0
	}
	return ext.data.altTarget
}

func (ext *unsafeExtension) setAltitudeTarget(a float32) {
	ext.data.altTarget = a
}

func (ext *unsafeExtension) turretTarget() Vec2f {
	return ext.data.target
}

func (ext *unsafeExtension) setTurretTarget(target Vec2f) {
	ext.data.target = target
}

func (ext *unsafeExtension) turretTargetTime() Ticks {
	if ext.data == nil {
		return 0
	}
	return ext.data.time
}

func (ext *unsafeExtension) setTurretTargetTime(t Ticks) {
	ext.data.time = t
}
