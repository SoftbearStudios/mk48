package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/exec"
	"sort"
)

type Sound struct {
	// Source file relative path
	Source string

	// Trimming
	Start float64
	End   float64
}

var sounds = map[string]Sound{
	"alarmSlow": {
		Source: "freesound.org/165504__ryanconway__missile-lock-detected.mp3",
		End:    2.1,
	},
	"alarmFast": {
		Source: "freesound.org/189327__alxy__missile-lock-on-sound.mp3",
		End:    0.641,
	},
	"ocean": {
		Source: "freesound.org/372181__amholma__ocean-noise-surf.wav",
	},
	"dive": {
		Source: "freesound.org/480002__craigsmith__r18-31-old-car-ahooga-horn.wav",
		Start:  2.85,
		End:    4.75,
	},
	/*
	"dive1": {
		Source: "freesound.org/156672__mkjunker__dive.wav",
		Start:  2.3,
		End:    4.4,
	},
	*/
	"horn": {
		Source: "freesound.org/532339__reznik-krkovicka__horn-mild.mp3",
		Start: 1.328,
		End: 5.588,
	},
	"sonar0": {
		Source: "freesound.org/90340__digit-al__sonar.wav",
		End:    5,
	},
	"sonar1": {
		Source: "freesound.org/493162__breviceps__submarine-sonar.wav",
		Start:  0.184,
		End:    1.964,
	},
	"sonar2": {
		Source: "freesound.org/38702__elanhickler__archi-sonar-03.wav",
		End:    2.5,
	},
	"sonar3": {
		Source: "freesound.org/70299__kizilsungur__sonar.wav",
	},
	"surface": {
		Source: "freesound.org/416079__davidlay1__shaving-cream-can-release.wav",
		End: 2,
	},
}

func main() {
	var manifest []string

	for name, sound := range sounds {
		Encode(name, sound)
		manifest = append(manifest, name)
	}

	sort.Strings(manifest)

	buf, _ := json.MarshalIndent(manifest, "", "\t")
	err := os.WriteFile("../../client/src/data/sounds.json", buf, 0644)
	if err != nil {
		log.Fatal(err)
	}
}

func Encode(name string, sound Sound) {
	args := []string{
		"-i", sound.Source,
		"-vn",
	}

	if sound.Start != 0 {
		args = append(args,
			"-ss", fmt.Sprint(sound.Start),
		)
	}

	if sound.End != 0 {
		args = append(args,
			"-to", fmt.Sprint(sound.End),
		)
	}

	args = append(args,
		"-ab", "128k",
		"-ar", "44100",
		"-filter:a", "loudnorm",
		"-y", // overwrite output file
		fmt.Sprintf("../../client/static/sounds/%s.mp3", name),
	)

	cmd := exec.Command("ffmpeg", args...)
	err := cmd.Run()
	if err != nil {
		log.Fatal(err)
	}
}
