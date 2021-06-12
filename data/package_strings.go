// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"bytes"
	"encoding/json"
	"log"
	"os"
)

// This file compiles all strings files into one

func main() {
	buf, err := os.ReadFile("strings.json")
	if err != nil {
		log.Fatal(err)
	}
	var main map[string]interface{}
	err = json.Unmarshal(buf, &main)
	if err != nil {
		log.Fatal(err)
	}

	translations := make(map[string]interface{})

	translations["en"] = main

	var out bytes.Buffer
	enc := json.NewEncoder(&out)
	enc.SetEscapeHTML(false)
	enc.SetIndent("", "	")

	err = enc.Encode(translations)
	if err != nil {
		log.Fatal(err)
	}
	err = os.WriteFile("../client/src/data/strings.json", out.Bytes(), 0644)
	if err != nil {
		log.Fatal(err)
	}
}
