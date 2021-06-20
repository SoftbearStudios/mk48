package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"log"
	"math"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
)

type Sound struct {
	// Source file relative path
	Source string

	// Trimming
	Start float64
	End   float64

	// Adjustments (negative decreases, positive increases)
	Volume float64
	Pitch  float64
}

var sounds = map[string]Sound{
	/*
		"new/6": {
			Source: "freesound.org/4366__qubodup__military-sounds/169743__qubodup__m1-abrams-tank-engine-and-shots-wombzerncci.flac",
			End: 0,
		},
		"new/7": {
			Source: "freesound.org/4366__qubodup__military-sounds/67477__qubodup__soft-explosion-2.flac",
			End: 0,
		},
		"new/8": {
			Source: "freesound.org/4366__qubodup__military-sounds/67479__qubodup__soft-explosion-3.flac",
			End: 0,
		},
	*/
	"aircraft": {
		Source: "freesound.org/479512__craigsmith__r03-04-airplane-engine-steady.wav",
		Start:  3.745,
		End:    6.420,
	},
	"alarmSlow": {
		Source: "freesound.org/165504__ryanconway__missile-lock-detected.mp3",
		End:    2.1,
	},
	"alarmFast": {
		Source: "freesound.org/189327__alxy__missile-lock-on-sound.mp3",
		End:    0.641,
	},
	"collect": {
		Source: "freesound.org/512216__saviraz__coins.mp3",
		Start:  0.065,
		End:    0.267,
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
	"explosionShort": {
		Source: "freesound.org/514647__david2317__03-gran-explosion.wav",
		Start:  2.471,
		Volume: -2,
	},
	"explosionLong": {
		Source: "freesound.org/235968__tommccann__explosion-01.wav",
		Start:  0.317,
		End:    6,
		Volume: -2,
	},
	"horn": {
		Source: "freesound.org/532339__reznik-krkovicka__horn-mild.mp3",
		Start:  1.328,
		End:    5.588,
	},
	"impact": {
		Source: "freesound.org/4366__qubodup__military-sounds/67468__qubodup__howitzer-gun-impacts-1.flac",
		End:    0,
	},
	"ocean": {
		Source: "freesound.org/372181__amholma__ocean-noise-surf.wav",
	},
	"rocket": {
		Source: "freesound.org/4366__qubodup__military-sounds/67541__qubodup__bgm-71-tow-missile-launch-1.flac",
		Volume: -1,
	},
	"shell0": {
		Source: "freesound.org/4366__qubodup__military-sounds/168707__qubodup__tank-firing.flac",
		Start:  0.045,
		End:    0.754,
		Volume: 1,
	},
	"shell1": {
		Source: "freesound.org/4366__qubodup__military-sounds/175430__qubodup__excalibur-howitzer-shot.flac",
		Start:  0.047,
	},
	"shell2": {
		Source: "freesound.org/4366__qubodup__military-sounds/162365__qubodup__navy-battleship-soundscape-turret-gunshots-mechanical-engine-humm-radio-chatter-officer-command-voices.flac",
		Start:  0.057,
		End:    2,
		Volume: -0.5,
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
		End:    2,
	},
	"torpedoLaunch": {
		Source: "freesound.org/367125__jofae__air-hiss.mp3",
		Pitch:  -0.25,
		Volume: -2,
	},
	"upgrade": {
		Source: "freesound.org/209772__johnnyfarmer__metal-boom.aiff",
		Start:  0.2,
		End:    1.66,
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

	var credits bytes.Buffer
	credited := map[string]struct{}{}

	fmt.Fprintf(&credits, "# Sound Credits\n\nSounds are licensed under CC0/public domain\n\n")

	for _, name := range manifest {
		sound := sounds[name]
		if strings.HasPrefix(sound.Source, "freesound.org") {
			segments := strings.Split(sound.Source, "/")
			parts := strings.Split(segments[1], "__")
			if _, already := credited[parts[2]]; already {
				continue
			}
			credited[parts[2]] = struct{}{}
			t := "s" // sound
			if len(segments) > 2 {
				t = "p" // pack
			}
			fmt.Fprintf(&credits, " - %s: [%s](https://freesound.org/%s/%s/) by %s\n", name, parts[2], t, parts[0], parts[1])
		}
	}

	err = os.WriteFile("./README.md", credits.Bytes(), 0644)
	if err != nil {
		log.Fatal(err)
	}

	// Find unused sounds
	err = filepath.Walk(".", func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if strings.HasPrefix(path, ".") || strings.HasSuffix(path, ".go") || strings.HasSuffix(path, ".md") {
			return nil
		}
		found := false
		for _, sound := range sounds {
			if sound.Source == path {
				found = true
				break
			}
		}
		if !found {
			fmt.Printf("%s is unused\n", path)
		}
		return nil
	})
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

	pitchFactor := math.Pow(2, sound.Pitch)
	const samples = 44100

	args = append(args,
		"-ab", "128k",
		"-ac", "1", // single channel audio
		//"-ar", "44100",
		"-af", fmt.Sprintf("volume=%.02f,asetrate=%d,aresample=%d,atempo=%.02f", math.Pow(2, sound.Volume), int(samples*pitchFactor), samples, 1/pitchFactor),
		"-y", // overwrite output file
		fmt.Sprintf("../../client/static/sounds/%s.mp3", name),
	)

	cmd := exec.Command("ffmpeg", args...)
	buf, err := cmd.CombinedOutput()
	if err != nil {
		log.Println(args)
		log.Println(string(buf))
		log.Fatal(err)
	}
}
