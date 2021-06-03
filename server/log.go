// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package server

import (
	"encoding/csv"
	"fmt"
	"os"
)

func AppendLog(filename string, fields []interface{}) (err error) {
	f, err := os.OpenFile(filename, os.O_APPEND|os.O_WRONLY|os.O_CREATE, 0644)
	if err != nil {
		return
	}
	defer f.Close()

	w := csv.NewWriter(f)

	var fieldStrings []string

	for _, field := range fields {
		var fieldString string

		switch v := field.(type) {
		case float32, float64:
			fieldString = fmt.Sprintf("%.2f", v)
		default:
			fieldString = fmt.Sprint(v)
		}

		fieldStrings = append(fieldStrings, fieldString)
	}

	err = w.Write(fieldStrings)
	if err != nil {
		return
	}

	w.Flush()
	// Error from flush
	err = w.Error()
	if err != nil {
		return
	}
	return
}
