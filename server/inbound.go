// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

package main

import (
	"github.com/chewxy/math32"
	"github.com/finnbear/moderation"
	"math/rand"
	"mk48/server/world"
	"strings"
	"time"
	"unicode"
	"unicode/utf8"
)

// Make sure to register in init function
type (
	// AddToTeam allows Owners to add members that are requesting and
	// outsiders to request to join.
	AddToTeam struct {
		TeamID   world.TeamID   `json:"teamID"`
		PlayerID world.PlayerID `json:"playerID"`
	}

	// AimTurrets sets your ship's TurretTarget.
	AimTurrets struct {
		Target world.Vec2f `json:"target"`
	}

	// CreateTeam creates a new team
	CreateTeam struct {
		Name string `json:"name"`
	}

	// Fire fires an armament
	Fire struct {
		world.Guidance
		PositionTarget world.Vec2f      `json:"positionTarget"`
		Index          int              `json:"index"`
		Type           world.EntityType `json:"type"`
	}

	// InvalidInbound means invalid message type from client (possibly out of date).
	// NOTE: Do not register, otherwise client could send type "invalidInbound"
	InvalidInbound struct {
		messageType messageType
	}

	// Manual controls your ship's Guidance and optionally sets a TurretTarget.
	// TODO embed AimTurrets
	Manual struct {
		world.Guidance
		AltitudeTarget *float32       `json:"altitudeTarget"`
		TurretTarget   world.Vec2f    `json:"turretTarget"`
		EntityID       world.EntityID `json:"entityID"`
	}

	// RemoveFromTeam either kicks someone from your team if you are Owner or leaves your team.
	RemoveFromTeam struct {
		PlayerID world.PlayerID `json:"playerID"`
	}

	// SendChat sends a chat message to global chat.
	SendChat struct {
		Message string `json:"message"`
		Team    bool   `json:"team"`
	}

	// Spawn spawns your ship.
	Spawn struct {
		Name string           `json:"name"`
		Type world.EntityType `json:"type"`
		Auth string           `json:"auth"`   // Auth unlocks certain names and removes some checks
		Code world.TeamCode   `json:"invite"` // Code automatically adds the Player to the team with that code
		New  bool             `json:"new"`    // Whether first time playing
	}

	// Trace sends debug info.
	Trace struct {
		FPS float32 `json:"fps"`
	}

	// Upgrade upgrades your ship.
	Upgrade struct {
		Type world.EntityType `json:"type"`
	}
)

func init() {
	registerInbound(
		AddToTeam{},
		AimTurrets{},
		CreateTeam{},
		Fire{},
		Manual{},
		RemoveFromTeam{},
		SendChat{},
		Spawn{},
		Trace{},
		Upgrade{},
	)
}

var reservedNames = [...]string{
	"admin",
	"administrator",
	"console",
	"editor",
	"dev",
	"developer",
	"mod",
	"moderator",
	"npc",
	"owner",
	"root",
	"server",
	"staff",
	"system",
}

func (data AddToTeam) Inbound(h *Hub, _ Client, player *Player) {
	teamID := data.TeamID
	if teamID == world.TeamIDInvalid {
		teamID = player.TeamID
	}

	playerID := data.PlayerID
	if playerID == world.PlayerIDInvalid {
		playerID = player.PlayerID()
	}

	team := h.teams[teamID]
	if team == nil {
		return
	}

	// Owner moves people from JoinRequests to Members
	// Everyone else can add themself to JoinRequests
	if team.Owner() == &player.Player {
		if len(team.Members) >= world.TeamMembersMax {
			return // Team is full
		}

		joiningPlayer := team.JoinRequests.GetByID(playerID)
		if joiningPlayer == nil {
			return
		}

		// Shouldn't happen ever
		if joiningPlayer.TeamID != world.TeamIDInvalid {
			panic("player with active join request already in team")
		}

		// Removes for all teams including this one
		h.clearTeamRequests(joiningPlayer)
		team.Members.Add(joiningPlayer)

		joiningPlayer.TeamID = player.TeamID
	} else if player.PlayerID() == playerID {
		if len(team.Members)+len(team.JoinRequests) >= world.TeamMembersMax {
			return // Team with requests is full
		}

		// Player must be data.PlayerID's player
		if player.TeamID != world.TeamIDInvalid {
			return // Already on a team
		}

		team.JoinRequests.Add(&player.Player)
	} // else possibly needed for voting system in future
}

func (data CreateTeam) Inbound(h *Hub, _ Client, player *Player) {
	if player.TeamID != world.TeamIDInvalid {
		return // Already on a team
	}

	// Validate name
	name, ok := sanitize(data.Name, true, world.TeamIDLengthMin, world.TeamIDLengthMax)
	// Invalid name
	if !ok {
		return
	}

	var teamID world.TeamID
	err := teamID.UnmarshalText([]byte(name))
	// Shouldn't happen because of validation
	if err != nil {
		panic(err.Error())
	}

	if existingTeam := h.teams[teamID]; existingTeam != nil {
		return // Team already exists
	}

	player.TeamID = teamID
	h.clearTeamRequests(&player.Player)
	team := &Team{}
	team.Create(&player.Player)
	h.teams[teamID] = team
}

func (data RemoveFromTeam) Inbound(h *Hub, _ Client, player *Player) {
	team := h.teams[player.TeamID]

	if team == nil {
		return
	}

	// Default to removing yourself
	if data.PlayerID == world.PlayerIDInvalid {
		data.PlayerID = player.PlayerID()
	}

	removePlayer := team.Members.GetByID(data.PlayerID)

	// You can remove yourself or other if you are owner
	if removePlayer != nil && (&player.Player == team.Owner() || &player.Player == removePlayer) {
		h.leaveTeam(removePlayer)
	}
}

func (data Spawn) Inbound(h *Hub, client Client, player *Player) {
	h.world.EntityByID(player.EntityID, func(oldShip *world.Entity) (_ bool) {
		if oldShip != nil {
			return // can only have one ship
		}

		authed := h.auth != "" && data.Auth == h.auth

		if d := data.Type.Data(); d.Kind != world.EntityKindBoat || (d.Level != 1 && !authed) {
			return
		}

		name, ok := sanitize(data.Name, true, world.PlayerNameLengthMin, world.PlayerNameLengthMax)
		// Invalid name
		if !ok {
			return
		}

		if !authed {
			lower := strings.ToLower(name)
			for _, reservedName := range reservedNames {
				if lower == reservedName {
					println("blocked reserved name", name)
					return // reserved
				}
			}
		}
		player.Name = name
		player.DeathMessage = ""

		// Team codes
		if code := data.Code; code != world.TeamCodeInvalid && player.TeamID == world.TeamIDInvalid {
			for teamID, team := range h.teams {
				if team.Code == code {
					if len(team.Members) < world.TeamMembersMax {
						h.clearTeamRequests(&player.Player)
						team.Members.Add(&player.Player)
						player.TeamID = teamID
					}
					break
				}
			}
		}

		entity := &world.Entity{
			Owner: &player.Player,
		}

		entity.Initialize(data.Type)
		spawnRadius := h.worldRadius * 0.6

		if team := h.teams[player.TeamID]; team != nil {
			// Spawn near the first other team member with a ship
			for _, member := range team.Members {
				if member == &player.Player {
					continue
				}

				var spawned bool

				h.world.EntityByID(member.EntityID, func(memberShip *world.Entity) (_ bool) {
					if memberShip == nil {
						return
					}
					entity.Position = memberShip.Position
					spawnRadius = 150
					spawned = true
					return
				})

				if spawned {
					break
				}
			}
		}

		h.spawnEntity(entity, spawnRadius)

		if _, bot := client.(*BotClient); !bot {
			h.cloud.IncrementPlaysStatistic()
			if data.New {
				h.cloud.IncrementNewPlayerStatistic()
			}
		}

		return
	})
}

func (data Upgrade) Inbound(h *Hub, _ Client, player *Player) {
	h.world.EntityByID(player.EntityID, func(oldShip *world.Entity) (_ bool) {
		if oldShip == nil {
			return // hasn't spawned yet
		}

		newShipType := data.Type
		if !oldShip.UpgradesTo(newShipType, player.Score) {
			return
		}

		oldShip.Initialize(newShipType)
		return
	})
}

func (data AimTurrets) Inbound(h *Hub, _ Client, player *Player) {
	h.world.EntityByID(player.EntityID, func(entity *world.Entity) (_ bool) {
		if entity == nil || entity.Owner != &player.Player {
			return
		}

		entity.SetTurretTarget(data.Target)
		return
	})
}

func (data Fire) Inbound(h *Hub, _ Client, player *Player) {
	h.world.EntityByID(player.EntityID, func(entity *world.Entity) (_ bool) {
		if entity == nil || entity.Owner != &player.Player {
			return
		}

		shipData := entity.Data()
		if data.Index >= len(shipData.Armaments) {
			return
		}

		if entity.ArmamentConsumption()[data.Index] != 0 {
			return
		}

		armamentData := shipData.Armaments[data.Index]
		armamentEntityData := armamentData.Default.Data()

		transform := entity.ArmamentTransform(data.Index)

		if armamentEntityData.SubKind == world.EntitySubKindDredger {
			// TODO find another way to calculate this
			if data.PositionTarget.DistanceSquared(transform.Position) > 60 {
				return
			}
			h.terrain.Sculpt(data.PositionTarget, 40)
		} else {
			armamentGuidance := data.Guidance

			// Calculate angle on server, since Transform math is present
			armamentGuidance.DirectionTarget = data.PositionTarget.Sub(transform.Position).Angle()

			if armamentGuidance.VelocityTarget == 0 || armamentData.Subtype == world.EntitySubKindShell ||
				armamentData.Subtype == world.EntitySubKindMissile {

				armamentGuidance.VelocityTarget = armamentEntityData.Speed
			}

			// Start distance/lifespan at 0 seconds, with few exceptions
			var lifespan world.Ticks

			if armamentData.Airdrop {
				const airdropRange = 500
				if data.PositionTarget.DistanceSquared(entity.Position) > airdropRange*airdropRange {
					// Exceeded max range
					return
				}
				// Drop the torpedo a bit away, pointed towards the target
				transform.Direction = world.Angle(rand.Float32() * math32.Pi * 2)
				transform.Position = entity.TurretTarget().AddScaled(transform.Direction.Vec2f(), -float32(50+rand.Intn(50)))
				armamentGuidance.DirectionTarget = transform.Direction

				// Start the lifespan near expiry to make these torpedoes not last long
				const maxLifespan = 10 * world.TicksPerSecond
				lifespan = armamentData.Default.ReducedLifespan(maxLifespan)
			} else if armamentData.Vertical {
				// Vertically-launched armaments can be launched in any horizontal direction
				transform.Direction = armamentGuidance.DirectionTarget
			}

			if armamentEntityData.SubKind == world.EntitySubKindRocket {
				transform.Direction += world.ToAngle((rand.Float32() - 0.5) * 0.1)
			}

			armamentEntity := &world.Entity{
				EntityType: armamentData.Default,
				Owner:      &player.Player,
				Transform:  transform,
				Guidance:   armamentGuidance,
				Lifespan:   lifespan,
			}

			h.spawnEntity(armamentEntity, 0)
		}

		entity.ConsumeArmament(data.Index)

		return
	})
}

func (data Manual) Inbound(h *Hub, _ Client, player *Player) {
	h.world.EntityByID(data.EntityID, func(entity *world.Entity) (_ bool) {
		if entity == nil || entity.Owner != &player.Player {
			return
		}

		entity.Guidance = data.Guidance

		if data.AltitudeTarget != nil {
			entity.SetAltitudeTarget(*data.AltitudeTarget)
		}

		entity.SetTurretTarget(data.TurretTarget)

		return
	})
}

func (data SendChat) Inbound(h *Hub, client Client, player *Player) {
	if len(data.Message) > 128 {
		return
	}

	name := player.Name

	// Allow spamming ones own team, since you can get kicked
	msg, ok := player.ChatHistory.Update(data.Message, data.Team)

	t := "user"
	if _, bot := client.(*BotClient); bot {
		t = "bot"
	}

	_ = AppendLog("/tmp/mk48-chat.log", []interface{}{
		time.Now().UnixNano() / 1e6,
		!ok,
		name,
		t,
		data.Message,
		msg,
	})

	if !ok {
		return
	}

	msg, ok = sanitize(msg, false, 1, 128)
	if !ok {
		return
	}

	chat := Chat{Message: msg, PlayerData: player.PlayerData}
	if data.Team {
		team := h.teams[player.TeamID]
		if team == nil {
			return
		}
		team.Chats = append(team.Chats, chat)
	} else {
		h.chats = append(h.chats, chat)
	}
}

func (trace Trace) Inbound(_ *Hub, _ Client, p *Player) {
	if trace.FPS <= 0 {
		return
	}

	// Clamp to 60 for people possibly playing above to not pollute average
	if trace.FPS > 60 {
		trace.FPS = 60
	}

	p.FPS = trace.FPS

	_ = AppendLog("/tmp/mk48-trace.log", []interface{}{
		time.Now().UnixNano() / 1e6,
		p.Name,
		trace.FPS,
	})
}

func (data InvalidInbound) Inbound(_ *Hub, _ Client, _ *Player) {}

func trimUtf8(in string, low, high int) (str string, ok bool) {
	if !utf8.ValidString(in) {
		return "", false
	}

	// Remove spaces
	str = strings.TrimSpace(in)
	str = strings.TrimFunc(str, func(r rune) bool {
		// NOTE: The following characters are not detected by
		// unicode.IsSpace() but show up as blank

		// https://www.compart.com/en/unicode/U+2800
		// https://www.compart.com/en/unicode/U+200B
		return r == 0x2800 || r == 0x200B
	})

	// Too long but can resize down
	if len(str) > high {
		var builder strings.Builder
		for _, r := range str {
			if builder.Len()+utf8.RuneLen(r) > high {
				break
			}
			builder.WriteRune(r)
		}
		str = builder.String()
	}

	// Too short
	if len(str) < low {
		return "", false
	}
	ok = true
	return
}

func sanitize(text string, name bool, low, high int) (string, bool) {
	if name {
		// Remove these characters
		// Brackets are used in formatting
		// * is used for censoring
		const removals = "()[]{}*"
		for i := 0; i < len(removals); i++ {
			text = strings.ReplaceAll(text, removals[i:i+1], "")
		}
	}

	text = strings.Map(func(r rune) rune {
		if unicode.IsPrint(r) || unicode.IsGraphic(r) {
			return r
		}
		return -1
	}, text)

	text, ok := trimUtf8(text, low, high)
	if !ok {
		return "", false
	}

	if name {
		// Censor name
		result := moderation.Scan(text)

		if result.Is(moderation.Inappropriate) {
			if result.Is(moderation.Inappropriate & moderation.Moderate) {
				return "", false
			}
			text, _ = moderation.Censor(text, moderation.Inappropriate)
		}
	}

	return text, true
}
