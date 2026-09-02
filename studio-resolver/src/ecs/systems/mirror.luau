--[[
	Instance mirror: preview instances become entities, synced by frame diff —
	walk the DOM, ensure entities and exclusive ChildOf pairs, delete entities
	whose instance left the game. Session artifacts (Highlights, the stamped
	container) stay out of the mirror.

	The tree itself is world data, not an API: each node carries a Children
	component (child entities, name-ordered), the root entity list is the
	Roots singleton, and the Instance -> entity index is the EntityOf
	singleton. Conflict entities get parented (ChildOf) under their target's
	mirror entity here too — readers walk components, nothing calls into this
	module.
]]

local packages = require("../../packages")

local types = require("../types")

local jecs = packages.jecs
local pair = jecs.pair
local ChildOf = jecs.ChildOf

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
	-- System-private Instance -> entity map, published as the EntityOf
	-- singleton (same table, mutated in place).
	local entities: { [Instance]: any } = {}
	world:set(cts.EntityOf, cts.EntityOf, entities)

	-- Anything a sync changes bumps MirrorGeneration, so readers (explorer
	-- row cache) can key derived data on it instead of re-walking per frame.
	local generation = 0
	local dirty = false

	local function entityFor(inst: Instance): any
		local entity = entities[inst]
		if not entity then
			entity = world:entity()
			world:set(entity, cts.Ref, inst)
			world:set(entity, jecs.Name, inst.Name)
			entities[inst] = entity
			dirty = true
		end
		return entity
	end

	local targets = world:query(cts.Target):cached()

	-- Model previews root at the imported content; places root at Workspace
	-- plus any service holding a conflict target.
	local function treeRoots(): { Instance }
		local previewFolder = workspace:FindFirstChild("GitRbxModelPreview")
		if previewFolder then
			return previewFolder:GetChildren()
		end
		local roots: { Instance } = { workspace }
		local seen: { [Instance]: boolean } = { [workspace :: Instance] = true }
		for _, target in targets do
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

	local function sameChildren(old: { any }?, new: { any }): boolean
		if old == nil or #old ~= #new then
			return false
		end
		for index, entity in new do
			if (old :: { any })[index] ~= entity then
				return false
			end
		end
		return true
	end

	local function syncTree(roots: { Instance })
		local seen: { [Instance]: boolean } = {}

		local function visit(inst: Instance, parentEntity: any?): any
			seen[inst] = true
			local entity = entityFor(inst)
			if parentEntity then
				if world:parent(entity) ~= parentEntity then
					world:add(entity, pair(ChildOf, parentEntity))
					dirty = true
				end
			end
			-- Renames must invalidate derived labels.
			if world:get(entity, jecs.Name) ~= inst.Name then
				world:set(entity, jecs.Name, inst.Name)
				dirty = true
			end

			local kids: { Instance } = {}
			for _, child in inst:GetChildren() do
				if child.ClassName ~= "Highlight" and child.Name ~= "__GitRbxConflicts" then
					table.insert(kids, child)
				end
			end
			table.sort(kids, function(a: Instance, b: Instance)
				return a.Name < b.Name
			end)

			local childEntities = table.create(#kids)
			for index, child in kids do
				childEntities[index] = visit(child, entity)
			end
			if not sameChildren(world:get(entity, cts.Children), childEntities) then
				world:set(entity, cts.Children, childEntities)
				dirty = true
			end
			return entity
		end

		local rootEntities = table.create(#roots)
		for index, root in roots do
			rootEntities[index] = visit(root, nil)
		end
		if not sameChildren(world:get(cts.Roots, cts.Roots), rootEntities) then
			world:set(cts.Roots, cts.Roots, rootEntities)
			dirty = true
		end

		for inst, entity in entities do
			if not seen[inst] and not inst:IsDescendantOf(game) then
				world:delete(entity)
				entities[inst] = nil
				dirty = true
			end
		end
	end

	-- Conflict entities live under their target's mirror entity; targets
	-- never change, so this settles after the first pass and self-heals.
	local function parentConflicts()
		for _, entity in (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {} do
			local targetEntity = entityFor(world:get(entity, cts.Target) :: Instance)
			if world:parent(entity) ~= targetEntity then
				world:add(entity, pair(ChildOf, targetEntity))
			end
		end
	end

	scheduler:addSystem({
		name = "mirror",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("rbxdiff_mirror")
			syncTree(treeRoots())
			parentConflicts()
			if dirty then
				dirty = false
				generation += 1
				world:set(cts.MirrorGeneration, cts.MirrorGeneration, generation)
			end
			debug.profileend()
		end,
	})
end
