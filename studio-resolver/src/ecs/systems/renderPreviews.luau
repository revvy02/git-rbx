--[[
	Staged-selection preview. Boot stashes the stamped container out of the
	scene and fills each conflict entity's side info (Ours/Theirs folders) and
	Base snapshot (what unpicking restores); the scheduled loop renders the
	staged selection into the scene — staging Ours renders ours' version,
	Theirs renders theirs', unpicked renders base. Property values apply to
	the target, moves reparent it, delete-vs-edit swaps the stamped clone in
	and out. None of this touches the disk file — the CLI at Complete is the
	only writer. Staged Resolution vs Rendered drives the frame diff.
]]

local packages = require("../../packages")

local format = require("../../library/format")
local types = require("../types")

type SideInfo = types.SideInfo
type Base = types.Base

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
	local conflictList = (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}

	local function eachPart(root: Instance, fn: (BasePart) -> ())
		if root:IsA("BasePart") then
			fn(root)
		end
		for _, descendant in root:GetDescendants() do
			if descendant:IsA("BasePart") then
				fn(descendant)
			end
		end
	end

	local function sideInfo(entry: Instance, sideName: string): SideInfo
		local info: SideInfo = {}
		local folder = entry:FindFirstChild(sideName)
		if folder then
			info.folder = folder
			for _, child in folder:GetChildren() do
				if child.Name == "Destination" and child:IsA("ObjectValue") then
					info.destination = child.Value
				else
					info.clone = child
				end
			end
			info.destinationPath = folder:GetAttribute("DestinationPath") :: string?
		end
		return info
	end

	-- Boot pass: the stamped container rides inside model files, so a model
	-- preview imports it into workspace — where every clone renders on top
	-- of its target. It is pure data for this session: stash it out of the
	-- scene (copy-only; the file keeps its container until finalize strips
	-- it), then per conflict entity set side clones + the base snapshot.
	local container = world:get(cts.Container, cts.Container) :: Instance
	if container:IsDescendantOf(workspace) then
		container.Parent = game:GetService("ServerStorage")
	end

	for _, entity in conflictList do
		local entry = world:get(entity, cts.Entry) :: Instance
		world:set(entity, cts.Ours, sideInfo(entry, "Ours"))
		world:set(entity, cts.Theirs, sideInfo(entry, "Theirs"))
		world:set(entity, cts.Rendered, "__init")

		local target = world:get(entity, cts.Target) :: Instance
		local kind = world:get(entity, cts.Kind)
		local base: Base = {}
		if kind == "Property" then
			local property = world:get(entity, cts.Property) :: string
			if property == "Name" then
				base.value = target.Name
			else
				local ok, value = format.readProperty(target, property)
				base.value = if ok then value else nil
			end
		elseif kind == "MoveTarget" then
			base.parent = target.Parent
		elseif kind == "DeleteVsEdit" then
			local parts: { [BasePart]: number } = {}
			eachPart(target, function(part)
				parts[part] = part.Transparency
			end)
			base.parts = parts
		end
		world:set(entity, cts.Base, base)
	end

	-- Render the staged selection; nil (unpicked) restores the base snapshot.
	local function applyPreview(entity: any)
		local staged = world:get(entity, cts.Resolution) :: string?
		local kind = world:get(entity, cts.Kind)
		local target = world:get(entity, cts.Target) :: Instance
		local ours = world:get(entity, cts.Ours) :: SideInfo
		local theirs = world:get(entity, cts.Theirs) :: SideInfo
		local base = world:get(entity, cts.Base) :: Base

		if kind == "Property" then
			if staged ~= "custom" then
				local sideClone = if staged == "ours"
					then ours.clone
					elseif staged == "theirs" then theirs.clone
					else nil
				local property = world:get(entity, cts.Property) :: string
				-- Serialized-only properties can't preview (Luau can't read or
				-- write them) — staging still works, the CLI applies them at
				-- Complete host-side.
				if sideClone then
					pcall(function()
						(target :: any)[property] = (sideClone :: any)[property]
					end)
				elseif staged == nil and base.value ~= nil then
					pcall(function()
						(target :: any)[property] = base.value
					end)
				end
			end
		elseif kind == "MoveTarget" then
			local destination = if staged == "ours"
				then ours.destination
				elseif staged == "theirs" then theirs.destination
				else nil
			-- A side with no live destination (moved into the other branch's added
			-- content) previews as base rather than sticking at a stale parent.
			destination = destination or base.parent
			if destination then
				target.Parent = destination
			end
		elseif kind == "DeleteVsEdit" then
			-- One side deletes, the other edits. Staging either side hides the
			-- base target; staging the edit side additionally swaps its stamped
			-- clone into the target's spot (it carries its own true coordinates).
			local editSide = if ours.clone then "ours" else "theirs"
			local editInfo = if editSide == "ours" then ours else theirs
			for part, original in base.parts :: { [BasePart]: number } do
				part.Transparency = if staged == nil then original else 1
			end
			local clone = editInfo.clone
			if clone then
				clone.Parent = if staged == editSide then target.Parent else editInfo.folder
			end
		end
	end

	scheduler:addSystem({
		name = "render_previews",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			if world:has(cts.Finished, cts.Finished) then
				return
			end
			debug.profilebegin("rbxdiff_previews")
			for _, entity in conflictList do
				local staged = (world:get(entity, cts.Resolution) :: string?) or "unresolved"
				if world:get(entity, cts.Rendered) ~= staged then
					world:set(entity, cts.Rendered, staged)
					applyPreview(entity)
				end
			end
			debug.profileend()
		end,
	})
end
