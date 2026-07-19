--[[
	Outline-only highlight on every conflict target (or the swapped-in clone
	a staged delete-vs-edit edit side renders), tinted by staged resolution.
]]

local packages = require("../../packages")

local theme = require("../../theme")

local conflicts = require("../library/conflicts")
local types = require("../types")

local jecs = packages.jecs

type SideInfo = types.SideInfo

return function(world: types.World, cts: types.Cts, scheduler: types.Scheduler)
	local liveHighlights: { [any]: Highlight } = {}

	scheduler:addSystem({
		phase = packages.planckRunService.Phases.Heartbeat,
		system = function()
			local finished = world:has(cts.Finished, cts.Finished)
			for _, entity in conflicts.all() do
				local target = world:get(entity, cts.Target) :: Instance
				local spatial = target:IsDescendantOf(workspace) and target:IsA("PVInstance")
				if not spatial or finished then
					local existing = liveHighlights[entity]
					if existing then
						existing:Destroy()
						liveHighlights[entity] = nil
					end
					continue
				end

				local highlight = liveHighlights[entity]
				if not highlight then
					highlight = Instance.new("Highlight")
					highlight.Name = `RbxDiffHighlight_{world:get(entity, jecs.Name)}`
					highlight.Parent = target
					liveHighlights[entity] = highlight
				end

				local resolution = world:get(entity, cts.Resolution) :: string?

				-- Adorn whatever the preview is actually rendering: normally the
				-- target, but a staged edit side of a delete-vs-edit renders its
				-- swapped-in clone instead.
				local adornee: Instance = target
				if world:get(entity, cts.Kind) == "DeleteVsEdit" then
					local ours = world:get(entity, cts.Ours) :: SideInfo
					local theirs = world:get(entity, cts.Theirs) :: SideInfo
					local editSide = if ours.clone then "ours" else "theirs"
					local info = if editSide == "ours" then ours else theirs
					if resolution == editSide and info.clone then
						adornee = info.clone :: Instance
					end
				end
				highlight.Adornee = adornee

				local color = theme.statusColor(resolution)
				highlight.OutlineColor = color
				highlight.FillColor = color
				highlight.FillTransparency = 1 -- outline only
			end
		end,
	})
end
