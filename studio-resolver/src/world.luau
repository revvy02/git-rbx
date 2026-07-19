--[[
	The jecs model layer. Instances in the preview DOM mirror to entities
	(Ref component + ChildOf pairs), synced by frame diff: walk the DOM,
	ensure entities and parent pairs, delete entities whose instance left the
	tree. Conflicts are entities too, carrying their record as ConflictData
	and a TargetRef to the contested instance's entity.

	This is the substrate the diff layers build on: per-instance status
	becomes components, tree rendering becomes ChildOf traversal, and the
	four side-worlds become relationships — the UI stays immediate-mode and
	just reads queries.
]]

local packages = require("@self/packages")
local state = require("@self/state")

local jecs = packages.jecs

type Conflict = state.Conflict

local world = jecs.world(true)

local Ref = world:component()
world:set(Ref, jecs.Name, "Ref")
local ConflictData = world:component()
world:set(ConflictData, jecs.Name, "ConflictData")

local ChildOf = jecs.ChildOf
local pair = jecs.pair

local entities: { [Instance]: any } = {}

local function entityFor(inst: Instance): any
	local entity = entities[inst]
	if not entity then
		entity = world:entity()
		world:set(entity, Ref, inst)
		entities[inst] = entity
	end
	return entity
end

local M = {}

M.world = world
M.Ref = Ref
M.ConflictData = ConflictData
M.ChildOf = ChildOf
M.pair = pair
M.entityFor = entityFor

function M.entityOf(inst: Instance): any?
	return entities[inst]
end

-- Boot: one entity per conflict, holding the shared record (mutations via
-- state.stage stay visible through both views).
function M.boot()
	for _, conflict in state.conflicts do
		local entity = world:entity()
		world:set(entity, ConflictData, conflict)
		world:add(entity, pair(jecs.ChildOf, entityFor(conflict.target)))
	end
end

-- Frame diff: mirror the live tree under the given roots. Instances gone
-- from the tree lose their entities.
function M.syncTree(roots: { Instance })
	local seen: { [Instance]: boolean } = {}

	local function visit(inst: Instance, parentEntity: any?)
		seen[inst] = true
		local entity = entityFor(inst)
		if parentEntity then
			local currentParent = world:parent(entity)
			if currentParent ~= parentEntity then
				world:add(entity, pair(ChildOf, parentEntity))
			end
		end
		for _, child in inst:GetChildren() do
			if child.ClassName ~= "Highlight" and child.Name ~= "__RbxDiffMerge" then
				visit(child, entity)
			end
		end
	end
	for _, root in roots do
		visit(root, nil)
	end

	for inst, entity in entities do
		if not seen[inst] and inst:IsA("Instance") then
			-- Conflict side clones live outside the tree on purpose; only
			-- drop entities whose instance is truly gone.
			if not inst:IsDescendantOf(game) then
				world:delete(entity)
				entities[inst] = nil
			end
		end
	end
end

-- Ordered children of an entity in the mirrored tree, by instance name.
function M.childrenOf(entity: any): { any }
	local out: { any } = {}
	for child in world:query(Ref):with(pair(ChildOf, entity)):iter() do
		table.insert(out, child)
	end
	table.sort(out, function(a: any, b: any)
		local ia = world:get(a, Ref) :: Instance
		local ib = world:get(b, Ref) :: Instance
		return ia.Name < ib.Name
	end)
	return out
end

return M
