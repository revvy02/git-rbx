--[[
	Outline-only highlight on every conflict target (or the swapped-in clone
	a staged delete-vs-edit edit side renders), tinted by staged resolution.
	Resolution vs Highlighted drives the frame diff — Highlight instances are
	only touched when a conflict's staged state actually changes, mirroring
	the Rendered pattern in renderPreviews.
]]

local packages = require("../../packages")

local theme = require("../../library/theme")

local conflicts = require("./library/conflicts")
local types = require("../types")

local jecs = packages.jecs

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
	local conflictList = (world:get(cts.Conflicts, cts.Conflicts) :: { any }?) or {}

	-- System-private Highlight instances, keyed by conflict entity.
	local liveHighlights: { [any]: Highlight } = {}

	local function applyHighlight(entity: any, resolution: string?)
		local target = world:get(entity, cts.Target) :: Instance

		local highlight = liveHighlights[entity]
		if not highlight then
			highlight = Instance.new("Highlight")
			highlight.Name = `RbxDiffHighlight_{world:get(entity, jecs.Name)}`
			highlight.Parent = target
			liveHighlights[entity] = highlight
		end

		-- Adorn whatever the preview is actually rendering: normally the
		-- target, but a staged keep side of a delete-vs-edit renders its
		-- swapped-in clone instead.
		highlight.Adornee = conflicts.renderedInstance(entity)

		local color = theme.statusColor(resolution)
		highlight.OutlineColor = color
		highlight.FillColor = color
		highlight.FillTransparency = 1 -- outline only
	end

	local function removeHighlight(entity: any)
		local highlight = liveHighlights[entity]
		if highlight then
			highlight:Destroy()
			liveHighlights[entity] = nil
		end
	end

	scheduler:addSystem({
		name = "update_highlights",
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			debug.profilebegin("rbxdiff_highlights")
			local finished = world:has(cts.Finished, cts.Finished)
			for _, entity in conflictList do
				local target = world:get(entity, cts.Target) :: Instance
				local spatial = target:IsDescendantOf(workspace) and target:IsA("PVInstance")

				local wanted: string? = nil
				if spatial and not finished then
					wanted = (world:get(entity, cts.Resolution) :: string?) or "unresolved"
				end

				if world:get(entity, cts.Highlighted) ~= wanted then
					world:set(entity, cts.Highlighted, wanted)
					if wanted == nil then
						removeHighlight(entity)
					else
						applyHighlight(entity, if wanted == "unresolved" then nil else wanted)
					end
				end
			end
			debug.profileend()
		end,
	})
end
