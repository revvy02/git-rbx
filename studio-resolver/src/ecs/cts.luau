--[[
	Component registry — every component/tag the resolver's world uses, in
	one place, after game-prototype's ecs/cts.luau. The world is the source
	of truth for merge + session state: conflicts are entities built from the
	stamped container (ChildOf their target's mirror entity), and session
	state lives as singletons — a component set on its own entity id, read
	with world:get(cts.X, cts.X). The diff layers will grow here:
	per-instance status tags, per-side value components.
]]

local packages = require("../packages")

local world = require("./world")

local jecs = packages.jecs

-- What staging a side means spatially, filled by preview.boot from the
-- stamped Ours/Theirs folders.
export type SideInfo = {
	folder: Instance?, -- the stamped side folder (a swapped-out clone lives here)
	clone: Instance?, -- this side's version (nil = this side deletes the target)
	destination: Instance?, -- MoveTarget: the parent this side moves the target to
	destinationPath: string?,
}

-- Boot snapshot of the base state, restored on unpick (preview.boot).
export type Base = {
	value: any, -- Property: value to restore
	parent: Instance?, -- MoveTarget: parent to restore
	parts: { [BasePart]: number }?, -- DeleteVsEdit: true transparencies
}

local cts = {
	-- The live preview Instance an entity mirrors.
	Ref = world:component(),

	-- Conflict entities, one per stamped container entry.
	Kind = world:component(), -- "Property" | "MoveTarget" | "DeleteVsEdit"
	Path = world:component(), -- display path of the contested instance
	Property = world:component(), -- Property kind: the contested property name
	Target = world:component(), -- the contested live Instance
	Entry = world:component(), -- the stamped container entry (Name = CLI --entry)
	Order = world:component(), -- stable list position (entry-name sort)
	Ours = world:component(), -- SideInfo
	Theirs = world:component(), -- SideInfo
	Base = world:component(), -- Base snapshot
	Resolution = world:component(), -- staged side "ours"|"theirs"|"custom"; absent = unpicked
	Rendered = world:component(), -- staged state the 3D layer currently reflects

	-- Session singletons.
	ViewTab = world:component(), -- "base" | "ours" | "theirs" | "result"
	SelectedConflict = world:component(), -- conflict entity
	SelectedInstance = world:component(), -- Instance the explorer/props focus on
	CompleteError = world:component(), -- last Complete failure message
	Completing = world:entity(), -- tag: CLI batch in flight
	Finished = world:entity(), -- tag: finalized, file written
}

for name, ct in cts do
	world:set(ct, jecs.Name, name)
end

return cts
