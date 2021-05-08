// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"github.com/chewxy/math32"
	"github.com/finnbear/moderation"
)

type ChatHistory struct {
	total         float32
	inappropriate float32

	// Content based filtering (packed into 8 bytes)
	recentLengths      [7]uint8
	recentLengthsIndex int8

	// Time last faded out in milliseconds
	updated int64
}

func (hist *ChatHistory) Update(message string) (string, bool) {
	hist.total++
	result := moderation.Scan(message)
	inappropriate := result.Is(moderation.Inappropriate)
	severelyInappropriate := result.Is(moderation.Inappropriate & moderation.Severe)

	var censorAmount int
	if inappropriate {
		// Censor message
		message, censorAmount = moderation.Censor(message, moderation.Inappropriate)
		hist.inappropriate++
	}

	inappropriateFraction := hist.inappropriate / hist.total

	// Length of message capped at 255
	n := uint8(math32.MaxUint8)
	if len(message) < math32.MaxUint8 {
		n = uint8(len(message))
	}

	hist.recentLengths[hist.recentLengthsIndex] = n
	hist.recentLengthsIndex = int8(int(hist.recentLengthsIndex+1) % len(hist.recentLengths))

	averageLength := float32(0)
	for _, length := range hist.recentLengths {
		averageLength += float32(length)
	}
	averageLength /= float32(len(hist.recentLengths))

	// Deviation of this comment
	lengthSpecificDeviation := int(n) - int(averageLength)
	if lengthSpecificDeviation < 0 {
		lengthSpecificDeviation = -lengthSpecificDeviation
	}

	lengthStandardDeviation := float32(0)
	for _, length := range hist.recentLengths {
		deviation := averageLength - float32(length)
		lengthStandardDeviation += deviation * deviation
	}
	lengthStandardDeviation /= float32(len(hist.recentLengths))

	// Count whole number of seconds since last update
	now := unixMillis()
	seconds := (now - hist.updated) / 1000

	if hist.updated == 0 {
		hist.updated = now
	} else if seconds > 0 {
		fadeRate := float32(0.95) // seconds

		// Inappropriate comments fade out slower
		if hist.inappropriate > 5 && inappropriateFraction > 0.5 {
			fadeRate = 0.999999 // days
		} else if hist.inappropriate > 4 && inappropriateFraction > 0.4 {
			fadeRate = 0.99999 // hours
		} else if hist.inappropriate > 3 && inappropriateFraction > 0.3 {
			fadeRate = 0.9999 // minutes
		} else if inappropriateFraction > 0.2 {
			fadeRate = 0.999
		} else if inappropriateFraction > 0.1 {
			fadeRate = 0.99
		}

		fade := math32.Pow(fadeRate, float32(seconds))

		// Fade in equal proportions to not distort inappropriateFraction
		hist.total *= fade
		hist.inappropriate *= fade

		hist.updated = now
	}

	repetitionThresholdTotal := 3
	/*
		if _, ok := repetitionFalsePositives[message]; ok {
			// Permit slightly more repetitions of a limited set of comments
			repetitionThresholdTotal = 5
		}
	*/

	frequencySpam := hist.total >= 10
	inappropriateSpam := hist.inappropriate > 2 && inappropriateFraction > 0.20
	repetitionSpam := int(hist.total) > repetitionThresholdTotal && lengthStandardDeviation < 3 && lengthSpecificDeviation < 3

	block := (inappropriate && censorAmount > 4) || severelyInappropriate || (frequencySpam || inappropriateSpam || repetitionSpam)

	return message, !block
}
