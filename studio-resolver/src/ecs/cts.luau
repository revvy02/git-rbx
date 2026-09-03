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
	clone: Instance?, -- this side's edited version (absent for deletes and reparents)
	moveOuts: { { folder: Instance, snapshot: Instance, destination: Instance } }?, -- edited descendants escaping a commonly deleted root
	deleted: boolean?, -- this side removes the subtree (stamped Deleted attribute)
	destination: Instance?, -- the parent this side reparents the target to
	destinationPath: string?,
	delta: CFrame?, -- Pivot: canonical/base content -> this side's world placement
	pivots: { PivotApplication }?, -- conditional pivots selected with this side
}

-- Canonical direct patch emitted by Rust for one side of a decision.
-- `operations` are the exact edit script shown by the sparse diff tree;
-- `affectedIds` supports exact de-duplicated counts when rigid cards aggregate.
export type ImpactOperation = {
	kind: string, -- Property | Add | Delete | Reparent
	nodeId: number?,
	path: string,
	property: string?,
	destination: string?,
	instanceCount: number,
	before: any?, -- serialized diff::PropertyValue
	after: any?,
}

export type ImpactSide = {
	operations: { ImpactOperation },
	affectedIds: { number },
	instanceCount: number,
	propertyCount: number,
}

-- Boot snapshot of the base state, restored on unpick (preview.boot).
export type Base = {
	value: any, -- Property: value to restore
	parent: Instance?, -- ReparentTarget: parent to restore
	parts: { [BasePart]: number }?, -- clone swaps: true target transparencies
}

-- One occurrence in a projected Explorer view. `subject` and `entity` retain
-- canonical merged-DOM identity across views; `source` is the concrete version
-- this occurrence presents. Virtual rows can omit all three.
export type ExplorerNode = {
	id: number?, -- logical identity shared by matched nodes across version trees
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
	idIndex: { [number]: ExplorerNode }?,
	selectionIndex: { [number]: ExplorerNode }?, -- exact node or nearest surviving ancestor
}

export type VersionTreeNode = {
	id: number,
	parent: number?,
	name: string,
	className: string,
}

export type VersionTrees = { [string]: { VersionTreeNode } }

export type PivotApplication = {
	target: Instance,
	order: number,
	parentOrder: number?,
	delta: CFrame,
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
	VersionTrees = world:component(), -- complete immutable base/ours/theirs manifests
	VirtualSubjects = world:component(), -- logical id -> live result Instance when present
	VirtualSubjectIds = world:component(), -- live result Instance -> logical id

	-- Conflict entities, one per stamped container entry.
	Kind = world:component(), -- "Property" | "PropertyBundle" | "ReparentTarget" | "DeleteVsEdit" | "Pivot"
	Path = world:component(), -- display path of the contested instance
	Property = world:component(), -- Property kind: the contested property name
	Properties = world:component(), -- PropertyBundle: atomic serialized property names
	Target = world:component(), -- the contested live Instance
	Entry = world:component(), -- the stamped container entry (Name = CLI --entry)
	Order = world:component(), -- stable list position (entry-name sort)
	PivotOrder = world:component(), -- hierarchical pivot order (ancestors first)
	PivotParentOrder = world:component(), -- nearest ancestor pivot decision
	Ours = world:component(), -- SideInfo
	Theirs = world:component(), -- SideInfo
	ImpactOurs = world:component(), -- ImpactSide
	ImpactTheirs = world:component(), -- ImpactSide
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
	Container = world:component(), -- the stamped __GitRbxConflicts container
	PivotPlan = world:component(), -- automatic local pivots deferred by a pivot conflict
	ViewTab = world:component(), -- "base" | "ours" | "theirs" | "result"
	SelectedConflict = world:component(), -- conflict entity
	SelectedInstance = world:component(), -- Instance the explorer/props focus on
	SelectedExplorerId = world:component(), -- logical selection shared by all version projections
	ExplorerRevealGeneration = world:component(), -- bumps when Select should expand + center again
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
