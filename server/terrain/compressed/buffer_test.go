// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package compressed

import (
	"bytes"
	"math/rand"
	"testing"
)

func TestCompressedBuffer_Write(t *testing.T) {
	const n = 1024
	var buffer Buffer

	_, _ = buffer.Write(make([]byte, n))

	if buf := buffer.Buffer(); len(buf) != n/16 {
		t.Error("Buffer.Write(make([]byte, 1024) expected", n/16, "got", len(buf))
		t.Error(buf)
	}
}

func TestCompressedBuffer_Read(t *testing.T) {
	const n = 1024
	var buffer Buffer

	input := make([]byte, n)
	for i := range input {
		input[i] = roundByte(byte(rand.Intn(256)))
	}

	_, _ = buffer.Write(input)

	output := make([]byte, n*2)
	r, _ := buffer.Read(output)
	output = output[:r]

	if !bytes.Equal(input, output) {
		t.Error("Buffer.Read expected", len(input), "got", len(output), "\ninput:", input, "\noutput:", output)
	}
}
