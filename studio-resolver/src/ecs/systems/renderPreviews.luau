--[[
	Render the staged selection into the scene — staging Ours renders ours'
	version, Theirs renders theirs', unpicked renders base. Property values
	apply to the target, moves reparent it, delete-vs-edit swaps the stamped
	clone itself in and out of the scene. Unpicking restores boot snapshots;
	none of this touches the disk file — the CLI at Complete is still the
	only writer. Staged Resolution vs Rendered drives the frame diff.
]]

local packages = require("../../packages")

local conflicts = require("../library/conflicts")
local types = require("../types")

type SideInfo = types.SideInfo
type Base = types.Base

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
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
				if sideClone then
					(target :: any)[property] = (sideClone :: any)[property]
				elseif staged == nil then
					(target :: any)[property] = base.value
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
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			if world:has(cts.Finished, cts.Finished) then
				return
			end
			for _, entity in conflicts.all() do
				local staged = (world:get(entity, cts.Resolution) :: string?) or "unresolved"
				if world:get(entity, cts.Rendered) ~= staged then
					world:set(entity, cts.Rendered, staged)
					applyPreview(entity)
				end
			end
		end,
	})
end
