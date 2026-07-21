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
	clone: Instance?, -- this side's edited version (absent for deletes and moves)
	deleted: boolean?, -- this side removes the subtree (stamped Deleted attribute)
	destination: Instance?, -- the parent this side moves the target to
	destinationPath: string?,
	delta: CFrame?, -- ModelFrame: canonical/base content -> this side's world frame
}

-- Boot snapshot of the base state, restored on unpick (preview.boot).
export type Base = {
	value: any, -- Property: value to restore
	parent: Instance?, -- MoveTarget: parent to restore
	parts: { [BasePart]: number }?, -- DeleteVsEdit: true transparencies
	frameParts: { [BasePart]: CFrame }?, -- ModelFrame: canonical world transforms
	frameModels: { [Model]: CFrame }?, -- ModelFrame: canonical pivots
}

-- One occurrence in a projected Explorer view. `subject` and `entity` retain
-- canonical merged-DOM identity across views; `source` is the concrete version
-- this occurrence presents. Virtual rows can omit all three.
export type ExplorerNode = {
	source: Instance?,
	subject: Instance?,
	entity: any?,
	name: string,
	className: string?,
	phantom: boolean?,
	parent: ExplorerNode?,
	children: { ExplorerNode },
}

export type ExplorerProjection = {
	roots: { ExplorerNode },
	index: { [Instance]: ExplorerNode },
}

local cts = {
	-- The live preview Instance an entity mirrors.
	Ref = world:component(),

	-- Mirror-derived tree data, written by the mirror system each sync:
	-- ordered child entities per node, plus singletons for the root entity
	-- list and the Instance -> entity index.
	Children = world:component(),
	Roots = world:component(),
	EntityOf = world:component(),
	MirrorGeneration = world:component(), -- bumps when a sync changes anything; readers key caches on it

	-- Explorer projections are immutable, view-scoped tree tables. Canonical
	-- mirror entities remain shared identity; per-view parentage lives in the
	-- table, avoiding an ECS entity and several relations for every visible row.
	ExplorerProjection = world:component(),
	ExplorerGeneration = world:component(),
	ExplorerBaseView = world:entity(),
	ExplorerOursView = world:entity(),
	ExplorerTheirsView = world:entity(),
	ExplorerResultView = world:entity(),

	-- Conflict entities, one per stamped container entry.
	Kind = world:component(), -- "Property" | "MoveTarget" | "DeleteVsEdit" | "ModelFrame"
	Path = world:component(), -- display path of the contested instance
	Property = world:component(), -- Property kind: the contested property name
	Target = world:component(), -- the contested live Instance
	Entry = world:component(), -- the stamped container entry (Name = CLI --entry)
	Order = world:component(), -- stable list position (entry-name sort)
	Ours = world:component(), -- SideInfo
	Theirs = world:component(), -- SideInfo
	Base = world:component(), -- Base snapshot
	Resolution = world:component(), -- staged side "ours"|"theirs"|"custom"; absent = unpicked
	Rendered = world:component(), -- staged state the 3D preview currently reflects
	Highlighted = world:component(), -- staged state the 3D highlight currently reflects

	-- Rigid-group entities (one per stamped Group_N): metadata over members.
	Members = world:component(), -- member conflict entity array
	Group = world:component(), -- member -> owning group entry name
	DeltaOurs = world:component(), -- CFrame world delta this side applies
	DeltaTheirs = world:component(),

	-- Session singletons.
	Conflicts = world:component(), -- boot-sorted conflict entity array (immutable after boot)
	Cards = world:component(), -- display list: group entities + ungrouped conflicts
	Container = world:component(), -- the stamped __RbxDiffMerge container
	ViewTab = world:component(), -- "base" | "ours" | "theirs" | "result"
	SelectedConflict = world:component(), -- conflict entity
	SelectedInstance = world:component(), -- Instance the explorer/props focus on
	CompleteError = world:component(), -- last Complete failure message
	Completing = world:entity(), -- tag: CLI batch in flight
	Finished = world:entity(), -- tag: finalized, file written

	-- Request entities: spawned with a payload component, consumed by their
	-- system, then tagged Destroy for the end-of-frame sweep.
	FocusRequest = world:component(), -- payload: conflict entity the camera should frame
	Destroy = world:entity(), -- tag: delete this entity in the cleanup sweep
}

for name, ct in cts do
	world:set(ct, jecs.Name, name)
end

return cts
