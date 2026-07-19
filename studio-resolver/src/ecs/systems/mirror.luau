--[[
	Instance mirror: preview instances become entities (Ref + exclusive
	ChildOf pairs), synced by frame diff — walk the DOM, ensure entities and
	parent pairs, delete entities whose instance left the game. Session
	artifacts (Highlights, the stamped container) stay out of the mirror.
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
		entities[inst] = entity
	end
	return entity
end

function mirror.entityOf(inst: Instance): any?
	return entities[inst]
end

function mirror.syncTree(roots: { Instance })
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

-- Ordered children of an entity in the mirrored tree, by instance name.
function mirror.childrenOf(entity: any): { any }
	local out: { any } = {}
	for child in world:query(cts.Ref):with(pair(ChildOf, entity)):iter() do
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
