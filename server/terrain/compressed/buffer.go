// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package compressed

import "io"

// Buffer writes 4 bits of a byte and uses run length encoding
// Each byte is 4 bits of data followed by 4 bits of count - 1
type Buffer struct {
	buf []byte
	off int // Read position
}

func (buffer *Buffer) Reset(buf []byte) {
	buffer.buf = buf
	buffer.off = 0
}

// Encodes a byte as its 4 most significant bits
func (buffer *Buffer) writeByte(b byte) {
	buf := buffer.buf

	// Nibble
	next := b >> 4

	var current, countMinusOne, tuple byte
	end := len(buf) - 1

	const maxCount = 15
	if len(buf) > 0 {
		tuple = buf[end]
		current = tuple >> 4
		countMinusOne = tuple & maxCount
	} else {
		countMinusOne = maxCount // Full
	}

	if next != current || countMinusOne == maxCount {
		// Start new tuple
		tuple = next << 4
		buf = append(buf, tuple)
	} else {
		// Add 1 to count
		buf[end] = tuple + 1
	}

	buffer.buf = buf
}

func (buffer *Buffer) Write(buf []byte) (int, error) {
	for _, b := range buf {
		buffer.writeByte(b)
	}
	return len(buf), nil
}

func (buffer *Buffer) readByte() (b byte, more bool) {
	tuple := buffer.buf[buffer.off]
	b = tuple & 0b11110000

	if tuple&15 > 0 {
		buffer.buf[buffer.off] = tuple - 1
		more = true
	} else {
		buffer.off++
		more = buffer.off < len(buffer.buf)
	}

	return
}

func (buffer *Buffer) Read(buf []byte) (int, error) {
	more := len(buffer.buf) > 0
	i := 0

	for ; i < len(buf) && more; i++ {
		buf[i], more = buffer.readByte()
	}

	if i == 0 {
		return 0, io.EOF
	}

	return i, nil
}

// Grow makes space for about n elements
func (buffer *Buffer) Grow(n int) {
	compressed := n / 2
	if old := buffer.Buffer(); len(old) < compressed {
		buf := make([]byte, len(old), len(old)+compressed)
		copy(buf, old)
		buffer.buf = buf
	}
}

func (buffer *Buffer) Buffer() []byte {
	return buffer.buf[buffer.off:]
}
