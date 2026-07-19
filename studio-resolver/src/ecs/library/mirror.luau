--[[
	Instance mirror: preview instances become entities (Ref + exclusive
	ChildOf pairs), synced by frame diff — walk the DOM, ensure entities and
	parent pairs, delete entities whose instance left the game. Session
	artifacts (Highlights, the stamped container) stay out of the mirror.
	systems/mirror.luau schedules the sync; this is the shared index API.
]]

local packages = require("../../packages")

local cts = require("../cts")
local world = require("../world")

local jecs = packages.jecs
local pair = jecs.pair
local ChildOf = jecs.ChildOf

local entities: { [Instance]: any } = {}

local mirror = {}

function mirror.entityFor(inst: Instance): any
	local entity = entities[inst]
	if not entity then
		entity = world:entity()
		world:set(entity, cts.Ref, inst)
		world:set(entity, jecs.Name, inst.Name)
		entities[inst] = entity
	end
	return entity
end

-- Model previews root at the imported content; places root at Workspace plus
-- any service holding a conflict target.
local currentRoots: { Instance } = {}

local function treeRoots(): { Instance }
	local previewFolder = workspace:FindFirstChild("RbxDiffModelPreview")
	if previewFolder then
		return previewFolder:GetChildren()
	end
	local roots: { Instance } = { workspace }
	local seen: { [Instance]: boolean } = { [workspace :: Instance] = true }
	for _, target in world:query(cts.Target) do
		local top: Instance = target :: Instance
		while top.Parent and top.Parent ~= game do
			top = top.Parent :: Instance
		end
		if not seen[top] then
			seen[top] = true
			table.insert(roots, top)
		end
	end
	return roots
end

function mirror.roots(): { Instance }
	return currentRoots
end

local function syncTree(roots: { Instance })
	local seen: { [Instance]: boolean } = {}

	local function visit(inst: Instance, parentEntity: any?)
		seen[inst] = true
		local entity = mirror.entityFor(inst)
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
		if not seen[inst] and not inst:IsDescendantOf(game) then
			world:delete(entity)
			entities[inst] = nil
		end
	end
end

-- The per-frame pass systems/mirror.luau schedules: refresh roots and
-- frame-diff the tree.
function mirror.sync()
	currentRoots = treeRoots()
	syncTree(currentRoots)
end

-- Ordered children of an entity in the mirrored tree, by instance name.
function mirror.childrenOf(entity: any): { any }
	local out: { any } = {}
	for child in world:query(cts.Ref):with(pair(ChildOf, entity)) do
		table.insert(out, child)
	end
	table.sort(out, function(a: any, b: any)
		local ia = world:get(a, cts.Ref) :: Instance
		local ib = world:get(b, cts.Ref) :: Instance
		return ia.Name < ib.Name
	end)
	return out
end

return mirror
