// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"os"
	"sort"
)

// This is an internal script used to translate entity count logs from server
// format to public format

func main() {
	// File sourced from server filesystem
	f, err := os.Open("mk48-entities.log")
	if err != nil {
		log.Fatal(err)
	}
	r := csv.NewReader(f)

	// Get all types present in CSV
	entityTypesSet := make(map[string]struct{})

	for {
		record, err := r.Read()
		if err != nil {
			if err == io.EOF {
				break
			}
			log.Fatal(err)
		}

		var counts map[string]int
		err = json.Unmarshal([]byte(record[1]), &counts)
		if err != nil {
			log.Fatal(err)
		}
		for t, _ := range counts {
			entityTypesSet[t] = struct{}{}
		}
	}

	entityTypes := make([]string, 0, len(entityTypesSet))

	for t, _ := range entityTypesSet {
		entityTypes = append(entityTypes, t)
	}

	sort.Strings(entityTypes)

	f.Close()

	f, err = os.Open("mk48-entities.log")
	if err != nil {
		log.Fatal(err)
	}
	r = csv.NewReader(f)

	o, err := os.Create("mk48-entities.csv")
	if err != nil {
		log.Fatal(err)
	}
	w := csv.NewWriter(o)

	// Header
	var header []string
	header = append(header, "timestamp")
	header = append(header, entityTypes...)
	w.Write(header)

outer:
	for {
		var fields []string
		var counts = make(map[string]int)

		// Condense data
		const group = 1000
		for i := 0; i < group; i++ {
			record, err := r.Read()
			if err != nil {
				if err == io.EOF {
					break outer
				}
				log.Fatal(err)
			}

			if i == 0 {
				fields = append(fields, record[0])
			}

			var subCounts map[string]int
			err = json.Unmarshal([]byte(record[1]), &subCounts)
			if err != nil {
				log.Fatal(err)
			}

			for t, c := range subCounts {
				counts[t] += c
			}
		}

		for _, t := range entityTypes {
			fields = append(fields, fmt.Sprint(float32(counts[t])/group))
		}

		w.Write(fields)
	}

	f.Close()
	o.Close()
}
